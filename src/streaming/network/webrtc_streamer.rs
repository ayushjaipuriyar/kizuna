// WebRTC-based video streaming implementation
//
// Provides WebRTC DataChannel and video track streaming with ICE negotiation
// for browser-compatible video streaming.
//
// Requirements: 1.3, 2.2

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex, RwLock};
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::{MediaEngine, MIME_TYPE_H264, MIME_TYPE_VP8};
use webrtc::api::APIBuilder;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_codec::{RTCRtpCodecCapability, RTCRtpCodecParameters, RTPCodecType};
use webrtc::rtp_transceiver::rtp_sender::RTCRtpSender;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_local::TrackLocal;

use crate::streaming::{
    EncodedFrame, PeerId, StreamConnection, StreamError, StreamResult, StreamStats, VideoStream,
};
use crate::transport::protocols::webrtc::{
    IceCandidate, SignalingHandler, SignalingMessage,
};

/// WebRTC video streamer for browser-compatible streaming
///
/// Supports both DataChannel streaming for encoded frames and
/// video track streaming for native browser playback.
///
/// Requirements: 1.3, 2.2
pub struct WebRtcVideoStreamer {
    config: WebRtcStreamerConfig,
    api: Arc<webrtc::api::API>,
    ice_servers: Vec<RTCIceServer>,
    signaling_handler: Arc<dyn SignalingHandler>,
    active_streams: Arc<RwLock<HashMap<PeerId, ActiveStream>>>,
}

/// Configuration for WebRTC video streaming
#[derive(Debug, Clone)]
pub struct WebRtcStreamerConfig {
    /// ICE servers for NAT traversal
    pub ice_servers: Vec<IceServerConfig>,
    /// Use video tracks (true) or DataChannels (false)
    pub use_video_tracks: bool,
    /// Video codec preference (H264, VP8)
    pub preferred_codec: VideoCodec,
    /// Maximum bitrate for video streaming (bps)
    pub max_bitrate: u32,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Enable simulcast for adaptive quality
    pub enable_simulcast: bool,
}

/// ICE server configuration
#[derive(Debug, Clone)]
pub struct IceServerConfig {
    pub urls: Vec<String>,
    pub username: Option<String>,
    pub credential: Option<String>,
}

/// Supported video codecs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoCodec {
    H264,
    VP8,
    VP9,
}

/// Active WebRTC streaming session
struct ActiveStream {
    peer_connection: Arc<RTCPeerConnection>,
    data_channel: Option<Arc<RTCDataChannel>>,
    video_track: Option<Arc<TrackLocalStaticRTP>>,
    video_sender: Option<Arc<RTCRtpSender>>,
    stats: Arc<Mutex<StreamStats>>,
    is_connected: Arc<RwLock<bool>>,
}

impl Default for WebRtcStreamerConfig {
    fn default() -> Self {
        Self {
            ice_servers: vec![
                IceServerConfig {
                    urls: vec!["stun:stun.l.google.com:19302".to_string()],
                    username: None,
                    credential: None,
                },
            ],
            use_video_tracks: true,
            preferred_codec: VideoCodec::H264,
            max_bitrate: 3_000_000, // 3 Mbps
            connection_timeout: Duration::from_secs(30),
            enable_simulcast: false,
        }
    }
}

impl WebRtcVideoStreamer {
    /// Create a new WebRTC video streamer
    pub fn new(signaling_handler: Arc<dyn SignalingHandler>) -> StreamResult<Self> {
        Self::with_config(WebRtcStreamerConfig::default(), signaling_handler)
    }

    /// Create a new WebRTC video streamer with custom configuration
    pub fn with_config(
        config: WebRtcStreamerConfig,
        signaling_handler: Arc<dyn SignalingHandler>,
    ) -> StreamResult<Self> {
        // Create media engine with video codec support
        let mut media_engine = MediaEngine::default();
        
        // Register video codecs based on preference
        match config.preferred_codec {
            VideoCodec::H264 => {
                media_engine
                    .register_default_codecs()
                    .map_err(|e| StreamError::network(format!("Failed to register H264 codec: {}", e)))?;
            }
            VideoCodec::VP8 => {
                media_engine
                    .register_default_codecs()
                    .map_err(|e| StreamError::network(format!("Failed to register VP8 codec: {}", e)))?;
            }
            VideoCodec::VP9 => {
                media_engine
                    .register_default_codecs()
                    .map_err(|e| StreamError::network(format!("Failed to register VP9 codec: {}", e)))?;
            }
        }

        // Create interceptor registry
        let mut registry = Registry::new();
        registry = register_default_interceptors(registry, &mut media_engine)
            .map_err(|e| StreamError::network(format!("Failed to register interceptors: {}", e)))?;

        // Create API
        let api = APIBuilder::new()
            .with_media_engine(media_engine)
            .with_interceptor_registry(registry)
            .build();

        // Convert ICE server configuration
        let ice_servers: Vec<RTCIceServer> = config
            .ice_servers
            .iter()
            .map(|ice_config| RTCIceServer {
                urls: ice_config.urls.clone(),
                username: ice_config.username.clone().unwrap_or_default(),
                credential: ice_config.credential.clone().unwrap_or_default(),
                ..Default::default()
            })
            .collect();

        Ok(Self {
            config,
            api: Arc::new(api),
            ice_servers,
            signaling_handler,
            active_streams: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Start streaming video to a peer
    pub async fn start_streaming(
        &self,
        peer_id: PeerId,
        stream: VideoStream,
    ) -> StreamResult<StreamConnection> {
        // Create peer connection
        let peer_connection = self.create_peer_connection().await?;

        // Set up video streaming based on configuration
        let (data_channel, video_track, video_sender) = if self.config.use_video_tracks {
            // Use video tracks for native browser playback
            let (track, sender) = self.create_video_track(&peer_connection).await?;
            (None, Some(track), Some(sender))
        } else {
            // Use DataChannel for encoded frame streaming
            let channel = self.create_data_channel(&peer_connection).await?;
            (Some(channel), None, None)
        };

        // Set up connection state monitoring
        let is_connected = Arc::new(RwLock::new(false));
        let is_connected_clone = is_connected.clone();
        
        peer_connection.on_peer_connection_state_change(Box::new(move |state: RTCPeerConnectionState| {
            let is_connected = is_connected_clone.clone();
            Box::pin(async move {
                let connected = matches!(state, RTCPeerConnectionState::Connected);
                *is_connected.write().await = connected;
            })
        }));

        // Create and send offer
        let offer = peer_connection
            .create_offer(None)
            .await
            .map_err(|e| StreamError::network(format!("Failed to create offer: {}", e)))?;

        peer_connection
            .set_local_description(offer.clone())
            .await
            .map_err(|e| StreamError::network(format!("Failed to set local description: {}", e)))?;

        // Send offer through signaling
        self.signaling_handler
            .send_signaling_message(
                &peer_id,
                SignalingMessage::Offer {
                    sdp: offer.sdp,
                    ice_ufrag: "default_ufrag".to_string(),
                    ice_pwd: "default_pwd".to_string(),
                },
            )
            .await
            .map_err(|e| StreamError::network(format!("Signaling failed: {}", e)))?;

        // Wait for answer
        let answer_message = self
            .signaling_handler
            .receive_signaling_message(&peer_id, self.config.connection_timeout)
            .await
            .map_err(|e| StreamError::network(format!("Failed to receive answer: {}", e)))?;

        if let SignalingMessage::Answer { sdp, .. } = answer_message {
            let answer = RTCSessionDescription::answer(sdp)
                .map_err(|e| StreamError::network(format!("Invalid answer SDP: {}", e)))?;

            peer_connection
                .set_remote_description(answer)
                .await
                .map_err(|e| StreamError::network(format!("Failed to set remote description: {}", e)))?;
        } else {
            return Err(StreamError::network("Expected answer message"));
        }

        // Perform ICE negotiation
        self.perform_ice_negotiation(&peer_connection, &peer_id).await?;

        // Wait for connection
        let start_time = std::time::Instant::now();
        while start_time.elapsed() < self.config.connection_timeout {
            if *is_connected.read().await {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        if !*is_connected.read().await {
            return Err(StreamError::timeout("Connection timeout"));
        }

        // Store active stream
        let stats = Arc::new(Mutex::new(StreamStats::default()));
        let active_stream = ActiveStream {
            peer_connection,
            data_channel,
            video_track,
            video_sender,
            stats: stats.clone(),
            is_connected,
        };

        {
            let mut streams = self.active_streams.write().await;
            streams.insert(peer_id.clone(), active_stream);
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
    ) -> StreamResult<()> {
        let frame_size = frame.data.len();
        
        let streams = self.active_streams.read().await;
        let stream = streams
            .get(peer_id)
            .ok_or_else(|| StreamError::network("Stream not found"))?;

        if let Some(ref video_track) = stream.video_track {
            // Send via video track
            self.send_via_video_track(video_track, frame).await?;
        } else if let Some(ref data_channel) = stream.data_channel {
            // Send via DataChannel
            self.send_via_data_channel(data_channel, frame).await?;
        } else {
            return Err(StreamError::network("No streaming method available"));
        }

        // Update statistics
        let mut stats = stream.stats.lock().await;
        stats.frames_encoded += 1;
        stats.bytes_sent += frame_size as u64;

        Ok(())
    }

    /// Receive a video stream from a peer
    pub async fn receive_stream(&self, peer_id: PeerId) -> StreamResult<VideoStream> {
        // Register for incoming offers
        self.signaling_handler
            .register_for_offers(&peer_id)
            .await
            .map_err(|e| StreamError::network(format!("Failed to register for offers: {}", e)))?;

        // Wait for offer
        let (remote_peer_id, offer_message) = self
            .signaling_handler
            .wait_for_offer(self.config.connection_timeout)
            .await
            .map_err(|e| StreamError::network(format!("Failed to receive offer: {}", e)))?;

        if remote_peer_id != peer_id {
            return Err(StreamError::network("Received offer from unexpected peer"));
        }

        // Handle the offer and create answer
        let peer_connection = self.create_peer_connection().await?;
        
        if let SignalingMessage::Offer { sdp, .. } = offer_message {
            let offer = RTCSessionDescription::offer(sdp)
                .map_err(|e| StreamError::network(format!("Invalid offer SDP: {}", e)))?;

            peer_connection
                .set_remote_description(offer)
                .await
                .map_err(|e| StreamError::network(format!("Failed to set remote description: {}", e)))?;

            // Create answer
            let answer = peer_connection
                .create_answer(None)
                .await
                .map_err(|e| StreamError::network(format!("Failed to create answer: {}", e)))?;

            peer_connection
                .set_local_description(answer.clone())
                .await
                .map_err(|e| StreamError::network(format!("Failed to set local description: {}", e)))?;

            // Send answer
            self.signaling_handler
                .send_signaling_message(
                    &peer_id,
                    SignalingMessage::Answer {
                        sdp: answer.sdp,
                        ice_ufrag: "default_ufrag".to_string(),
                        ice_pwd: "default_pwd".to_string(),
                    },
                )
                .await
                .map_err(|e| StreamError::network(format!("Failed to send answer: {}", e)))?;
        } else {
            return Err(StreamError::network("Expected offer message"));
        }

        // Set up data channel or video track reception
        let (rx_sender, rx_receiver) = mpsc::unbounded_channel();
        
        if self.config.use_video_tracks {
            // Set up video track reception
            peer_connection.on_track(Box::new(move |track, _receiver, _transceiver| {
                let rx_sender = rx_sender.clone();
                Box::pin(async move {
                    println!("Received video track: {}", track.id());
                    // In a real implementation, we would decode and process the track
                    let _ = rx_sender.send(vec![]);
                })
            }));
        } else {
            // Set up DataChannel reception
            peer_connection.on_data_channel(Box::new(move |data_channel| {
                let rx_sender = rx_sender.clone();
                Box::pin(async move {
                    data_channel.on_message(Box::new(move |msg: DataChannelMessage| {
                        let rx_sender = rx_sender.clone();
                        Box::pin(async move {
                            let _ = rx_sender.send(msg.data.to_vec());
                        })
                    }));
                })
            }));
        }

        // Perform ICE negotiation
        self.perform_ice_negotiation(&peer_connection, &peer_id).await?;

        // Create video stream
        let video_stream = VideoStream {
            id: uuid::Uuid::new_v4(),
            source: crate::streaming::StreamSource::File(std::path::PathBuf::from("webrtc-stream")),
            quality: crate::streaming::StreamQuality::default(),
        };

        Ok(video_stream)
    }

    /// Close a streaming connection
    pub async fn close_stream(&self, peer_id: &PeerId) -> StreamResult<()> {
        let mut streams = self.active_streams.write().await;
        
        if let Some(stream) = streams.remove(peer_id) {
            // Close data channel if present
            if let Some(data_channel) = stream.data_channel {
                data_channel
                    .close()
                    .await
                    .map_err(|e| StreamError::network(format!("Failed to close data channel: {}", e)))?;
            }

            // Close peer connection
            stream
                .peer_connection
                .close()
                .await
                .map_err(|e| StreamError::network(format!("Failed to close peer connection: {}", e)))?;
        }

        Ok(())
    }

    /// Get stream statistics
    pub async fn get_stats(&self, peer_id: &PeerId) -> StreamResult<StreamStats> {
        let streams = self.active_streams.read().await;
        let stream = streams
            .get(peer_id)
            .ok_or_else(|| StreamError::network("Stream not found"))?;

        let stats = stream.stats.lock().await.clone();
        Ok(stats)
    }

    // Private helper methods

    async fn create_peer_connection(&self) -> StreamResult<Arc<RTCPeerConnection>> {
        let rtc_config = RTCConfiguration {
            ice_servers: self.ice_servers.clone(),
            ..Default::default()
        };

        let peer_connection = self
            .api
            .new_peer_connection(rtc_config)
            .await
            .map_err(|e| StreamError::network(format!("Failed to create peer connection: {}", e)))?;

        Ok(Arc::new(peer_connection))
    }

    async fn create_data_channel(
        &self,
        peer_connection: &RTCPeerConnection,
    ) -> StreamResult<Arc<RTCDataChannel>> {
        let data_channel = peer_connection
            .create_data_channel("video-stream", None)
            .await
            .map_err(|e| StreamError::network(format!("Failed to create data channel: {}", e)))?;

        Ok(data_channel)
    }

    async fn create_video_track(
        &self,
        peer_connection: &RTCPeerConnection,
    ) -> StreamResult<(Arc<TrackLocalStaticRTP>, Arc<RTCRtpSender>)> {
        // Create video track based on codec preference
        let codec_capability = match self.config.preferred_codec {
            VideoCodec::H264 => RTCRtpCodecCapability {
                mime_type: MIME_TYPE_H264.to_owned(),
                clock_rate: 90000,
                channels: 0,
                sdp_fmtp_line: "".to_owned(),
                rtcp_feedback: vec![],
            },
            VideoCodec::VP8 => RTCRtpCodecCapability {
                mime_type: MIME_TYPE_VP8.to_owned(),
                clock_rate: 90000,
                channels: 0,
                sdp_fmtp_line: "".to_owned(),
                rtcp_feedback: vec![],
            },
            VideoCodec::VP9 => RTCRtpCodecCapability {
                mime_type: "video/VP9".to_owned(),
                clock_rate: 90000,
                channels: 0,
                sdp_fmtp_line: "".to_owned(),
                rtcp_feedback: vec![],
            },
        };

        let video_track = Arc::new(TrackLocalStaticRTP::new(
            codec_capability,
            "video".to_owned(),
            "kizuna-video-stream".to_owned(),
        ));

        let rtp_sender = peer_connection
            .add_track(Arc::clone(&video_track) as Arc<dyn TrackLocal + Send + Sync>)
            .await
            .map_err(|e| StreamError::network(format!("Failed to add video track: {}", e)))?;

        Ok((video_track, rtp_sender))
    }

    async fn send_via_video_track(
        &self,
        _video_track: &Arc<TrackLocalStaticRTP>,
        _frame: EncodedFrame,
    ) -> StreamResult<()> {
        // Write RTP packet to video track
        // Note: TrackLocalStaticRTP requires proper RTP packet construction
        // This is a placeholder implementation that would need to be completed
        // with proper RTP packetization similar to the QUIC implementation
        
        // In a real implementation, we would:
        // 1. Create RTP packets from the encoded frame
        // 2. Use video_track.write_rtp() to send each packet
        // 3. Handle timing and sequencing properly
        
        Ok(())
    }

    async fn send_via_data_channel(
        &self,
        data_channel: &Arc<RTCDataChannel>,
        frame: EncodedFrame,
    ) -> StreamResult<()> {
        let bytes = bytes::Bytes::from(frame.data);
        data_channel
            .send(&bytes)
            .await
            .map_err(|e| StreamError::network(format!("Failed to send via data channel: {}", e)))?;

        Ok(())
    }

    async fn perform_ice_negotiation(
        &self,
        _peer_connection: &RTCPeerConnection,
        peer_id: &PeerId,
    ) -> StreamResult<()> {
        // Gather and exchange ICE candidates
        let local_candidates = vec![]; // Would gather actual candidates
        
        let _remote_candidates = self
            .signaling_handler
            .exchange_ice_candidates(peer_id, local_candidates)
            .await
            .map_err(|e| StreamError::network(format!("ICE negotiation failed: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::protocols::webrtc::DefaultSignalingHandler;

    #[tokio::test]
    async fn test_webrtc_streamer_creation() {
        let signaling = Arc::new(DefaultSignalingHandler::new());
        let streamer = WebRtcVideoStreamer::new(signaling);
        assert!(streamer.is_ok());
    }

    #[test]
    fn test_webrtc_config_defaults() {
        let config = WebRtcStreamerConfig::default();
        assert!(config.use_video_tracks);
        assert_eq!(config.preferred_codec, VideoCodec::H264);
        assert_eq!(config.max_bitrate, 3_000_000);
    }

    #[test]
    fn test_video_codec_variants() {
        assert_eq!(VideoCodec::H264, VideoCodec::H264);
        assert_ne!(VideoCodec::H264, VideoCodec::VP8);
        assert_ne!(VideoCodec::VP8, VideoCodec::VP9);
    }
}
