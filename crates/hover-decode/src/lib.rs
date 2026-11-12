use gif::DisposalMethod;
use hover_core::CancellationToken;
use hover_core::cache::ByteSized;
use std::io::{Cursor, Read};
use std::path::Path;
use thiserror::Error;
pub mod local_io;

#[cfg(all(windows, feature = "wic"))]
pub mod wic;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Gif,
    Bmp,
    Tiff,
    Ico,
    WebP,
    Avif,
    Heif,
    Svg,
    JpegXl,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub struct DecodeLimits {
    pub max_local_file_size_bytes: u64,
    pub max_decoded_pixels: u64,
    pub max_output_dimension: u32,
}

impl Default for DecodeLimits {
    fn default() -> Self {
        Self {
            max_local_file_size_bytes: 512 * 1024 * 1024,
            max_decoded_pixels: 200_000_000,
            max_output_dimension: 32_768,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub pixels_bgra: Vec<u8>,
    pub format: ImageFormat,
}

#[derive(Debug, Clone)]
pub struct DecodedFrame {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub pixels_bgra: Vec<u8>,
    pub delay_ms: u32,
}

impl ByteSized for DecodedFrame {
    fn byte_size(&self) -> usize {
        self.pixels_bgra.len()
    }
}

#[derive(Debug, Clone)]
pub struct DecodedAnimation {
    pub frames: Vec<DecodedFrame>,
    pub format: ImageFormat,
}

impl ByteSized for DecodedAnimation {
    fn byte_size(&self) -> usize {
        self.frames.iter().map(ByteSized::byte_size).sum()
    }
}

impl ByteSized for DecodedImage {
    fn byte_size(&self) -> usize {
        self.pixels_bgra.len()
    }
}

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("source is too large: {actual} bytes exceeds {limit} bytes")]
    FileTooLarge { actual: u64, limit: u64 },
    #[error("decoded image is too large: {pixels} pixels exceeds {limit} pixels")]
    TooManyPixels { pixels: u64, limit: u64 },
    #[error("unsupported image format")]
    UnsupportedFormat,
    #[error("source is not readable")]
    NotReadable,
    #[error("{0}")]
    Source(String),
    #[error(
        "The file changed while being read. Hover discarded this preview; hover again to reload."
    )]
    SourceChanged,
    #[error("Preview budget exceeded: {0}")]
    Budget(&'static str),
    #[error("decode was cancelled")]
    Cancelled,
    #[error("Windows Imaging Component failed: {0}")]
    Wic(String),
}

pub fn decode_gif_animation(
    path: &Path,
    target_width: u32,
    target_height: u32,
    limits: DecodeLimits,
    cancellation: &CancellationToken,
) -> Result<DecodedAnimation, DecodeError> {
    if cancellation.is_cancelled() {
        return Err(DecodeError::Cancelled);
    }
    let format = validate_local_image_source(path, limits)?;
    if format != ImageFormat::Gif {
        return Err(DecodeError::UnsupportedFormat);
    }

    let bytes =
        local_io::read_local_snapshot(path, limits.max_local_file_size_bytes, cancellation)?;
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    options.set_memory_limit(gif::MemoryLimit::Bytes(
        std::num::NonZeroU64::new(32 * 1024 * 1024).unwrap(),
    ));
    let mut decoder = options
        .read_info(Cursor::new(&bytes))
        .map_err(|_| DecodeError::UnsupportedFormat)?;
    let width = u32::from(decoder.width());
    let height = u32::from(decoder.height());
    ensure_pixel_limit(width, height, limits)?;

    let (output_width, output_height) = fit_decode_size(
        width,
        height,
        target_width,
        target_height,
        limits.max_output_dimension,
    );
    const MAX_FRAMES: usize = 1_000;
    const MAX_ANIMATION_BYTES: usize = 256 * 1024 * 1024;
    let canvas_bytes = width as usize * height as usize * 4;
    let frame_bytes = output_width as usize * output_height as usize * 4;
    // Reserve source bytes, decoder output, canvas/disposal copies, scaling and
    // channel-conversion scratch before retaining any composited frames.
    let working_bytes = bytes
        .len()
        .saturating_add(canvas_bytes.saturating_mul(4))
        .saturating_add(frame_bytes.saturating_mul(2));
    if working_bytes.saturating_add(frame_bytes) > MAX_ANIMATION_BYTES {
        return Err(DecodeError::Budget("GIF working buffers exceed 256 MiB."));
    }
    let mut frames = Vec::new();
    let mut canvas = vec![0u8; canvas_bytes];
    let mut decoded_bytes = 0usize;

    while let Some(frame) = decoder
        .read_next_frame()
        .map_err(|_| DecodeError::UnsupportedFormat)?
    {
        if cancellation.is_cancelled() {
            return Err(DecodeError::Cancelled);
        }
        if frames.len() >= MAX_FRAMES {
            return Err(DecodeError::Budget(
                "GIF exceeds 1,000 frames; an incomplete animation was not displayed as complete.",
            ));
        }
        if working_bytes
            .saturating_add(decoded_bytes)
            .saturating_add(frame_bytes)
            > MAX_ANIMATION_BYTES
        {
            return Err(DecodeError::Budget(
                "GIF exceeds 256 MiB; an incomplete animation was not displayed as complete.",
            ));
        }
        let restore_canvas = if frame.dispose == DisposalMethod::Previous {
            Some(canvas.clone())
        } else {
            None
        };
        composite_gif_frame(&mut canvas, width, height, frame);
        let rgba = if output_width != width || output_height != height {
            resize_rgba_nearest(&canvas, width, height, output_width, output_height)
        } else {
            canvas.clone()
        };
        let pixels_bgra = rgba_to_bgra(&rgba);
        decoded_bytes = decoded_bytes.saturating_add(pixels_bgra.len());
        frames.push(DecodedFrame {
            width: output_width,
            height: output_height,
            stride: output_width.saturating_mul(4),
            pixels_bgra,
            delay_ms: (u32::from(frame.delay) * 10).max(33),
        });

        match frame.dispose {
            DisposalMethod::Background => clear_gif_frame_rect(&mut canvas, width, height, frame),
            DisposalMethod::Previous => {
                if let Some(restore_canvas) = restore_canvas {
                    canvas = restore_canvas;
                }
            }
            DisposalMethod::Any | DisposalMethod::Keep => {}
        }
    }

    if frames.is_empty() {
        return Err(DecodeError::UnsupportedFormat);
    }
    Ok(DecodedAnimation {
        frames,
        format: ImageFormat::Gif,
    })
}

pub fn sniff_image_format(header: &[u8]) -> ImageFormat {
    if header.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return ImageFormat::Jpeg;
    }
    if header.starts_with(b"\x89PNG\r\n\x1A\n") {
        return ImageFormat::Png;
    }
    if header.starts_with(b"GIF87a") || header.starts_with(b"GIF89a") {
        return ImageFormat::Gif;
    }
    if header.starts_with(b"BM") {
        return ImageFormat::Bmp;
    }
    if header.starts_with(b"II*\0") || header.starts_with(b"MM\0*") {
        return ImageFormat::Tiff;
    }
    if header.starts_with(&[0, 0, 1, 0]) {
        return ImageFormat::Ico;
    }
    if header.len() >= 12 && &header[0..4] == b"RIFF" && &header[8..12] == b"WEBP" {
        return ImageFormat::WebP;
    }
    if header.len() >= 12 && &header[4..8] == b"ftyp" {
        let brand = &header[8..12];
        return match brand {
            b"avif" | b"avis" => ImageFormat::Avif,
            b"heic" | b"heix" | b"hevc" | b"hevx" | b"mif1" | b"msf1" => ImageFormat::Heif,
            _ => ImageFormat::Unknown,
        };
    }
    if header.starts_with(&[0xFF, 0x0A]) {
        return ImageFormat::JpegXl;
    }
    let trimmed = trim_ascii_start(header);
    if trimmed.starts_with(b"<svg") || trimmed.starts_with(b"<?xml") {
        return ImageFormat::Svg;
    }
    ImageFormat::Unknown
}

pub fn validate_local_image_source(
    path: &Path,
    limits: DecodeLimits,
) -> Result<ImageFormat, DecodeError> {
    let metadata =
        local_io::local_metadata(path).map_err(|error| DecodeError::Source(error.to_string()))?;
    if !metadata.is_file() {
        return Err(DecodeError::UnsupportedFormat);
    }
    if metadata.len() > limits.max_local_file_size_bytes {
        return Err(DecodeError::FileTooLarge {
            actual: metadata.len(),
            limit: limits.max_local_file_size_bytes,
        });
    }
    let mut file = local_io::open_preview_file(path)
        .map_err(|error| DecodeError::Source(error.to_string()))?;
    let mut header = [0u8; 64];
    let count = file
        .read(&mut header)
        .map_err(|_| DecodeError::NotReadable)?;
    let format = sniff_image_format(&header[..count]);
    if format == ImageFormat::Unknown {
        Err(DecodeError::UnsupportedFormat)
    } else {
        Ok(format)
    }
}

pub fn ensure_pixel_limit(
    width: u32,
    height: u32,
    limits: DecodeLimits,
) -> Result<(), DecodeError> {
    let pixels = u64::from(width).saturating_mul(u64::from(height));
    if pixels > limits.max_decoded_pixels {
        Err(DecodeError::TooManyPixels {
            pixels,
            limit: limits.max_decoded_pixels,
        })
    } else {
        Ok(())
    }
}

fn trim_ascii_start(bytes: &[u8]) -> &[u8] {
    let mut start = 0;
    while start < bytes.len() && bytes[start].is_ascii_whitespace() {
        start += 1;
    }
    &bytes[start..]
}

pub fn rgba_to_bgra(rgba: &[u8]) -> Vec<u8> {
    let mut bgra = Vec::with_capacity(rgba.len());
    for chunk in rgba.as_chunks::<4>().0 {
        bgra.push(chunk[2]);
        bgra.push(chunk[1]);
        bgra.push(chunk[0]);
        bgra.push(chunk[3]);
    }
    bgra
}

fn resize_rgba_nearest(
    rgba: &[u8],
    width: u32,
    height: u32,
    output_width: u32,
    output_height: u32,
) -> Vec<u8> {
    let mut output = vec![0u8; output_width as usize * output_height as usize * 4];
    for y in 0..output_height {
        let src_y = ((y as u64 * height as u64) / output_height as u64)
            .min(height.saturating_sub(1) as u64) as u32;
        for x in 0..output_width {
            let src_x = ((x as u64 * width as u64) / output_width as u64)
                .min(width.saturating_sub(1) as u64) as u32;
            let src = ((src_y * width + src_x) * 4) as usize;
            let dst = ((y * output_width + x) * 4) as usize;
            if src + 4 <= rgba.len() && dst + 4 <= output.len() {
                output[dst..dst + 4].copy_from_slice(&rgba[src..src + 4]);
            }
        }
    }
    output
}

fn composite_gif_frame(
    canvas: &mut [u8],
    canvas_width: u32,
    canvas_height: u32,
    frame: &gif::Frame<'_>,
) {
    let left = u32::from(frame.left);
    let top = u32::from(frame.top);
    let frame_width = u32::from(frame.width);
    let frame_height = u32::from(frame.height);
    for frame_y in 0..frame_height {
        let canvas_y = top + frame_y;
        if canvas_y >= canvas_height {
            continue;
        }
        for frame_x in 0..frame_width {
            let canvas_x = left + frame_x;
            if canvas_x >= canvas_width {
                continue;
            }
            let src = ((frame_y * frame_width + frame_x) * 4) as usize;
            let dst = ((canvas_y * canvas_width + canvas_x) * 4) as usize;
            if src + 4 > frame.buffer.len() || dst + 4 > canvas.len() {
                continue;
            }
            let alpha = frame.buffer[src + 3] as u32;
            if alpha == 0 {
                continue;
            }
            if alpha == 255 {
                canvas[dst..dst + 4].copy_from_slice(&frame.buffer[src..src + 4]);
                continue;
            }
            let inverse_alpha = 255 - alpha;
            for channel in 0..3 {
                canvas[dst + channel] = (((frame.buffer[src + channel] as u32 * alpha)
                    + (canvas[dst + channel] as u32 * inverse_alpha))
                    / 255) as u8;
            }
            canvas[dst + 3] = (alpha + (canvas[dst + 3] as u32 * inverse_alpha / 255)) as u8;
        }
    }
}

fn clear_gif_frame_rect(
    canvas: &mut [u8],
    canvas_width: u32,
    canvas_height: u32,
    frame: &gif::Frame<'_>,
) {
    let left = u32::from(frame.left);
    let top = u32::from(frame.top);
    let frame_width = u32::from(frame.width);
    let frame_height = u32::from(frame.height);
    for frame_y in 0..frame_height {
        let canvas_y = top + frame_y;
        if canvas_y >= canvas_height {
            continue;
        }
        for frame_x in 0..frame_width {
            let canvas_x = left + frame_x;
            if canvas_x >= canvas_width {
                continue;
            }
            let dst = ((canvas_y * canvas_width + canvas_x) * 4) as usize;
            if dst + 4 <= canvas.len() {
                canvas[dst..dst + 4].fill(0);
            }
        }
    }
}

fn fit_decode_size(
    width: u32,
    height: u32,
    target_width: u32,
    target_height: u32,
    max_dimension: u32,
) -> (u32, u32) {
    let target_width = target_width.max(1).min(max_dimension);
    let target_height = target_height.max(1).min(max_dimension);
    if width <= target_width && height <= target_height {
        return (width, height);
    }
    let scale = (target_width as f32 / width as f32).min(target_height as f32 / height as f32);
    let out_width = ((width as f32 * scale).round() as u32)
        .max(1)
        .min(max_dimension);
    let out_height = ((height as f32 * scale).round() as u32)
        .max(1)
        .min(max_dimension);
    (out_width, out_height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffs_common_formats() {
        assert_eq!(
            sniff_image_format(&[0xFF, 0xD8, 0xFF, 0xE0]),
            ImageFormat::Jpeg
        );
        assert_eq!(
            sniff_image_format(b"\x89PNG\r\n\x1A\nabc"),
            ImageFormat::Png
        );
        assert_eq!(sniff_image_format(b"GIF89a..."), ImageFormat::Gif);
        assert_eq!(sniff_image_format(b"BMxxxx"), ImageFormat::Bmp);
        assert_eq!(sniff_image_format(b"RIFF....WEBPVP8 "), ImageFormat::WebP);
    }

    #[test]
    fn rejects_unknown_header() {
        assert_eq!(sniff_image_format(b"not an image"), ImageFormat::Unknown);
    }

    #[test]
    fn enforces_pixel_limit() {
        let limits = DecodeLimits {
            max_decoded_pixels: 10,
            ..Default::default()
        };
        assert!(ensure_pixel_limit(4, 3, limits).is_err());
        assert!(ensure_pixel_limit(2, 5, limits).is_ok());
    }

    #[test]
    fn converts_rgba_to_bgra() {
        assert_eq!(
            rgba_to_bgra(&[1, 2, 3, 4, 5, 6, 7, 8]),
            vec![3, 2, 1, 4, 7, 6, 5, 8]
        );
    }
}
