//! Clipboard content processing and format conversion

use async_trait::async_trait;
use image::ImageFormat as ImgFormat;
use std::io::Cursor;
use crate::clipboard::{
    ClipboardContent, ClipboardResult, ClipboardError,
    TextContent, ImageContent, ImageFormat, TextFormat, TextEncoding
};

/// Text processor for handling various text formats and encodings
pub struct TextProcessor {
    max_text_size: usize,
}

impl TextProcessor {
    /// Create new text processor with default 1MB limit
    pub fn new() -> Self {
        Self {
            max_text_size: 1024 * 1024, // 1MB
        }
    }
    
    /// Create text processor with custom size limit
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            max_text_size: max_size,
        }
    }
    
    /// Process text content with full Unicode support
    pub fn process_text(&self, text: &str, format: TextFormat) -> ClipboardResult<TextContent> {
        // Validate text size
        let size = text.len();
        if size > self.max_text_size {
            return Err(ClipboardError::size(size, self.max_text_size));
        }
        
        // Ensure valid UTF-8
        if !text.is_empty() && !self.is_valid_utf8(text) {
            return Err(ClipboardError::content("Invalid UTF-8 text content"));
        }
        
        // Normalize line endings to \n for consistency
        let normalized_text = self.normalize_line_endings(text);
        
        Ok(TextContent {
            text: normalized_text.clone(),
            encoding: TextEncoding::Utf8,
            format,
            size: normalized_text.len(),
        })
    }
    
    /// Validate UTF-8 encoding
    fn is_valid_utf8(&self, text: &str) -> bool {
        std::str::from_utf8(text.as_bytes()).is_ok()
    }
    
    /// Normalize line endings to Unix-style \n
    fn normalize_line_endings(&self, text: &str) -> String {
        text.replace("\r\n", "\n").replace('\r', "\n")
    }
    
    /// Convert text to plain format by stripping formatting
    pub fn to_plain_text(&self, content: &TextContent) -> ClipboardResult<TextContent> {
        let plain_text = match content.format {
            TextFormat::Plain => content.text.clone(),
            TextFormat::Html => self.strip_html(&content.text)?,
            TextFormat::Rtf => self.strip_rtf(&content.text)?,
            TextFormat::Markdown => self.strip_markdown(&content.text)?,
        };
        
        self.process_text(&plain_text, TextFormat::Plain)
    }
    
    /// Strip HTML tags to get plain text
    fn strip_html(&self, html: &str) -> ClipboardResult<String> {
        // Simple HTML tag removal - in production, use a proper HTML parser
        let mut result = String::new();
        let mut in_tag = false;
        let mut in_script_or_style = false;
        let mut tag_name = String::new();
        
        for c in html.chars() {
            match c {
                '<' => {
                    in_tag = true;
                    tag_name.clear();
                }
                '>' => {
                    in_tag = false;
                    // Check if we're entering/exiting script or style tags
                    let tag_lower = tag_name.to_lowercase();
                    if tag_lower == "script" || tag_lower == "style" {
                        in_script_or_style = true;
                    } else if tag_lower == "/script" || tag_lower == "/style" {
                        in_script_or_style = false;
                    }
                }
                _ => {
                    if in_tag {
                        tag_name.push(c);
                    } else if !in_script_or_style {
                        result.push(c);
                    }
                }
            }
        }
        
        // Decode common HTML entities
        let result = result
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&nbsp;", " ");
        
        Ok(result)
    }
    
    /// Strip RTF formatting to get plain text
    fn strip_rtf(&self, rtf: &str) -> ClipboardResult<String> {
        // Basic RTF stripping - remove control words and groups
        let mut result = String::new();
        let mut in_control = false;
        let mut in_group: i32 = 0;
        let mut skip_next = false;
        
        for c in rtf.chars() {
            if skip_next {
                skip_next = false;
                continue;
            }
            
            match c {
                '\\' => {
                    in_control = true;
                }
                '{' => {
                    in_group += 1;
                }
                '}' => {
                    in_group = in_group.saturating_sub(1);
                }
                ' ' | '\n' | '\r' if in_control => {
                    in_control = false;
                }
                _ => {
                    if !in_control && in_group == 0 {
                        result.push(c);
                    } else if in_control {
                        // Skip control word characters
                    }
                }
            }
        }
        
        Ok(result.trim().to_string())
    }
    
    /// Strip Markdown formatting to get plain text
    fn strip_markdown(&self, markdown: &str) -> ClipboardResult<String> {
        // Basic markdown stripping
        let mut result = String::new();
        let lines: Vec<&str> = markdown.lines().collect();
        
        for line in lines {
            let trimmed = line.trim();
            
            // Skip headers, but keep the text
            let text = if trimmed.starts_with('#') {
                trimmed.trim_start_matches('#').trim()
            } else {
                trimmed
            };
            
            // Remove bold/italic markers
            let text = text
                .replace("**", "")
                .replace("__", "")
                .replace('*', "")
                .replace('_', "");
            
            // Remove links but keep text [text](url) -> text
            let text = self.extract_markdown_link_text(&text);
            
            if !text.is_empty() {
                result.push_str(&text);
                result.push('\n');
            }
        }
        
        Ok(result.trim().to_string())
    }
    
    /// Extract text from markdown links
    fn extract_markdown_link_text(&self, text: &str) -> String {
        let mut result = String::new();
        let mut chars = text.chars().peekable();
        
        while let Some(c) = chars.next() {
            if c == '[' {
                // Extract link text
                let mut link_text = String::new();
                while let Some(ch) = chars.next() {
                    if ch == ']' {
                        // Skip the URL part
                        if chars.peek() == Some(&'(') {
                            chars.next(); // consume '('
                            while let Some(ch) = chars.next() {
                                if ch == ')' {
                                    break;
                                }
                            }
                        }
                        result.push_str(&link_text);
                        break;
                    }
                    link_text.push(ch);
                }
            } else {
                result.push(c);
            }
        }
        
        result
    }
    
    /// Validate text content structure and encoding
    pub fn validate_text(&self, content: &TextContent) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        
        // Check size limits
        if content.size > self.max_text_size {
            errors.push(format!(
                "Text content size {} exceeds limit {}",
                content.size, self.max_text_size
            ));
        }
        
        // Validate UTF-8 encoding
        if !self.is_valid_utf8(&content.text) {
            errors.push("Text contains invalid UTF-8 sequences".to_string());
        }
        
        // Check encoding consistency
        if content.encoding != TextEncoding::Utf8 {
            warnings.push(format!(
                "Text encoding {:?} will be converted to UTF-8",
                content.encoding
            ));
        }
        
        // Check for extremely long lines
        let max_line_length = content.text.lines().map(|l| l.len()).max().unwrap_or(0);
        if max_line_length > 10000 {
            warnings.push(format!(
                "Text contains very long lines (max: {} chars) that may cause display issues",
                max_line_length
            ));
        }
        
        // Check for null bytes
        if content.text.contains('\0') {
            warnings.push("Text contains null bytes which may cause issues".to_string());
        }
        
        // Format-specific validation
        match content.format {
            TextFormat::Html => {
                if !content.text.contains('<') && !content.text.contains('>') {
                    warnings.push("HTML format specified but no HTML tags detected".to_string());
                }
            }
            TextFormat::Rtf => {
                if !content.text.starts_with("{\\rtf") {
                    warnings.push("RTF format specified but content doesn't start with RTF header".to_string());
                }
            }
            _ => {}
        }
        
        ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            size_bytes: content.size,
        }
    }
    
    /// Preserve text formatting during conversion
    pub fn preserve_format(&self, content: &TextContent, target_format: TextFormat) -> ClipboardResult<TextContent> {
        // If formats match, return as-is
        if content.format == target_format {
            return Ok(content.clone());
        }
        
        // Convert between formats
        let converted_text = match (&content.format, &target_format) {
            (_, TextFormat::Plain) => {
                // Any format to plain text
                self.to_plain_text(content)?.text
            }
            (TextFormat::Plain, TextFormat::Html) => {
                // Plain text to HTML
                self.plain_to_html(&content.text)
            }
            (TextFormat::Plain, TextFormat::Markdown) => {
                // Plain text to Markdown (minimal conversion)
                content.text.clone()
            }
            (TextFormat::Markdown, TextFormat::Html) => {
                // Markdown to HTML (basic conversion)
                self.markdown_to_html(&content.text)
            }
            _ => {
                // For other conversions, go through plain text
                let plain = self.to_plain_text(content)?;
                return self.preserve_format(&plain, target_format);
            }
        };
        
        self.process_text(&converted_text, target_format)
    }
    
    /// Convert plain text to HTML
    fn plain_to_html(&self, text: &str) -> String {
        let escaped = text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\n', "<br>\n");
        
        format!("<!DOCTYPE html>\n<html>\n<body>\n{}\n</body>\n</html>", escaped)
    }
    
    /// Convert Markdown to HTML (basic)
    fn markdown_to_html(&self, markdown: &str) -> String {
        let mut html = String::from("<!DOCTYPE html>\n<html>\n<body>\n");
        
        for line in markdown.lines() {
            let trimmed = line.trim();
            
            if trimmed.is_empty() {
                html.push_str("<br>\n");
                continue;
            }
            
            // Headers
            if let Some(stripped) = trimmed.strip_prefix("# ") {
                html.push_str(&format!("<h1>{}</h1>\n", stripped));
            } else if let Some(stripped) = trimmed.strip_prefix("## ") {
                html.push_str(&format!("<h2>{}</h2>\n", stripped));
            } else if let Some(stripped) = trimmed.strip_prefix("### ") {
                html.push_str(&format!("<h3>{}</h3>\n", stripped));
            } else {
                // Regular paragraph
                html.push_str(&format!("<p>{}</p>\n", trimmed));
            }
        }
        
        html.push_str("</body>\n</html>");
        html
    }
}

impl Default for TextProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// Image processor for handling various image formats and compression
pub struct ImageProcessor {
    compression_threshold: usize,
    jpeg_quality: u8,
    max_image_size: usize,
}

impl ImageProcessor {
    /// Create new image processor with default settings
    pub fn new() -> Self {
        Self {
            compression_threshold: 5 * 1024 * 1024, // 5MB
            jpeg_quality: 85,
            max_image_size: 50 * 1024 * 1024, // 50MB max
        }
    }
    
    /// Create image processor with custom settings
    pub fn with_settings(threshold: usize, quality: u8, max_size: usize) -> Self {
        Self {
            compression_threshold: threshold,
            jpeg_quality: quality,
            max_image_size: max_size,
        }
    }
    
    /// Process image content with format detection and validation
    pub fn process_image(&self, data: &[u8], format: ImageFormat) -> ClipboardResult<ImageContent> {
        // Validate size
        if data.len() > self.max_image_size {
            return Err(ClipboardError::size(data.len(), self.max_image_size));
        }
        
        // Load and validate image
        let img = image::load_from_memory(data)
            .map_err(|e| ClipboardError::content(format!("Failed to load image: {}", e)))?;
        
        let width = img.width();
        let height = img.height();
        
        // Validate dimensions
        if width == 0 || height == 0 {
            return Err(ClipboardError::content("Image has invalid dimensions"));
        }
        
        Ok(ImageContent {
            data: data.to_vec(),
            format,
            width,
            height,
            compressed: false,
        })
    }
    
    /// Compress image if it exceeds the threshold
    pub fn compress_if_needed(&self, image: &ImageContent) -> ClipboardResult<ImageContent> {
        if image.data.len() <= self.compression_threshold || image.compressed {
            return Ok(image.clone());
        }
        
        self.compress_image(image)
    }
    
    /// Compress image to JPEG format with quality preservation
    pub fn compress_image(&self, image: &ImageContent) -> ClipboardResult<ImageContent> {
        // Load image from bytes
        let img = image::load_from_memory(&image.data)
            .map_err(|e| ClipboardError::content(format!("Failed to load image for compression: {}", e)))?;
        
        // Encode to JPEG with specified quality
        let mut compressed_data = Vec::new();
        let mut cursor = Cursor::new(&mut compressed_data);
        
        // Use JPEG encoder with quality setting
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, self.jpeg_quality);
        img.write_with_encoder(encoder)
            .map_err(|e| ClipboardError::content(format!("Failed to compress image: {}", e)))?;
        
        Ok(ImageContent {
            data: compressed_data,
            format: ImageFormat::Jpeg,
            width: image.width,
            height: image.height,
            compressed: true,
        })
    }
    
    /// Convert image between formats while preserving quality
    pub fn convert_format(&self, image: &ImageContent, target_format: ImageFormat) -> ClipboardResult<ImageContent> {
        // If formats match, return as-is
        if image.format == target_format {
            return Ok(image.clone());
        }
        
        // Load image
        let img = image::load_from_memory(&image.data)
            .map_err(|e| ClipboardError::content(format!("Failed to load image for conversion: {}", e)))?;
        
        // Convert to target format
        let mut converted_data = Vec::new();
        let mut cursor = Cursor::new(&mut converted_data);
        
        let img_format = self.to_image_format(&target_format);
        
        // Use appropriate encoder based on format
        let is_jpeg = matches!(target_format, ImageFormat::Jpeg);
        match &target_format {
            ImageFormat::Jpeg => {
                let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, self.jpeg_quality);
                img.write_with_encoder(encoder)
                    .map_err(|e| ClipboardError::content(format!("Failed to encode JPEG: {}", e)))?;
            }
            ImageFormat::Png => {
                img.write_to(&mut cursor, ImgFormat::Png)
                    .map_err(|e| ClipboardError::content(format!("Failed to encode PNG: {}", e)))?;
            }
            _ => {
                img.write_to(&mut cursor, img_format)
                    .map_err(|e| ClipboardError::content(format!("Failed to encode image: {}", e)))?;
            }
        }
        
        Ok(ImageContent {
            data: converted_data,
            format: target_format,
            width: image.width,
            height: image.height,
            compressed: is_jpeg,
        })
    }
    
    /// Detect image format from data
    pub fn detect_format(&self, data: &[u8]) -> ClipboardResult<ImageFormat> {
        let format = image::guess_format(data)
            .map_err(|e| ClipboardError::content(format!("Failed to detect image format: {}", e)))?;
        
        Ok(self.from_image_format(format))
    }
    
    /// Validate image content
    pub fn validate_image(&self, image: &ImageContent) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        
        // Check size limits
        if image.data.len() > self.max_image_size {
            errors.push(format!(
                "Image size {} exceeds maximum limit {}",
                image.data.len(), self.max_image_size
            ));
        }
        
        // Check dimensions
        if image.width == 0 || image.height == 0 {
            errors.push("Image has invalid dimensions (width or height is 0)".to_string());
        }
        
        // Check for very large dimensions
        if image.width > 10000 || image.height > 10000 {
            warnings.push(format!(
                "Image has very large dimensions ({}x{}) that may cause performance issues",
                image.width, image.height
            ));
        }
        
        // Warn about uncompressed large images
        if image.data.len() > self.compression_threshold && !image.compressed {
            warnings.push(format!(
                "Large image ({} bytes) should be compressed for better performance",
                image.data.len()
            ));
        }
        
        // Validate image data can be loaded
        if let Err(e) = image::load_from_memory(&image.data) {
            errors.push(format!("Image data is corrupted or invalid: {}", e));
        }
        
        // Format-specific validation
        match image.format {
            ImageFormat::Png | ImageFormat::Jpeg => {
                // These are well-supported formats
            }
            ImageFormat::Bmp | ImageFormat::Gif | ImageFormat::Tiff => {
                warnings.push(format!(
                    "Image format {:?} may not be supported on all platforms",
                    image.format
                ));
            }
        }
        
        ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            size_bytes: image.data.len(),
        }
    }
    
    /// Get image metadata without loading full image
    pub fn get_metadata(&self, data: &[u8]) -> ClipboardResult<(u32, u32, ImageFormat)> {
        let format = self.detect_format(data)?;
        let img = image::load_from_memory(data)
            .map_err(|e| ClipboardError::content(format!("Failed to load image metadata: {}", e)))?;
        
        Ok((img.width(), img.height(), format))
    }
    
    /// Convert image format enum to image crate format
    fn to_image_format(&self, format: &ImageFormat) -> ImgFormat {
        match format {
            ImageFormat::Png => ImgFormat::Png,
            ImageFormat::Jpeg => ImgFormat::Jpeg,
            ImageFormat::Bmp => ImgFormat::Bmp,
            ImageFormat::Gif => ImgFormat::Gif,
            ImageFormat::Tiff => ImgFormat::Tiff,
        }
    }
    
    /// Convert image crate format to our format enum
    fn from_image_format(&self, format: ImgFormat) -> ImageFormat {
        match format {
            ImgFormat::Png => ImageFormat::Png,
            ImgFormat::Jpeg => ImageFormat::Jpeg,
            ImgFormat::Bmp => ImageFormat::Bmp,
            ImgFormat::Gif => ImageFormat::Gif,
            ImgFormat::Tiff => ImageFormat::Tiff,
            _ => ImageFormat::Png, // Default fallback
        }
    }
}

impl Default for ImageProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// Platform-specific clipboard format information
#[derive(Debug, Clone, PartialEq)]
pub enum PlatformFormat {
    /// Windows clipboard formats
    Windows(WindowsFormat),
    /// macOS pasteboard types
    MacOS(MacOSFormat),
    /// Linux/X11 MIME types
    Linux(String),
    /// Generic cross-platform format
    Generic(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum WindowsFormat {
    Text,
    UnicodeText,
    Html,
    Rtf,
    Bitmap,
    Dib,
    Png,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum MacOSFormat {
    String,
    Rtf,
    Html,
    Png,
    Tiff,
    Pdf,
    Custom(String),
}

/// Cross-platform format converter
pub struct FormatConverter {
    text_processor: TextProcessor,
    image_processor: ImageProcessor,
}

impl FormatConverter {
    /// Create new format converter
    pub fn new() -> Self {
        Self {
            text_processor: TextProcessor::new(),
            image_processor: ImageProcessor::new(),
        }
    }
    
    /// Convert clipboard content to platform-specific format
    pub fn to_platform_format(
        &self,
        content: &ClipboardContent,
        platform: &str,
    ) -> ClipboardResult<Vec<(PlatformFormat, Vec<u8>)>> {
        match content {
            ClipboardContent::Text(text) => self.text_to_platform_formats(text, platform),
            ClipboardContent::Image(image) => self.image_to_platform_formats(image, platform),
            ClipboardContent::Files(files) => self.files_to_platform_formats(files, platform),
            ClipboardContent::Custom { mime_type, data } => {
                Ok(vec![(PlatformFormat::Generic(mime_type.clone()), data.clone())])
            }
        }
    }
    
    /// Convert platform-specific format to clipboard content
    pub fn from_platform_format(
        &self,
        format: PlatformFormat,
        data: Vec<u8>,
    ) -> ClipboardResult<ClipboardContent> {
        match format {
            PlatformFormat::Windows(win_format) => self.from_windows_format(win_format, data),
            PlatformFormat::MacOS(mac_format) => self.from_macos_format(mac_format, data),
            PlatformFormat::Linux(mime_type) => self.from_linux_format(&mime_type, data),
            PlatformFormat::Generic(mime_type) => self.from_generic_format(&mime_type, data),
        }
    }
    
    /// Convert text content to platform-specific formats
    fn text_to_platform_formats(
        &self,
        text: &TextContent,
        platform: &str,
    ) -> ClipboardResult<Vec<(PlatformFormat, Vec<u8>)>> {
        let mut formats = Vec::new();
        
        match platform {
            "windows" => {
                // Windows prefers Unicode text
                formats.push((
                    PlatformFormat::Windows(WindowsFormat::UnicodeText),
                    self.encode_utf16(&text.text),
                ));
                
                // Also provide plain text
                formats.push((
                    PlatformFormat::Windows(WindowsFormat::Text),
                    text.text.as_bytes().to_vec(),
                ));
                
                // Add format-specific versions
                match text.format {
                    TextFormat::Html => {
                        formats.push((
                            PlatformFormat::Windows(WindowsFormat::Html),
                            self.encode_windows_html(&text.text),
                        ));
                    }
                    TextFormat::Rtf => {
                        formats.push((
                            PlatformFormat::Windows(WindowsFormat::Rtf),
                            text.text.as_bytes().to_vec(),
                        ));
                    }
                    _ => {}
                }
            }
            "macos" => {
                // macOS uses NSString for text
                formats.push((
                    PlatformFormat::MacOS(MacOSFormat::String),
                    text.text.as_bytes().to_vec(),
                ));
                
                // Add format-specific versions
                match text.format {
                    TextFormat::Html => {
                        formats.push((
                            PlatformFormat::MacOS(MacOSFormat::Html),
                            text.text.as_bytes().to_vec(),
                        ));
                    }
                    TextFormat::Rtf => {
                        formats.push((
                            PlatformFormat::MacOS(MacOSFormat::Rtf),
                            text.text.as_bytes().to_vec(),
                        ));
                    }
                    _ => {}
                }
            }
            "linux" => {
                // Linux uses MIME types
                formats.push((
                    PlatformFormat::Linux("text/plain;charset=utf-8".to_string()),
                    text.text.as_bytes().to_vec(),
                ));
                
                // Add format-specific versions
                match text.format {
                    TextFormat::Html => {
                        formats.push((
                            PlatformFormat::Linux("text/html".to_string()),
                            text.text.as_bytes().to_vec(),
                        ));
                    }
                    TextFormat::Rtf => {
                        formats.push((
                            PlatformFormat::Linux("text/rtf".to_string()),
                            text.text.as_bytes().to_vec(),
                        ));
                    }
                    _ => {}
                }
            }
            _ => {
                // Generic platform
                formats.push((
                    PlatformFormat::Generic("text/plain".to_string()),
                    text.text.as_bytes().to_vec(),
                ));
            }
        }
        
        Ok(formats)
    }
    
    /// Convert image content to platform-specific formats
    fn image_to_platform_formats(
        &self,
        image: &ImageContent,
        platform: &str,
    ) -> ClipboardResult<Vec<(PlatformFormat, Vec<u8>)>> {
        let mut formats = Vec::new();
        
        match platform {
            "windows" => {
                // Windows prefers PNG for clipboard
                let png_image = if image.format != ImageFormat::Png {
                    self.image_processor.convert_format(image, ImageFormat::Png)?
                } else {
                    image.clone()
                };
                
                formats.push((
                    PlatformFormat::Windows(WindowsFormat::Png),
                    png_image.data,
                ));
            }
            "macos" => {
                // macOS supports PNG and TIFF
                let png_image = if image.format != ImageFormat::Png {
                    self.image_processor.convert_format(image, ImageFormat::Png)?
                } else {
                    image.clone()
                };
                
                formats.push((
                    PlatformFormat::MacOS(MacOSFormat::Png),
                    png_image.data,
                ));
            }
            "linux" => {
                // Linux uses MIME types
                let mime_type = match image.format {
                    ImageFormat::Png => "image/png",
                    ImageFormat::Jpeg => "image/jpeg",
                    ImageFormat::Bmp => "image/bmp",
                    ImageFormat::Gif => "image/gif",
                    ImageFormat::Tiff => "image/tiff",
                };
                
                formats.push((
                    PlatformFormat::Linux(mime_type.to_string()),
                    image.data.clone(),
                ));
            }
            _ => {
                // Generic platform
                formats.push((
                    PlatformFormat::Generic("image/png".to_string()),
                    image.data.clone(),
                ));
            }
        }
        
        Ok(formats)
    }
    
    /// Convert file list to platform-specific formats
    fn files_to_platform_formats(
        &self,
        files: &[String],
        platform: &str,
    ) -> ClipboardResult<Vec<(PlatformFormat, Vec<u8>)>> {
        let file_list = files.join("\n");
        
        let format = match platform {
            "windows" => PlatformFormat::Windows(WindowsFormat::Custom("FileNameW".to_string())),
            "macos" => PlatformFormat::MacOS(MacOSFormat::Custom("public.file-url".to_string())),
            "linux" => PlatformFormat::Linux("text/uri-list".to_string()),
            _ => PlatformFormat::Generic("text/uri-list".to_string()),
        };
        
        Ok(vec![(format, file_list.as_bytes().to_vec())])
    }
    
    /// Convert from Windows clipboard format
    fn from_windows_format(
        &self,
        format: WindowsFormat,
        data: Vec<u8>,
    ) -> ClipboardResult<ClipboardContent> {
        match format {
            WindowsFormat::Text | WindowsFormat::UnicodeText => {
                let text = if format == WindowsFormat::UnicodeText {
                    self.decode_utf16(&data)?
                } else {
                    String::from_utf8(data)
                        .map_err(|e| ClipboardError::content(format!("Invalid UTF-8: {}", e)))?
                };
                
                let text_content = self.text_processor.process_text(&text, TextFormat::Plain)?;
                Ok(ClipboardContent::Text(text_content))
            }
            WindowsFormat::Html => {
                let html = String::from_utf8(data)
                    .map_err(|e| ClipboardError::content(format!("Invalid UTF-8 HTML: {}", e)))?;
                let text_content = self.text_processor.process_text(&html, TextFormat::Html)?;
                Ok(ClipboardContent::Text(text_content))
            }
            WindowsFormat::Rtf => {
                let rtf = String::from_utf8(data)
                    .map_err(|e| ClipboardError::content(format!("Invalid UTF-8 RTF: {}", e)))?;
                let text_content = self.text_processor.process_text(&rtf, TextFormat::Rtf)?;
                Ok(ClipboardContent::Text(text_content))
            }
            WindowsFormat::Png | WindowsFormat::Bitmap | WindowsFormat::Dib => {
                let format = self.image_processor.detect_format(&data)?;
                let image_content = self.image_processor.process_image(&data, format)?;
                Ok(ClipboardContent::Image(image_content))
            }
            WindowsFormat::Custom(name) => {
                Ok(ClipboardContent::Custom {
                    mime_type: format!("windows/{}", name),
                    data,
                })
            }
        }
    }
    
    /// Convert from macOS clipboard format
    fn from_macos_format(
        &self,
        format: MacOSFormat,
        data: Vec<u8>,
    ) -> ClipboardResult<ClipboardContent> {
        match format {
            MacOSFormat::String => {
                let text = String::from_utf8(data)
                    .map_err(|e| ClipboardError::content(format!("Invalid UTF-8: {}", e)))?;
                let text_content = self.text_processor.process_text(&text, TextFormat::Plain)?;
                Ok(ClipboardContent::Text(text_content))
            }
            MacOSFormat::Html => {
                let html = String::from_utf8(data)
                    .map_err(|e| ClipboardError::content(format!("Invalid UTF-8 HTML: {}", e)))?;
                let text_content = self.text_processor.process_text(&html, TextFormat::Html)?;
                Ok(ClipboardContent::Text(text_content))
            }
            MacOSFormat::Rtf => {
                let rtf = String::from_utf8(data)
                    .map_err(|e| ClipboardError::content(format!("Invalid UTF-8 RTF: {}", e)))?;
                let text_content = self.text_processor.process_text(&rtf, TextFormat::Rtf)?;
                Ok(ClipboardContent::Text(text_content))
            }
            MacOSFormat::Png | MacOSFormat::Tiff | MacOSFormat::Pdf => {
                let format = self.image_processor.detect_format(&data)?;
                let image_content = self.image_processor.process_image(&data, format)?;
                Ok(ClipboardContent::Image(image_content))
            }
            MacOSFormat::Custom(name) => {
                Ok(ClipboardContent::Custom {
                    mime_type: format!("macos/{}", name),
                    data,
                })
            }
        }
    }
    
    /// Convert from Linux clipboard format (MIME type)
    fn from_linux_format(&self, mime_type: &str, data: Vec<u8>) -> ClipboardResult<ClipboardContent> {
        if mime_type.starts_with("text/") {
            let text = String::from_utf8(data)
                .map_err(|e| ClipboardError::content(format!("Invalid UTF-8: {}", e)))?;
            
            let format = if mime_type.contains("html") {
                TextFormat::Html
            } else if mime_type.contains("rtf") {
                TextFormat::Rtf
            } else {
                TextFormat::Plain
            };
            
            let text_content = self.text_processor.process_text(&text, format)?;
            Ok(ClipboardContent::Text(text_content))
        } else if mime_type.starts_with("image/") {
            let format = self.image_processor.detect_format(&data)?;
            let image_content = self.image_processor.process_image(&data, format)?;
            Ok(ClipboardContent::Image(image_content))
        } else {
            Ok(ClipboardContent::Custom {
                mime_type: mime_type.to_string(),
                data,
            })
        }
    }
    
    /// Convert from generic format
    fn from_generic_format(&self, mime_type: &str, data: Vec<u8>) -> ClipboardResult<ClipboardContent> {
        self.from_linux_format(mime_type, data)
    }
    
    /// Encode string as UTF-16 for Windows
    fn encode_utf16(&self, text: &str) -> Vec<u8> {
        let utf16: Vec<u16> = text.encode_utf16().collect();
        let mut bytes = Vec::with_capacity(utf16.len() * 2);
        for word in utf16 {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes
    }
    
    /// Decode UTF-16 from Windows
    fn decode_utf16(&self, data: &[u8]) -> ClipboardResult<String> {
        if data.len() % 2 != 0 {
            return Err(ClipboardError::content("Invalid UTF-16 data length"));
        }
        
        let mut utf16 = Vec::with_capacity(data.len() / 2);
        for chunk in data.chunks_exact(2) {
            utf16.push(u16::from_le_bytes([chunk[0], chunk[1]]));
        }
        
        String::from_utf16(&utf16)
            .map_err(|e| ClipboardError::content(format!("Invalid UTF-16: {}", e)))
    }
    
    /// Encode HTML for Windows clipboard (with header)
    fn encode_windows_html(&self, html: &str) -> Vec<u8> {
        // Windows HTML clipboard format requires specific header
        let header = "Version:0.9\r\nStartHTML:0000000000\r\nEndHTML:0000000000\r\n";
        let full_html = format!("{}{}", header, html);
        full_html.as_bytes().to_vec()
    }
    
    /// Detect common format from content
    pub fn detect_content_format(&self, data: &[u8]) -> ClipboardResult<String> {
        // Try to detect if it's text
        if let Ok(text) = std::str::from_utf8(data) {
            if text.trim_start().starts_with("<!DOCTYPE") || text.trim_start().starts_with("<html") {
                return Ok("text/html".to_string());
            } else if text.starts_with("{\\rtf") {
                return Ok("text/rtf".to_string());
            } else {
                return Ok("text/plain".to_string());
            }
        }
        
        // Try to detect image format
        if let Ok(format) = self.image_processor.detect_format(data) {
            return Ok(match format {
                ImageFormat::Png => "image/png",
                ImageFormat::Jpeg => "image/jpeg",
                ImageFormat::Bmp => "image/bmp",
                ImageFormat::Gif => "image/gif",
                ImageFormat::Tiff => "image/tiff",
            }.to_string());
        }
        
        Ok("application/octet-stream".to_string())
    }
    
    /// Validate content integrity
    pub fn validate_integrity(&self, content: &ClipboardContent) -> ClipboardResult<bool> {
        match content {
            ClipboardContent::Text(text) => {
                let validation = self.text_processor.validate_text(text);
                Ok(validation.is_valid)
            }
            ClipboardContent::Image(image) => {
                let validation = self.image_processor.validate_image(image);
                Ok(validation.is_valid)
            }
            ClipboardContent::Files(_) | ClipboardContent::Custom { .. } => {
                // Basic validation - just check if data exists
                Ok(true)
            }
        }
    }
}

impl Default for FormatConverter {
    fn default() -> Self {
        Self::new()
    }
}

/// Processed content ready for transmission
#[derive(Debug, Clone)]
pub struct ProcessedContent {
    pub content: ClipboardContent,
    pub compressed: bool,
    pub original_size: usize,
    pub processed_size: usize,
}

/// Content validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub size_bytes: usize,
}

/// Content processor trait
#[async_trait]
pub trait ContentProcessor: Send + Sync {
    /// Process content for outgoing transmission
    async fn process_outgoing_content(&self, content: ClipboardContent) -> ClipboardResult<ProcessedContent>;
    
    /// Process content received from peers
    async fn process_incoming_content(&self, content: ProcessedContent) -> ClipboardResult<ClipboardContent>;
    
    /// Compress image content
    async fn compress_image(&self, image: ImageContent) -> ClipboardResult<ImageContent>;
    
    /// Validate clipboard content
    async fn validate_content(&self, content: &ClipboardContent) -> ClipboardResult<ValidationResult>;
}

/// Default content processor implementation
pub struct DefaultContentProcessor {
    max_content_size: usize,
    text_processor: TextProcessor,
    image_processor: ImageProcessor,
    format_converter: FormatConverter,
}

impl DefaultContentProcessor {
    /// Create new content processor
    pub fn new() -> Self {
        Self {
            max_content_size: 1024 * 1024, // 1MB
            text_processor: TextProcessor::new(),
            image_processor: ImageProcessor::new(),
            format_converter: FormatConverter::new(),
        }
    }
    
    /// Create content processor with custom settings
    pub fn with_settings(max_size: usize, compression_threshold: usize, quality: u8) -> Self {
        Self {
            max_content_size: max_size,
            text_processor: TextProcessor::with_max_size(max_size),
            image_processor: ImageProcessor::with_settings(compression_threshold, quality, max_size),
            format_converter: FormatConverter::new(),
        }
    }
    
    /// Convert content to platform-specific format
    pub fn to_platform_format(
        &self,
        content: &ClipboardContent,
        platform: &str,
    ) -> ClipboardResult<Vec<(PlatformFormat, Vec<u8>)>> {
        self.format_converter.to_platform_format(content, platform)
    }
    
    /// Convert from platform-specific format
    pub fn from_platform_format(
        &self,
        format: PlatformFormat,
        data: Vec<u8>,
    ) -> ClipboardResult<ClipboardContent> {
        self.format_converter.from_platform_format(format, data)
    }
    
    /// Detect content format from raw data
    pub fn detect_format(&self, data: &[u8]) -> ClipboardResult<String> {
        self.format_converter.detect_content_format(data)
    }
    
    /// Validate content integrity
    pub fn validate_integrity(&self, content: &ClipboardContent) -> ClipboardResult<bool> {
        self.format_converter.validate_integrity(content)
    }
    
    /// Validate text content using TextProcessor
    fn validate_text_content(&self, text_content: &TextContent) -> ValidationResult {
        self.text_processor.validate_text(text_content)
    }
    
    /// Validate image content using ImageProcessor
    fn validate_image_content(&self, image_content: &ImageContent) -> ValidationResult {
        self.image_processor.validate_image(image_content)
    }
}

#[async_trait]
impl ContentProcessor for DefaultContentProcessor {
    async fn process_outgoing_content(&self, content: ClipboardContent) -> ClipboardResult<ProcessedContent> {
        let original_size = content.size();
        let mut processed_content = content.clone();
        let mut compressed = false;
        
        // Validate content first
        let validation = self.validate_content(&content).await?;
        if !validation.is_valid {
            return Err(ClipboardError::content(format!(
                "Content validation failed: {}",
                validation.errors.join(", ")
            )));
        }
        
        // Process based on content type
        match &content {
            ClipboardContent::Image(image_content) => {
                // Compress large images using ImageProcessor
                let processed_image = self.image_processor.compress_if_needed(image_content)?;
                if processed_image.compressed && !image_content.compressed {
                    compressed = true;
                }
                processed_content = ClipboardContent::Image(processed_image);
            }
            ClipboardContent::Text(text_content) => {
                // Process and normalize text content using TextProcessor
                let processed_text = self.text_processor.process_text(
                    &text_content.text,
                    text_content.format.clone()
                )?;
                processed_content = ClipboardContent::Text(processed_text);
            }
            _ => {
                // Other content types pass through unchanged
            }
        }
        
        let processed_size = processed_content.size();
        
        Ok(ProcessedContent {
            content: processed_content,
            compressed,
            original_size,
            processed_size,
        })
    }
    
    async fn process_incoming_content(&self, processed: ProcessedContent) -> ClipboardResult<ClipboardContent> {
        // Validate incoming content
        let validation = self.validate_content(&processed.content).await?;
        if !validation.is_valid {
            return Err(ClipboardError::content(format!(
                "Incoming content validation failed: {}",
                validation.errors.join(", ")
            )));
        }
        
        // Content is already processed and ready to use
        Ok(processed.content)
    }
    
    async fn compress_image(&self, image: ImageContent) -> ClipboardResult<ImageContent> {
        self.image_processor.compress_image(&image)
    }
    
    async fn validate_content(&self, content: &ClipboardContent) -> ClipboardResult<ValidationResult> {
        let result = match content {
            ClipboardContent::Text(text_content) => {
                self.validate_text_content(text_content)
            }
            ClipboardContent::Image(image_content) => {
                self.validate_image_content(image_content)
            }
            ClipboardContent::Files(files) => {
                let total_size = files.iter().map(|f| f.len()).sum();
                ValidationResult {
                    is_valid: total_size <= self.max_content_size,
                    errors: if total_size > self.max_content_size {
                        vec![format!("File list size {} exceeds limit {}", total_size, self.max_content_size)]
                    } else {
                        vec![]
                    },
                    warnings: vec![],
                    size_bytes: total_size,
                }
            }
            ClipboardContent::Custom { data, mime_type } => {
                ValidationResult {
                    is_valid: data.len() <= self.max_content_size,
                    errors: if data.len() > self.max_content_size {
                        vec![format!("Custom content size {} exceeds limit {}", data.len(), self.max_content_size)]
                    } else {
                        vec![]
                    },
                    warnings: vec![format!("Custom content type '{}' may not be supported on all platforms", mime_type)],
                    size_bytes: data.len(),
                }
            }
        };
        
        Ok(result)
    }
}

impl Default for DefaultContentProcessor {
    fn default() -> Self {
        Self::new()
    }
}