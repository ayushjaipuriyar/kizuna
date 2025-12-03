// Video encoding and decoding module
//
// Provides H.264 encoding/decoding with hardware acceleration support
// and adaptive quality scaling.

use async_trait::async_trait;
use std::sync::{Arc, Mutex};

use crate::streaming::{
    EncodedFrame, EncoderCapabilities, EncoderConfig, EncodingQuality, StreamError, StreamResult,
    VideoFrame, VideoCodecType, Resolution, PixelFormat,
};

mod encoder;
mod decoder;
mod performance;

pub use encoder::{H264Encoder, HardwareAccelerator, EncoderBackend};
pub use decoder::{H264Decoder, DecoderBackend};
pub use performance::{EncoderPerformanceMonitor, EncoderSelector, EncoderOptimizer};

/// Video codec implementation with hardware acceleration
/// 
/// Provides H.264 encoding and decoding with automatic hardware acceleration
/// detection and software fallback.
/// 
/// Requirements: 1.2, 2.1, 9.1
pub struct VideoCodecImpl {
    encoder: Arc<Mutex<Option<H264Encoder>>>,
    decoder: Arc<Mutex<Option<H264Decoder>>>,
    config: Arc<Mutex<Option<EncoderConfig>>>,
    hardware_acceleration_enabled: bool,
}

impl VideoCodecImpl {
    /// Create a new video codec instance
    pub fn new() -> Self {
        Self {
            encoder: Arc::new(Mutex::new(None)),
            decoder: Arc::new(Mutex::new(None)),
            config: Arc::new(Mutex::new(None)),
            hardware_acceleration_enabled: false,
        }
    }

    /// Initialize encoder with current configuration
    fn init_encoder(&self) -> StreamResult<()> {
        let config = self.config.lock().unwrap();
        let config = config.as_ref().ok_or_else(|| {
            StreamError::configuration("Encoder not configured")
        })?;

        let encoder = H264Encoder::new(config.clone(), self.hardware_acceleration_enabled)?;
        *self.encoder.lock().unwrap() = Some(encoder);
        Ok(())
    }

    /// Initialize decoder
    fn init_decoder(&self) -> StreamResult<()> {
        let decoder = H264Decoder::new(self.hardware_acceleration_enabled)?;
        *self.decoder.lock().unwrap() = Some(decoder);
        Ok(())
    }
}

#[async_trait]
impl crate::streaming::VideoCodec for VideoCodecImpl {
    async fn encode_frame(
        &self,
        frame: VideoFrame,
        quality: EncodingQuality,
    ) -> StreamResult<EncodedFrame> {
        let mut encoder_guard = self.encoder.lock().unwrap();
        
        if encoder_guard.is_none() {
            drop(encoder_guard);
            self.init_encoder()?;
            encoder_guard = self.encoder.lock().unwrap();
        }

        let encoder = encoder_guard.as_mut().ok_or_else(|| {
            StreamError::encoding("Encoder not initialized")
        })?;

        encoder.encode(frame, quality)
    }

    async fn decode_frame(&self, data: &[u8]) -> StreamResult<VideoFrame> {
        let mut decoder_guard = self.decoder.lock().unwrap();
        
        if decoder_guard.is_none() {
            drop(decoder_guard);
            self.init_decoder()?;
            decoder_guard = self.decoder.lock().unwrap();
        }

        let decoder = decoder_guard.as_mut().ok_or_else(|| {
            StreamError::decoding("Decoder not initialized")
        })?;

        decoder.decode(data)
    }

    async fn configure_encoder(&self, config: EncoderConfig) -> StreamResult<()> {
        *self.config.lock().unwrap() = Some(config);
        
        // Reinitialize encoder if it was already created
        if self.encoder.lock().unwrap().is_some() {
            self.init_encoder()?;
        }
        
        Ok(())
    }

    async fn get_encoder_capabilities(&self) -> StreamResult<EncoderCapabilities> {
        let hw_available = HardwareAccelerator::detect_available_accelerators().is_ok();
        
        Ok(EncoderCapabilities {
            supported_codecs: vec![VideoCodecType::H264],
            hardware_acceleration_available: hw_available,
            max_resolution: Resolution { width: 3840, height: 2160 }, // 4K
            max_framerate: 60,
        })
    }

    async fn enable_hardware_acceleration(&self) -> StreamResult<bool> {
        let available = HardwareAccelerator::detect_available_accelerators().is_ok();
        Ok(available)
    }
}

impl Default for VideoCodecImpl {
    fn default() -> Self {
        Self::new()
    }
}

mod quality;

pub use quality::{QualityScaler, BitrateController, AdaptiveQualityManager};
