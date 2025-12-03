// QUIC-based video streaming implementation
//
// Provides RTP over QUIC for efficient low-latency video streaming
// with stream multiplexing and optimized buffer management.
//
// Requirements: 1.3, 2.2

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{mpsc, Mutex, RwLock};
use quinn::{Connection as QuinnConnection, RecvStream, SendStream};

use crate::streaming::{
    EncodedFrame, PeerId, StreamConnection, StreamError, StreamResult, StreamStats, VideoStream,
};
use crate::transport::protocols::quic::{QuicTransport, QuicConfig};
use crate::transport::{PeerAddress, Transport, TransportCapabilities};

/// QUIC-based video streamer for low-latency streaming
///
/// Uses RTP over QUIC with stream multiplexing to efficiently
/// stream video with minimal latency and optimal bandwidth usage.
///
/// Requirements: 1.3, 2.2
pub struct QuicVideoStreamer {
    config: QuicStreamerConfig,
    transport: Arc<QuicTransport>,
    active_streams: Arc<RwLock<HashMap<PeerId, ActiveQuicStream>>>,
    stream_multiplexer: Arc<Mutex<StreamMultiplexer>>,
}

/// Configuration for QUIC video streaming
#[derive(Debug, Clone)]
pub struct QuicStreamerConfig {
    /// Maximum concurrent video streams per connection
    pub max_concurrent_streams: u32,
    /// Stream priority for video data
    pub video_stream_priority: u8,
    /// Enable stream multiplexing for multiple quality levels
    pub enable_multiplexing: bool,
    /// Buffer size for video frames
    pub frame_buffer_size: usize,
    /// Connection idle timeout
    pub idle_timeout: Duration,
    /// Enable 0-RTT for faster connection establishment
    pub enable_0rtt: bool,
}

/// Active QUIC streaming session
struct ActiveQuicStream {
    connection: QuinnConnection,
    video_streams: HashMap<u64, VideoStreamChannel>,
    stats: Arc<Mutex<StreamStats>>,
    created_at: SystemTime,
}

/// Video stream channel over QUIC
struct VideoStreamChannel {
    send_stream: SendStream,
    recv_stream: Option<RecvStream>,
    stream_id: u64,
    quality_level: QualityLevel,
}

/// Quality level for multiplexed streams
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QualityLevel {
    Low,
    Medium,
    High,
    Ultra,
}

/// Stream multiplexer for managing multiple video streams
struct StreamMultiplexer {
    next_stream_id: u64,
    stream_assignments: HashMap<PeerId, Vec<u64>>,
}

/// RTP packet header for video streaming
#[derive(Debug, Clone)]
struct RtpHeader {
    version: u8,
    padding: bool,
    extension: bool,
    csrc_count: u8,
    marker: bool,
    payload_type: u8,
    sequence_number: u16,
    timestamp: u32,
    ssrc: u32,
}

/// RTP packet for video frame transmission
#[derive(Debug, Clone)]
struct RtpPacket {
    header: RtpHeader,
    payload: Vec<u8>,
}

impl Default for QuicStreamerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_streams: 10,
            video_stream_priority: 200, // High priority
            enable_multiplexing: true,
            frame_buffer_size: 1024 * 1024, // 1MB
            idle_timeout: Duration::from_secs(30),
            enable_0rtt: true,
        }
    }
}

impl QuicVideoStreamer {
    /// Create a new QUIC video streamer
    pub fn new() -> StreamResult<Self> {
        Self::with_config(QuicStreamerConfig::default())
    }

    /// Create a new QUIC video streamer with custom configuration
    pub fn with_config(config: QuicStreamerConfig) -> StreamResult<Self> {
        // Create QUIC transport with streaming-optimized configuration
        let mut quic_config = QuicConfig::default();
        quic_config.max_concurrent_streams = config.max_concurrent_streams;
        quic_config.idle_timeout = config.idle_timeout;
        quic_config.enable_0rtt = config.enable_0rtt;

        let transport = QuicTransport::with_config(quic_config)
            .map_err(|e| StreamError::network(format!("Failed to create QUIC transport: {}", e)))?;

        Ok(Self {
            config,
            transport: Arc::new(transport),
            active_streams: Arc::new(RwLock::new(HashMap::new())),
            stream_multiplexer: Arc::new(Mutex::new(StreamMultiplexer::new())),
        })
    }

    /// Start streaming video to a peer
    pub async fn start_streaming(
        &self,
        peer_id: PeerId,
        stream: VideoStream,
        peer_address: PeerAddress,
    ) -> StreamResult<StreamConnection> {
        // Connect to peer using QUIC transport
        let connection = self
            .transport
            .connect(&peer_address)
            .await
            .map_err(|e| StreamError::network(format!("Failed to connect: {}", e)))?;

        // Extract QUIC connection from the transport connection
        let quic_connection = self.get_quic_connection(&connection).await?;

        // Open video stream(s) based on multiplexing configuration
        let video_streams = if self.config.enable_multiplexing {
            self.open_multiplexed_streams(&quic_connection).await?
        } else {
            self.open_single_stream(&quic_connection, QualityLevel::High).await?
        };

        // Store active stream
        let stats = Arc::new(Mutex::new(StreamStats::default()));
        let active_stream = ActiveQuicStream {
            connection: quic_connection,
            video_streams,
            stats: stats.clone(),
            created_at: SystemTime::now(),
        };

        {
            let mut streams = self.active_streams.write().await;
            streams.insert(peer_id.clone(), active_stream);
        }

        // Register stream with multiplexer
        {
            let mut multiplexer = self.stream_multiplexer.lock().await;
            multiplexer.register_stream(&peer_id);
        }

        Ok(StreamConnection {
            id: uuid::Uuid::new_v4(),
            peer_id,
            stream_id: stream.id,
        })
    }

    /// Send an encoded video frame to a peer
    pub async fn send_frame(
        &self,
        peer_id: &PeerId,
        frame: EncodedFrame,
        quality_level: QualityLevel,
    ) -> StreamResult<()> {
        let frame_size = frame.data.len();
        
        // Convert frame to RTP packets
        let rtp_packets = self.frame_to_rtp_packets(frame)?;

        // Get mutable access to streams
        let mut streams = self.active_streams.write().await;
        let stream = streams
            .get_mut(peer_id)
            .ok_or_else(|| StreamError::network("Stream not found"))?;

        // Find the appropriate video stream for the quality level
        let video_stream = stream
            .video_streams
            .get_mut(&(quality_level as u64))
            .ok_or_else(|| StreamError::network("Quality level stream not found"))?;

        // Send RTP packets over QUIC stream
        for packet in rtp_packets {
            self.send_rtp_packet(&mut video_stream.send_stream, packet).await?;
        }

        // Update statistics
        let mut stats = stream.stats.lock().await;
        stats.frames_encoded += 1;
        stats.bytes_sent += frame_size as u64;
        stats.last_updated = SystemTime::now();

        Ok(())
    }

    /// Receive a video stream from a peer
    pub async fn receive_stream(
        &self,
        peer_id: PeerId,
        peer_address: PeerAddress,
    ) -> StreamResult<VideoStream> {
        // Connect to peer
        let connection = self
            .transport
            .connect(&peer_address)
            .await
            .map_err(|e| StreamError::network(format!("Failed to connect: {}", e)))?;

        let quic_connection = self.get_quic_connection(&connection).await?;

        // Accept incoming video streams
        let (frame_sender, frame_receiver) = mpsc::unbounded_channel();
        
        // Spawn task to receive video frames
        let connection_clone = quic_connection.clone();
        tokio::spawn(async move {
            loop {
                match connection_clone.accept_bi().await {
                    Ok((send, mut recv)) => {
                        let frame_sender = frame_sender.clone();
                        tokio::spawn(async move {
                            let mut buffer = vec![0u8; 65536];
                            while let Ok(Some(n)) = recv.read(&mut buffer).await {
                                // Parse RTP packet and extract frame data
                                if let Ok(frame_data) = Self::parse_rtp_packet(&buffer[..n]) {
                                    let _ = frame_sender.send(frame_data);
                                }
                            }
                        });
                    }
                    Err(_) => break,
                }
            }
        });

        // Create video stream
        let video_stream = VideoStream {
            id: uuid::Uuid::new_v4(),
            source: crate::streaming::StreamSource::File(std::path::PathBuf::from("quic-stream")),
            quality: crate::streaming::StreamQuality::default(),
        };

        Ok(video_stream)
    }

    /// Close a streaming connection
    pub async fn close_stream(&self, peer_id: &PeerId) -> StreamResult<()> {
        let mut streams = self.active_streams.write().await;
        
        if let Some(stream) = streams.remove(peer_id) {
            // Close all video streams
            for (_, mut video_stream) in stream.video_streams {
                let _ = video_stream.send_stream.finish();
            }

            // Close QUIC connection
            stream.connection.close(0u32.into(), b"Stream closed");
        }

        // Unregister from multiplexer
        {
            let mut multiplexer = self.stream_multiplexer.lock().await;
            multiplexer.unregister_stream(peer_id);
        }

        Ok(())
    }

    /// Get stream statistics
    pub async fn get_stats(&self, peer_id: &PeerId) -> StreamResult<StreamStats> {
        let streams = self.active_streams.read().await;
        let stream = streams
            .get(peer_id)
            .ok_or_else(|| StreamError::network("Stream not found"))?;

        let mut stats = stream.stats.lock().await.clone();
        
        // Update with QUIC connection statistics
        let quic_stats = stream.connection.stats();
        stats.latency_ms = quic_stats.path.rtt.as_millis() as u32;
        stats.current_bitrate = self.estimate_bitrate(&stream.connection).await;

        Ok(stats)
    }

    /// Adjust stream quality based on network conditions
    pub async fn adjust_quality(
        &self,
        peer_id: &PeerId,
        target_quality: QualityLevel,
    ) -> StreamResult<()> {
        let streams = self.active_streams.read().await;
        let _stream = streams
            .get(peer_id)
            .ok_or_else(|| StreamError::network("Stream not found"))?;

        // In a real implementation, this would switch to a different quality stream
        // or adjust encoding parameters
        println!("Adjusting stream quality to {:?}", target_quality);

        Ok(())
    }

    // Private helper methods

    async fn get_quic_connection(
        &self,
        _connection: &Box<dyn crate::transport::Connection>,
    ) -> StreamResult<QuinnConnection> {
        // In a real implementation, this would extract the QUIC connection
        // from the transport connection wrapper
        Err(StreamError::unsupported("QUIC connection extraction not implemented"))
    }

    async fn open_single_stream(
        &self,
        connection: &QuinnConnection,
        quality_level: QualityLevel,
    ) -> StreamResult<HashMap<u64, VideoStreamChannel>> {
        let (send_stream, recv_stream) = connection
            .open_bi()
            .await
            .map_err(|e| StreamError::network(format!("Failed to open stream: {}", e)))?;

        let stream_id = 0;
        let video_stream = VideoStreamChannel {
            send_stream,
            recv_stream: Some(recv_stream),
            stream_id,
            quality_level,
        };

        let mut streams = HashMap::new();
        streams.insert(stream_id, video_stream);

        Ok(streams)
    }

    async fn open_multiplexed_streams(
        &self,
        connection: &QuinnConnection,
    ) -> StreamResult<HashMap<u64, VideoStreamChannel>> {
        let mut streams = HashMap::new();
        let quality_levels = vec![
            QualityLevel::Low,
            QualityLevel::Medium,
            QualityLevel::High,
        ];

        for (idx, quality_level) in quality_levels.into_iter().enumerate() {
            let (send_stream, recv_stream) = connection
                .open_bi()
                .await
                .map_err(|e| StreamError::network(format!("Failed to open stream: {}", e)))?;

            let video_stream = VideoStreamChannel {
                send_stream,
                recv_stream: Some(recv_stream),
                stream_id: idx as u64,
                quality_level,
            };

            streams.insert(idx as u64, video_stream);
        }

        Ok(streams)
    }

    fn frame_to_rtp_packets(&self, frame: EncodedFrame) -> StreamResult<Vec<RtpPacket>> {
        let mut packets = Vec::new();
        let max_payload_size = 1200; // MTU-safe size
        let chunks: Vec<&[u8]> = frame.data.chunks(max_payload_size).collect();
        let total_chunks = chunks.len();

        for (idx, chunk) in chunks.into_iter().enumerate() {
            let header = RtpHeader {
                version: 2,
                padding: false,
                extension: false,
                csrc_count: 0,
                marker: idx == total_chunks - 1, // Mark last packet
                payload_type: 96, // Dynamic payload type for H.264
                sequence_number: idx as u16,
                timestamp: frame.timestamp.duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u32,
                ssrc: 0x12345678, // Synchronization source identifier
            };

            packets.push(RtpPacket {
                header,
                payload: chunk.to_vec(),
            });
        }

        Ok(packets)
    }

    async fn send_rtp_packet(
        &self,
        send_stream: &mut SendStream,
        packet: RtpPacket,
    ) -> StreamResult<()> {
        // Serialize RTP packet
        let mut buffer = Vec::new();
        buffer.push((packet.header.version << 6) | (packet.header.csrc_count & 0x0F));
        buffer.push((if packet.header.marker { 0x80 } else { 0 }) | packet.header.payload_type);
        buffer.extend_from_slice(&packet.header.sequence_number.to_be_bytes());
        buffer.extend_from_slice(&packet.header.timestamp.to_be_bytes());
        buffer.extend_from_slice(&packet.header.ssrc.to_be_bytes());
        buffer.extend_from_slice(&packet.payload);

        // Send over QUIC stream
        send_stream
            .write_all(&buffer)
            .await
            .map_err(|e| StreamError::network(format!("Failed to send RTP packet: {}", e)))?;

        Ok(())
    }

    fn parse_rtp_packet(data: &[u8]) -> StreamResult<Vec<u8>> {
        if data.len() < 12 {
            return Err(StreamError::network("Invalid RTP packet size"));
        }

        // Extract payload (skip 12-byte header)
        Ok(data[12..].to_vec())
    }

    async fn estimate_bitrate(&self, connection: &QuinnConnection) -> u32 {
        let stats = connection.stats();
        
        // Estimate bitrate from congestion window and RTT
        if stats.path.rtt.as_millis() > 0 {
            ((stats.path.cwnd * 8 * 1000) / stats.path.rtt.as_millis() as u64) as u32
        } else {
            0
        }
    }
}

impl StreamMultiplexer {
    fn new() -> Self {
        Self {
            next_stream_id: 0,
            stream_assignments: HashMap::new(),
        }
    }

    fn register_stream(&mut self, peer_id: &PeerId) {
        let stream_id = self.next_stream_id;
        self.next_stream_id += 1;
        
        self.stream_assignments
            .entry(peer_id.clone())
            .or_insert_with(Vec::new)
            .push(stream_id);
    }

    fn unregister_stream(&mut self, peer_id: &PeerId) {
        self.stream_assignments.remove(peer_id);
    }

    fn get_stream_ids(&self, peer_id: &PeerId) -> Option<&Vec<u64>> {
        self.stream_assignments.get(peer_id)
    }
}

impl RtpHeader {
    /// Create a new RTP header for video streaming
    pub fn new(sequence_number: u16, timestamp: u32, marker: bool) -> Self {
        Self {
            version: 2,
            padding: false,
            extension: false,
            csrc_count: 0,
            marker,
            payload_type: 96, // H.264
            sequence_number,
            timestamp,
            ssrc: 0x12345678,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_quic_streamer_creation() {
        let streamer = QuicVideoStreamer::new();
        assert!(streamer.is_ok());
    }

    #[test]
    fn test_quic_config_defaults() {
        let config = QuicStreamerConfig::default();
        assert_eq!(config.max_concurrent_streams, 10);
        assert_eq!(config.video_stream_priority, 200);
        assert!(config.enable_multiplexing);
        assert!(config.enable_0rtt);
    }

    #[test]
    fn test_quality_levels() {
        assert_eq!(QualityLevel::Low, QualityLevel::Low);
        assert_ne!(QualityLevel::Low, QualityLevel::High);
    }

    #[test]
    fn test_rtp_header_creation() {
        let header = RtpHeader::new(100, 12345, true);
        assert_eq!(header.version, 2);
        assert_eq!(header.sequence_number, 100);
        assert_eq!(header.timestamp, 12345);
        assert!(header.marker);
    }

    #[test]
    fn test_stream_multiplexer() {
        let mut multiplexer = StreamMultiplexer::new();
        let peer_id = "test-peer".to_string();
        
        multiplexer.register_stream(&peer_id);
        assert!(multiplexer.get_stream_ids(&peer_id).is_some());
        
        multiplexer.unregister_stream(&peer_id);
        assert!(multiplexer.get_stream_ids(&peer_id).is_none());
    }
}
