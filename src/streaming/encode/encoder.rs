// H.264 encoder with hardware acceleration support
//
// Provides H.264 encoding using hardware acceleration (NVENC, QuickSync, VCE)
// with software fallback using GStreamer.
//
// Requirements: 1.2, 9.1

use std::time::SystemTime;
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;

use crate::streaming::{
    EncodedFrame, EncoderConfig, EncodingQuality, PixelFormat, StreamError, StreamResult,
    VideoFrame,
};

/// Hardware acceleration types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareAccelerator {
    /// NVIDIA NVENC
    NVENC,
    /// Intel Quick Sync Video
    QuickSync,
    /// AMD VCE (Video Coding Engine)
    VCE,
    /// Apple VideoToolbox
    VideoToolbox,
    /// Software fallback
    Software,
}

impl HardwareAccelerator {
    /// Detect available hardware accelerators
    pub fn detect_available_accelerators() -> StreamResult<Vec<HardwareAccelerator>> {
        gst::init().map_err(|e| StreamError::initialization(format!("GStreamer init failed: {}", e)))?;
        
        let mut accelerators = Vec::new();
        
        // Check for NVENC (NVIDIA)
        if let Some(_) = gst::ElementFactory::find("nvh264enc") {
            accelerators.push(HardwareAccelerator::NVENC);
        }
        
        // Check for Quick Sync (Intel)
        if let Some(_) = gst::ElementFactory::find("mfh264enc") {
            accelerators.push(HardwareAccelerator::QuickSync);
        }
        
        // Check for VCE (AMD)
        if let Some(_) = gst::ElementFactory::find("vaapih264enc") {
            accelerators.push(HardwareAccelerator::VCE);
        }
        
        // Check for VideoToolbox (Apple)
        #[cfg(target_os = "macos")]
        if let Some(_) = gst::ElementFactory::find("vtenc_h264") {
            accelerators.push(HardwareAccelerator::VideoToolbox);
        }
        
        // Software fallback is always available
        accelerators.push(HardwareAccelerator::Software);
        
        if accelerators.is_empty() {
            return Err(StreamError::unsupported("No encoders available"));
        }
        
        Ok(accelerators)
    }

    /// Get the GStreamer element name for this accelerator
    fn element_name(&self) -> &'static str {
        match self {
            HardwareAccelerator::NVENC => "nvh264enc",
            HardwareAccelerator::QuickSync => "mfh264enc",
            HardwareAccelerator::VCE => "vaapih264enc",
            HardwareAccelerator::VideoToolbox => "vtenc_h264",
            HardwareAccelerator::Software => "x264enc",
        }
    }
}

/// Encoder backend implementation
pub enum EncoderBackend {
    Hardware {
        accelerator: HardwareAccelerator,
        pipeline: gst::Pipeline,
        appsrc: gst_app::AppSrc,
        appsink: gst_app::AppSink,
    },
    Software {
        pipeline: gst::Pipeline,
        appsrc: gst_app::AppSrc,
        appsink: gst_app::AppSink,
    },
}

impl EncoderBackend {
    /// Create a new encoder backend
    fn new(config: &EncoderConfig, use_hardware: bool) -> StreamResult<Self> {
        gst::init().map_err(|e| StreamError::initialization(format!("GStreamer init failed: {}", e)))?;
        
        let accelerators = if use_hardware {
            HardwareAccelerator::detect_available_accelerators()?
        } else {
            vec![HardwareAccelerator::Software]
        };
        
        // Try hardware accelerators first, then fall back to software
        for accelerator in accelerators {
            if let Ok(backend) = Self::create_pipeline(config, accelerator) {
                return Ok(backend);
            }
        }
        
        Err(StreamError::encoding("Failed to create encoder pipeline"))
    }

    /// Create GStreamer pipeline for encoding
    fn create_pipeline(config: &EncoderConfig, accelerator: HardwareAccelerator) -> StreamResult<Self> {
        let pipeline = gst::Pipeline::with_name("encoder_pipeline");
        
        // Create appsrc for input frames
        let appsrc = gst::ElementFactory::make("appsrc")
            .name("src")
            .build()
            .map_err(|e| StreamError::encoding(format!("Failed to create appsrc: {}", e)))?;
        
        let appsrc = appsrc
            .dynamic_cast::<gst_app::AppSrc>()
            .map_err(|_| StreamError::encoding("Failed to cast to AppSrc"))?;
        
        // Configure appsrc
        appsrc.set_caps(Some(&Self::create_caps(config)?));
        appsrc.set_property("format", gst::Format::Time);
        appsrc.set_property("is-live", true);
        
        // Create encoder element
        let encoder = gst::ElementFactory::make(accelerator.element_name())
            .name("encoder")
            .build()
            .map_err(|e| StreamError::encoding(format!("Failed to create encoder: {}", e)))?;
        
        // Configure encoder parameters
        Self::configure_encoder(&encoder, config, accelerator)?;
        
        // Create h264parse element
        let h264parse = gst::ElementFactory::make("h264parse")
            .name("parse")
            .build()
            .map_err(|e| StreamError::encoding(format!("Failed to create h264parse: {}", e)))?;
        
        // Create appsink for output
        let appsink = gst::ElementFactory::make("appsink")
            .name("sink")
            .build()
            .map_err(|e| StreamError::encoding(format!("Failed to create appsink: {}", e)))?;
        
        let appsink = appsink
            .dynamic_cast::<gst_app::AppSink>()
            .map_err(|_| StreamError::encoding("Failed to cast to AppSink"))?;
        
        appsink.set_property("emit-signals", false);
        appsink.set_property("sync", false);
        
        // Add elements to pipeline
        pipeline.add_many(&[appsrc.upcast_ref(), &encoder, &h264parse, appsink.upcast_ref()])
            .map_err(|e| StreamError::encoding(format!("Failed to add elements: {}", e)))?;
        
        // Link elements
        gst::Element::link_many(&[appsrc.upcast_ref(), &encoder, &h264parse, appsink.upcast_ref()])
            .map_err(|e| StreamError::encoding(format!("Failed to link elements: {}", e)))?;
        
        // Start pipeline
        pipeline.set_state(gst::State::Playing)
            .map_err(|e| StreamError::encoding(format!("Failed to start pipeline: {}", e)))?;
        
        if accelerator == HardwareAccelerator::Software {
            Ok(EncoderBackend::Software {
                pipeline,
                appsrc,
                appsink,
            })
        } else {
            Ok(EncoderBackend::Hardware {
                accelerator,
                pipeline,
                appsrc,
                appsink,
            })
        }
    }

    /// Create GStreamer caps for the input format
    fn create_caps(config: &EncoderConfig) -> StreamResult<gst::Caps> {
        let caps = gst::Caps::builder("video/x-raw")
            .field("format", "I420")
            .field("width", config.resolution.width as i32)
            .field("height", config.resolution.height as i32)
            .field("framerate", gst::Fraction::new(config.framerate as i32, 1))
            .build();
        
        Ok(caps)
    }

    /// Configure encoder element parameters
    fn configure_encoder(
        encoder: &gst::Element,
        config: &EncoderConfig,
        accelerator: HardwareAccelerator,
    ) -> StreamResult<()> {
        match accelerator {
            HardwareAccelerator::NVENC => {
                encoder.set_property("bitrate", config.bitrate / 1000); // kbps
                encoder.set_property("preset", "low-latency-hq");
            }
            HardwareAccelerator::QuickSync => {
                encoder.set_property("bitrate", config.bitrate / 1000); // kbps
                encoder.set_property("rate-control", "cbr");
            }
            HardwareAccelerator::VCE => {
                encoder.set_property("bitrate", config.bitrate / 1000); // kbps
                encoder.set_property("rate-control", "cbr");
            }
            HardwareAccelerator::VideoToolbox => {
                encoder.set_property("bitrate", config.bitrate / 1000); // kbps
            }
            HardwareAccelerator::Software => {
                encoder.set_property("bitrate", config.bitrate / 1000); // kbps
                encoder.set_property("speed-preset", "ultrafast");
                encoder.set_property("tune", "zerolatency");
            }
        }
        
        Ok(())
    }

    /// Encode a video frame
    fn encode(&mut self, frame: VideoFrame, quality: EncodingQuality) -> StreamResult<EncodedFrame> {
        let (appsrc, appsink) = match self {
            EncoderBackend::Hardware { appsrc, appsink, .. } => (appsrc, appsink),
            EncoderBackend::Software { appsrc, appsink, .. } => (appsrc, appsink),
        };
        
        // Convert frame data to GStreamer buffer
        let mut buffer = gst::Buffer::from_slice(frame.data);
        {
            let buffer_ref = buffer.get_mut().unwrap();
            buffer_ref.set_pts(gst::ClockTime::from_nseconds(
                frame.timestamp.duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64
            ));
        }
        
        // Push buffer to appsrc
        appsrc.push_buffer(buffer)
            .map_err(|e| StreamError::encoding(format!("Failed to push buffer: {:?}", e)))?;
        
        // Pull encoded sample from appsink
        let sample = appsink.pull_sample()
            .map_err(|e| StreamError::encoding(format!("Failed to pull sample: {:?}", e)))?;
        
        let buffer = sample.buffer()
            .ok_or_else(|| StreamError::encoding("No buffer in sample"))?;
        
        let map = buffer.map_readable()
            .map_err(|e| StreamError::encoding(format!("Failed to map buffer: {}", e)))?;
        
        let data = map.as_slice().to_vec();
        
        // Check if this is a keyframe
        let is_keyframe = !buffer.flags().contains(gst::BufferFlags::DELTA_UNIT);
        
        Ok(EncodedFrame {
            data,
            timestamp: frame.timestamp,
            is_keyframe,
        })
    }
}

impl Drop for EncoderBackend {
    fn drop(&mut self) {
        let pipeline = match self {
            EncoderBackend::Hardware { pipeline, .. } => pipeline,
            EncoderBackend::Software { pipeline, .. } => pipeline,
        };
        
        let _ = pipeline.set_state(gst::State::Null);
    }
}

/// H.264 encoder with hardware acceleration
///
/// Requirements: 1.2, 9.1
pub struct H264Encoder {
    backend: EncoderBackend,
    config: EncoderConfig,
}

impl H264Encoder {
    /// Create a new H.264 encoder
    pub fn new(config: EncoderConfig, use_hardware: bool) -> StreamResult<Self> {
        let backend = EncoderBackend::new(&config, use_hardware)?;
        
        Ok(Self {
            backend,
            config,
        })
    }

    /// Encode a video frame
    pub fn encode(&mut self, frame: VideoFrame, quality: EncodingQuality) -> StreamResult<EncodedFrame> {
        // Validate frame format
        if frame.format != PixelFormat::YUV420 {
            return Err(StreamError::encoding("Only YUV420 format is supported"));
        }
        
        // Validate frame dimensions
        if frame.width != self.config.resolution.width || frame.height != self.config.resolution.height {
            return Err(StreamError::encoding("Frame dimensions don't match encoder configuration"));
        }
        
        self.backend.encode(frame, quality)
    }

    /// Get encoder configuration
    pub fn config(&self) -> &EncoderConfig {
        &self.config
    }

    /// Check if using hardware acceleration
    pub fn is_hardware_accelerated(&self) -> bool {
        matches!(self.backend, EncoderBackend::Hardware { .. })
    }
}
