// SPDX-License-Identifier: MIT
//! Saved thumbnail only; format constants independently implemented from Blender's public format headers.
use crate::{DecodeLimits, DecodedFrame};

pub const PREFIX_BYTES: usize = 2 * 1024 * 1024;
const MAX_EDGE: u32 = 512;

#[derive(Debug)]
pub enum SavedPreview {
    Image(DecodedFrame),
    NoSavedImage,
    UnsupportedContainer,
    InvalidMetadata,
}

fn get(bytes: &[u8], start: usize, len: usize) -> Option<&[u8]> {
    bytes.get(start..start.checked_add(len)?)
}

fn version_is_digits(bytes: &[u8]) -> bool {
    bytes.iter().all(u8::is_ascii_digit) && bytes.iter().any(|&digit| digit != b'0')
}

fn block_layout(bytes: &[u8]) -> Option<(usize, usize, usize, usize)> {
    match bytes.get(7..9)? {
        b"-v" if version_is_digits(bytes.get(9..12)?) && bytes.get(9..12)? >= &b"250"[..] => Some((12, 24, 4, 4)),
        b"_v" if version_is_digits(bytes.get(9..12)?) && bytes.get(9..12)? >= &b"250"[..] => Some((12, 20, 4, 4)),
        b"17" if bytes.get(9..13)? == b"-01v" && version_is_digits(bytes.get(13..17)?) => {
            Some((17, 32, 16, 8))
        }
        _ => None,
    }
}

fn payload(bytes: &[u8], start: usize, len: usize, limits: DecodeLimits) -> Option<DecodedFrame> {
    let header = get(bytes, start, 8)?;
    let width = u32::from_le_bytes(header[..4].try_into().ok()?);
    let height = u32::from_le_bytes(header[4..].try_into().ok()?);
    let max_edge = MAX_EDGE.min(limits.max_output_dimension);
    if width == 0 || height == 0 || width > max_edge || height > max_edge {
        return None;
    }
    let pixels = u64::from(width) * u64::from(height);
    if pixels > limits.max_decoded_pixels {
        return None;
    }
    let stride = width * 4;
    let expected = (pixels * 4) as usize;
    if len != expected + 8 {
        return None;
    }
    let data = get(bytes, start + 8, expected)?;
    let mut pixels_bgra = Vec::with_capacity(expected);
    for row in data.rchunks_exact(stride as usize) {
        for texel in row.chunks_exact(4) {
            pixels_bgra.extend_from_slice(&[texel[2], texel[1], texel[0], texel[3]]);
        }
    }
    Some(DecodedFrame {
        width,
        height,
        stride,
        pixels_bgra,
        delay_ms: 0,
    })
}

/// Only accepts supported little-endian, uncompressed headers and initial thumbnail metadata.
pub fn inspect(bytes: &[u8], limits: DecodeLimits) -> SavedPreview {
    let view = &bytes[..bytes.len().min(PREFIX_BYTES)];
    if !view.starts_with(b"BLENDER") {
        return SavedPreview::UnsupportedContainer;
    }
    let Some((mut offset, block_width, length_offset, length_width)) = block_layout(view) else {
        return SavedPreview::UnsupportedContainer;
    };
    for _ in 0..64 {
        let Some(block) = get(view, offset, block_width) else {
            return SavedPreview::InvalidMetadata;
        };
        let marker = &block[..4];
        if marker != b"REND" && marker != b"TEST" {
            return SavedPreview::NoSavedImage;
        }
        let Some(encoded) = block.get(length_offset..length_offset + length_width) else {
            return SavedPreview::InvalidMetadata;
        };
        let mut padded = [0_u8; 8];
        padded[..length_width].copy_from_slice(encoded);
        let Ok(len) = usize::try_from(u64::from_le_bytes(padded)) else {
            return SavedPreview::InvalidMetadata;
        };
        let start = offset + block_width;
        if len > PREFIX_BYTES || get(view, start, len).is_none() {
            return SavedPreview::InvalidMetadata;
        }
        if marker == b"TEST" {
            return match payload(view, start, len, limits) {
                Some(frame) => SavedPreview::Image(frame),
                None => SavedPreview::InvalidMetadata,
            };
        }
        offset = start + len;
    }
    SavedPreview::InvalidMetadata
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(header: &[u8], layout: (usize, usize, usize), tag: &[u8; 4]) -> Vec<u8> {
        let (block_width, offset, length_width) = layout;
        let mut bytes = header.to_vec();
        let mut block = vec![0; block_width];
        block[..4].copy_from_slice(tag);
        let shape_and_pixels = [
            1, 0, 0, 0, 2, 0, 0, 0, // width and height
            1, 2, 3, 255, // bottom
            11, 12, 13, 255, // top
        ];
        block[offset..offset + length_width]
            .copy_from_slice(&(shape_and_pixels.len() as u64).to_le_bytes()[..length_width]);
        bytes.extend(block);
        bytes.extend(shape_and_pixels);
        bytes
    }

    #[test]
    fn supports_both_header_generations_with_top_down_bgra() {
        for (header, layout) in [
            (&b"BLENDER-v405"[..], (24, 4, 4)),
            (&b"BLENDER_v300"[..], (20, 4, 4)),
            (&b"BLENDER17-01v0503"[..], (32, 16, 8)),
        ] {
            let data = fixture(header, layout, b"TEST");
            let SavedPreview::Image(frame) = inspect(&data, DecodeLimits::default()) else {
                panic!("saved image not recognized");
            };
            assert_eq!((frame.width, frame.height, frame.stride), (1, 2, 4));
            assert_eq!(frame.pixels_bgra, [13, 12, 11, 255, 3, 2, 1, 255]);
        }
    }

    #[test]
    fn distinguishes_no_image_unsupported_and_invalid_metadata() {
        let header = b"BLENDER17-01v0503";
        let mut data = fixture(header, (32, 16, 8), b"DATA");
        assert!(matches!(inspect(&data, DecodeLimits::default()), SavedPreview::NoSavedImage));
        data[17..21].copy_from_slice(b"TEST");
        data.pop();
        assert!(matches!(inspect(&data, DecodeLimits::default()), SavedPreview::InvalidMetadata));
        assert!(matches!(inspect(b"\x1f\x8b", DecodeLimits::default()), SavedPreview::UnsupportedContainer));
    }

    #[test]
    fn first_non_metadata_block_stops_the_search() {
        let header = b"BLENDER-v405";
        let mut data = fixture(header, (24, 4, 4), b"REND");
        data.extend_from_slice(&fixture(header, (24, 4, 4), b"TEST")[header.len()..]);
        assert!(matches!(inspect(&data, DecodeLimits::default()), SavedPreview::Image(_)));
        data[header.len()..header.len() + 4].copy_from_slice(b"DATA");
        assert!(matches!(inspect(&data, DecodeLimits::default()), SavedPreview::NoSavedImage));
    }
}
