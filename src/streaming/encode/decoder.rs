// H.264 decoder with hardware acceleration support
//
// Provides H.264 decoding using hardware acceleration with software fallback.
//
// Requirements: 2.1, 2.2

use std::time::SystemTime;
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use gstreamer_video;

use crate::streaming::{
    PixelFormat, StreamError, StreamResult, VideoFrame,
};

/// Create H.264 input caps
fn create_h264_caps() -> gst::Caps {
    gst::Caps::builder("video/x-h264")
        .field("stream-format", "byte-stream")
        .field("alignment", "au")
        .build()
}

/// Create I420 output caps
fn create_i420_caps() -> gst::Caps {
    gst::Caps::builder("video/x-raw")
        .field("format", "I420")
        .build()
}

/// Decoder backend implementation
pub enum DecoderBackend {
    Hardware {
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

impl DecoderBackend {
    /// Create a new decoder backend
    fn new(use_hardware: bool) -> StreamResult<Self> {
        gst::init().map_err(|e| StreamError::initialization(format!("GStreamer init failed: {}", e)))?;
        
        if use_hardware {
            // Try hardware decoder first
            if let Ok(backend) = Self::create_hardware_pipeline() {
                return Ok(backend);
            }
        }
        
        // Fall back to software decoder
        Self::create_software_pipeline()
    }

    /// Create hardware-accelerated decoder pipeline
    fn create_hardware_pipeline() -> StreamResult<Self> {
        let pipeline = gst::Pipeline::with_name("hw_decoder_pipeline");
        
        // Create appsrc for input data
        let appsrc = gst::ElementFactory::make("appsrc")
            .name("src")
            .build()
            .map_err(|e| StreamError::decoding(format!("Failed to create appsrc: {}", e)))?;
        
        let appsrc = appsrc
            .dynamic_cast::<gst_app::AppSrc>()
            .map_err(|_| StreamError::decoding("Failed to cast to AppSrc"))?;
        
        // Configure appsrc for H.264 stream
        let caps = create_h264_caps();
        appsrc.set_caps(Some(&caps));
        appsrc.set_property("format", gst::Format::Time);
        
        // Create h264parse element
        let h264parse = gst::ElementFactory::make("h264parse")
            .name("parse")
            .build()
            .map_err(|e| StreamError::decoding(format!("Failed to create h264parse: {}", e)))?;
        
        // Try hardware decoder (platform-specific)
        let decoder = Self::create_hardware_decoder()?;
        
        // Create videoconvert for format conversion
        let videoconvert = gst::ElementFactory::make("videoconvert")
            .name("convert")
            .build()
            .map_err(|e| StreamError::decoding(format!("Failed to create videoconvert: {}", e)))?;
        
        // Create appsink for output
        let appsink = gst::ElementFactory::make("appsink")
            .name("sink")
            .build()
            .map_err(|e| StreamError::decoding(format!("Failed to create appsink: {}", e)))?;
        
        let appsink = appsink
            .dynamic_cast::<gst_app::AppSink>()
            .map_err(|_| StreamError::decoding("Failed to cast to AppSink"))?;
        
        // Configure appsink for I420 output
        let caps = create_i420_caps();
        appsink.set_caps(Some(&caps));
        appsink.set_property("emit-signals", false);
        appsink.set_property("sync", false);
        
        // Add elements to pipeline
        pipeline.add_many(&[
            appsrc.upcast_ref(),
            &h264parse,
            &decoder,
            &videoconvert,
            appsink.upcast_ref(),
        ])
        .map_err(|e| StreamError::decoding(format!("Failed to add elements: {}", e)))?;
        
        // Link elements
        gst::Element::link_many(&[
            appsrc.upcast_ref(),
            &h264parse,
            &decoder,
            &videoconvert,
            appsink.upcast_ref(),
        ])
        .map_err(|e| StreamError::decoding(format!("Failed to link elements: {}", e)))?;
        
        // Start pipeline
        pipeline.set_state(gst::State::Playing)
            .map_err(|e| StreamError::decoding(format!("Failed to start pipeline: {}", e)))?;
        
        Ok(DecoderBackend::Hardware {
            pipeline,
            appsrc,
            appsink,
        })
    }

    /// Create software decoder pipeline
    fn create_software_pipeline() -> StreamResult<Self> {
        let pipeline = gst::Pipeline::with_name("sw_decoder_pipeline");
        
        // Create appsrc for input data
        let appsrc = gst::ElementFactory::make("appsrc")
            .name("src")
            .build()
            .map_err(|e| StreamError::decoding(format!("Failed to create appsrc: {}", e)))?;
        
        let appsrc = appsrc
            .dynamic_cast::<gst_app::AppSrc>()
            .map_err(|_| StreamError::decoding("Failed to cast to AppSrc"))?;
        
        // Configure appsrc for H.264 stream
        let caps = create_h264_caps();
        appsrc.set_caps(Some(&caps));
        appsrc.set_property("format", gst::Format::Time);
        
        // Create h264parse element
        let h264parse = gst::ElementFactory::make("h264parse")
            .name("parse")
            .build()
            .map_err(|e| StreamError::decoding(format!("Failed to create h264parse: {}", e)))?;
        
        // Create software decoder (avdec_h264)
        let decoder = gst::ElementFactory::make("avdec_h264")
            .name("decoder")
            .build()
            .map_err(|e| StreamError::decoding(format!("Failed to create avdec_h264: {}", e)))?;
        
        // Create videoconvert for format conversion
        let videoconvert = gst::ElementFactory::make("videoconvert")
            .name("convert")
            .build()
            .map_err(|e| StreamError::decoding(format!("Failed to create videoconvert: {}", e)))?;
        
        // Create appsink for output
        let appsink = gst::ElementFactory::make("appsink")
            .name("sink")
            .build()
            .map_err(|e| StreamError::decoding(format!("Failed to create appsink: {}", e)))?;
        
        let appsink = appsink
            .dynamic_cast::<gst_app::AppSink>()
            .map_err(|_| StreamError::decoding("Failed to cast to AppSink"))?;
        
        // Configure appsink for I420 output
        let caps = create_i420_caps();
        appsink.set_caps(Some(&caps));
        appsink.set_property("emit-signals", false);
        appsink.set_property("sync", false);
        
        // Add elements to pipeline
        pipeline.add_many(&[
            appsrc.upcast_ref(),
            &h264parse,
            &decoder,
            &videoconvert,
            appsink.upcast_ref(),
        ])
        .map_err(|e| StreamError::decoding(format!("Failed to add elements: {}", e)))?;
        
        // Link elements
        gst::Element::link_many(&[
            appsrc.upcast_ref(),
            &h264parse,
            &decoder,
            &videoconvert,
            appsink.upcast_ref(),
        ])
        .map_err(|e| StreamError::decoding(format!("Failed to link elements: {}", e)))?;
        
        // Start pipeline
        pipeline.set_state(gst::State::Playing)
            .map_err(|e| StreamError::decoding(format!("Failed to start pipeline: {}", e)))?;
        
        Ok(DecoderBackend::Software {
            pipeline,
            appsrc,
            appsink,
        })
    }

    /// Create platform-specific hardware decoder
    fn create_hardware_decoder() -> StreamResult<gst::Element> {
        // Try NVDEC (NVIDIA)
        if let Ok(decoder) = gst::ElementFactory::make("nvh264dec")
            .name("decoder")
            .build()
        {
            return Ok(decoder);
        }
        
        // Try VAAPI (Intel/AMD on Linux)
        #[cfg(target_os = "linux")]
        if let Ok(decoder) = gst::ElementFactory::make("vaapih264dec")
            .name("decoder")
            .build()
        {
            return Ok(decoder);
        }
        
        // Try VideoToolbox (Apple)
        #[cfg(target_os = "macos")]
        if let Ok(decoder) = gst::ElementFactory::make("vtdec_h264")
            .name("decoder")
            .build()
        {
            return Ok(decoder);
        }
        
        // Try Media Foundation (Windows)
        #[cfg(target_os = "windows")]
        if let Ok(decoder) = gst::ElementFactory::make("mfh264dec")
            .name("decoder")
            .build()
        {
            return Ok(decoder);
        }
        
        Err(StreamError::unsupported("No hardware decoder available"))
    }

    /// Decode H.264 data
    fn decode(&mut self, data: &[u8]) -> StreamResult<VideoFrame> {
        let (appsrc, appsink) = match self {
            DecoderBackend::Hardware { appsrc, appsink, .. } => (appsrc, appsink),
            DecoderBackend::Software { appsrc, appsink, .. } => (appsrc, appsink),
        };
        
        // Create buffer from input data
        let buffer = gst::Buffer::from_slice(data.to_vec());
        
        // Push buffer to appsrc
        appsrc.push_buffer(buffer)
            .map_err(|e| StreamError::decoding(format!("Failed to push buffer: {:?}", e)))?;
        
        // Pull decoded sample from appsink
        let sample = appsink.pull_sample()
            .map_err(|e| StreamError::decoding(format!("Failed to pull sample: {:?}", e)))?;
        
        let buffer = sample.buffer()
            .ok_or_else(|| StreamError::decoding("No buffer in sample"))?;
        
        let caps = sample.caps()
            .ok_or_else(|| StreamError::decoding("No caps in sample"))?;
        
        // Extract video info from caps
        let video_info = gstreamer_video::VideoInfo::from_caps(caps)
            .map_err(|e| StreamError::decoding(format!("Failed to get video info: {}", e)))?;
        
        let width = video_info.width();
        let height = video_info.height();
        
        // Map buffer and copy data
        let map = buffer.map_readable()
            .map_err(|e| StreamError::decoding(format!("Failed to map buffer: {}", e)))?;
        
        let data = map.as_slice().to_vec();
        
        Ok(VideoFrame {
            data,
            width,
            height,
            format: PixelFormat::YUV420,
            timestamp: SystemTime::now(),
        })
    }
}

impl Drop for DecoderBackend {
    fn drop(&mut self) {
        let pipeline = match self {
            DecoderBackend::Hardware { pipeline, .. } => pipeline,
            DecoderBackend::Software { pipeline, .. } => pipeline,
        };
        
        let _ = pipeline.set_state(gst::State::Null);
    }
}

/// H.264 decoder with hardware acceleration
///
/// Requirements: 2.1, 2.2
pub struct H264Decoder {
    backend: DecoderBackend,
}

impl H264Decoder {
    /// Create a new H.264 decoder
    pub fn new(use_hardware: bool) -> StreamResult<Self> {
        let backend = DecoderBackend::new(use_hardware)?;
        
        Ok(Self {
            backend,
        })
    }

    /// Decode H.264 encoded data
    pub fn decode(&mut self, data: &[u8]) -> StreamResult<VideoFrame> {
        if data.is_empty() {
            return Err(StreamError::decoding("Empty input data"));
        }
        
        self.backend.decode(data)
    }

    /// Check if using hardware acceleration
    pub fn is_hardware_accelerated(&self) -> bool {
        matches!(self.backend, DecoderBackend::Hardware { .. })
    }
}
