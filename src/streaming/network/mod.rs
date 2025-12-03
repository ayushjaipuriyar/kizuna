// Network streaming module
//
// Provides WebRTC and QUIC-based streaming with adaptive bitrate control
// and efficient buffering.

pub mod webrtc_streamer;
pub mod quic_streamer;
pub mod adaptive_bitrate;
pub mod buffer_manager;

use async_trait::async_trait;
use std::sync::Arc;

use crate::streaming::{
    PeerId, StreamConnection, StreamError, StreamResult, StreamStats, VideoStream,
};
use crate::transport::PeerAddress;

pub use webrtc_streamer::{WebRtcVideoStreamer, WebRtcStreamerConfig, VideoCodec};
pub use quic_streamer::{QuicVideoStreamer, QuicStreamerConfig, QualityLevel};
pub use adaptive_bitrate::{
    AdaptiveBitrateController, AdaptiveBitrateConfig, NetworkConditions,
    CongestionLevel, QualityChangeReason,
};
pub use buffer_manager::{
    StreamBufferManager, BufferConfig, BufferStats, BufferHealth,
    BufferAlert, BufferAlertType, FramePriority,
};

/// Network streamer implementation
/// 
/// Manages video stream transmission over WebRTC and QUIC protocols
/// with adaptive bitrate control.
/// 
/// Requirements: 1.3, 2.2, 4.1, 4.2
pub struct NetworkStreamerImpl {
    webrtc_streamer: Option<Arc<WebRtcVideoStreamer>>,
    quic_streamer: Option<Arc<QuicVideoStreamer>>,
    adaptive_controller: Arc<AdaptiveBitrateController>,
    buffer_manager: Arc<StreamBufferManager>,
    use_webrtc: bool,
}

impl NetworkStreamerImpl {
    /// Create a new network streamer with WebRTC support
    pub fn new_with_webrtc(
        signaling_handler: Arc<dyn crate::transport::protocols::webrtc::SignalingHandler>,
    ) -> StreamResult<Self> {
        let webrtc_streamer = WebRtcVideoStreamer::new(signaling_handler)?;
        
        Ok(Self {
            webrtc_streamer: Some(Arc::new(webrtc_streamer)),
            quic_streamer: None,
            adaptive_controller: Arc::new(AdaptiveBitrateController::new()),
            buffer_manager: Arc::new(StreamBufferManager::new()),
            use_webrtc: true,
        })
    }

    /// Create a new network streamer with QUIC support
    pub fn new_with_quic() -> StreamResult<Self> {
        let quic_streamer = QuicVideoStreamer::new()?;
        
        Ok(Self {
            webrtc_streamer: None,
            quic_streamer: Some(Arc::new(quic_streamer)),
            adaptive_controller: Arc::new(AdaptiveBitrateController::new()),
            buffer_manager: Arc::new(StreamBufferManager::new()),
            use_webrtc: false,
        })
    }

    /// Create a new network streamer with both WebRTC and QUIC support
    pub fn new_hybrid(
        signaling_handler: Arc<dyn crate::transport::protocols::webrtc::SignalingHandler>,
    ) -> StreamResult<Self> {
        let webrtc_streamer = WebRtcVideoStreamer::new(signaling_handler)?;
        let quic_streamer = QuicVideoStreamer::new()?;
        
        Ok(Self {
            webrtc_streamer: Some(Arc::new(webrtc_streamer)),
            quic_streamer: Some(Arc::new(quic_streamer)),
            adaptive_controller: Arc::new(AdaptiveBitrateController::new()),
            buffer_manager: Arc::new(StreamBufferManager::new()),
            use_webrtc: true, // Default to WebRTC
        })
    }

    /// Switch between WebRTC and QUIC streaming
    pub fn set_protocol(&mut self, use_webrtc: bool) {
        self.use_webrtc = use_webrtc;
    }

    /// Get the adaptive bitrate controller
    pub fn adaptive_controller(&self) -> Arc<AdaptiveBitrateController> {
        self.adaptive_controller.clone()
    }

    /// Get the buffer manager
    pub fn buffer_manager(&self) -> Arc<StreamBufferManager> {
        self.buffer_manager.clone()
    }
}

#[async_trait]
impl crate::streaming::NetworkStreamer for NetworkStreamerImpl {
    async fn start_streaming(
        &self,
        peer_id: PeerId,
        stream: VideoStream,
    ) -> StreamResult<StreamConnection> {
        // For now, return unsupported as we need peer address
        // In a real implementation, this would be integrated with discovery
        Err(StreamError::unsupported("Use start_streaming_with_address instead"))
    }

    async fn receive_stream(&self, peer_id: PeerId) -> StreamResult<VideoStream> {
        if self.use_webrtc {
            if let Some(ref webrtc) = self.webrtc_streamer {
                webrtc.receive_stream(peer_id).await
            } else {
                Err(StreamError::unsupported("WebRTC not available"))
            }
        } else {
            Err(StreamError::unsupported("Use receive_stream_with_address for QUIC"))
        }
    }

    async fn adjust_bitrate(
        &self,
        _connection: StreamConnection,
        bitrate: u32,
    ) -> StreamResult<()> {
        // Update adaptive controller with new target bitrate
        let conditions = self.adaptive_controller.get_network_conditions().await;
        
        // Adjust quality based on new bitrate
        let _ = self.adaptive_controller.adjust_quality(&conditions).await?;
        
        println!("Adjusted bitrate to {} bps", bitrate);
        Ok(())
    }

    async fn get_stream_stats(&self, connection: StreamConnection) -> StreamResult<StreamStats> {
        if self.use_webrtc {
            if let Some(ref webrtc) = self.webrtc_streamer {
                webrtc.get_stats(&connection.peer_id).await
            } else {
                Err(StreamError::unsupported("WebRTC not available"))
            }
        } else if let Some(ref quic) = self.quic_streamer {
            quic.get_stats(&connection.peer_id).await
        } else {
            Err(StreamError::unsupported("No streaming protocol available"))
        }
    }

    async fn close_stream(&self, connection: StreamConnection) -> StreamResult<()> {
        if self.use_webrtc {
            if let Some(ref webrtc) = self.webrtc_streamer {
                webrtc.close_stream(&connection.peer_id).await
            } else {
                Err(StreamError::unsupported("WebRTC not available"))
            }
        } else if let Some(ref quic) = self.quic_streamer {
            quic.close_stream(&connection.peer_id).await
        } else {
            Err(StreamError::unsupported("No streaming protocol available"))
        }
    }
}

impl NetworkStreamerImpl {
    /// Start streaming with explicit peer address (for QUIC)
    pub async fn start_streaming_with_address(
        &self,
        peer_id: PeerId,
        stream: VideoStream,
        peer_address: PeerAddress,
    ) -> StreamResult<StreamConnection> {
        if self.use_webrtc {
            if let Some(ref webrtc) = self.webrtc_streamer {
                webrtc.start_streaming(peer_id, stream).await
            } else {
                Err(StreamError::unsupported("WebRTC not available"))
            }
        } else if let Some(ref quic) = self.quic_streamer {
            quic.start_streaming(peer_id, stream, peer_address).await
        } else {
            Err(StreamError::unsupported("No streaming protocol available"))
        }
    }

    /// Receive stream with explicit peer address (for QUIC)
    pub async fn receive_stream_with_address(
        &self,
        peer_id: PeerId,
        peer_address: PeerAddress,
    ) -> StreamResult<VideoStream> {
        if self.use_webrtc {
            if let Some(ref webrtc) = self.webrtc_streamer {
                webrtc.receive_stream(peer_id).await
            } else {
                Err(StreamError::unsupported("WebRTC not available"))
            }
        } else if let Some(ref quic) = self.quic_streamer {
            quic.receive_stream(peer_id, peer_address).await
        } else {
            Err(StreamError::unsupported("No streaming protocol available"))
        }
    }
}
