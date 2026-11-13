//! Code-native Hover mark shared by the tray and executable resources.
pub fn icon_image(size: usize) -> (Vec<u8>, Vec<u8>) {
    let stride = size.div_ceil(32) * 4;
    let mut pixels = vec![0; size * size * 4];
    let mut mask = vec![0; stride * size];
    for y in 0..size {
        for x in 0..size {
            let sx = (x as f32 + 0.5) * 64.0 / size as f32 - 0.5;
            let sy = (y as f32 + 0.5) * 64.0 / size as f32 - 0.5;
            let distance = ((sx - 31.5).powi(2) + (sy - 31.5).powi(2)).sqrt() / 31.5;
            let idx = ((size - 1 - y) * size + x) * 4;
            if distance > 1.0 {
                mask[(size - 1 - y) * stride + x / 8] |= 0x80 >> (x % 8);
                continue;
            }
            let rim = (1.0 - distance).clamp(0.0, 1.0);
            let glow = (sx / 64.0 * 0.55 + (1.0 - sy / 64.0) * 0.45).clamp(0.0, 1.0);
            let color = if (18.0..47.0).contains(&sy)
                && ((18.0..25.0).contains(&sx) || (39.0..46.0).contains(&sx))
                || (29.0..36.0).contains(&sy) && (18.0..46.0).contains(&sx)
            {
                [255, 255, 255, 255]
            } else if (20.0..45.0).contains(&sy)
                && (49.0..54.0).contains(&sx)
                && (sy > 31.0 || sx < 52.0)
            {
                [207, 236, 255, 230]
            } else {
                [
                    (236.0 + 18.0 * rim) as u8,
                    (124.0 + 64.0 * glow + 28.0 * rim) as u8,
                    (42.0 + 58.0 * glow + 36.0 * rim) as u8,
                    255,
                ]
            };
            pixels[idx..idx + 4].copy_from_slice(&color);
        }
    }
    (pixels, mask)
}

#[allow(dead_code)] // Only the resource build needs the ICO container.
pub fn ico() -> Vec<u8> {
    let sizes = [16usize, 32, 48, 64, 256];
    let mut result = vec![0, 0, 1, 0, sizes.len() as u8, 0];
    let mut offset = 6 + sizes.len() * 16;
    let mut images = Vec::new();
    for size in sizes {
        let (pixels, mask) = icon_image(size);
        let mut data = Vec::new();
        for value in [40, size as u32, size as u32 * 2] {
            data.extend(value.to_le_bytes());
        }
        data.extend(1u16.to_le_bytes());
        data.extend(32u16.to_le_bytes());
        for value in [0, (pixels.len() + mask.len()) as u32, 0, 0, 0, 0] {
            data.extend(value.to_le_bytes());
        }
        data.extend(pixels);
        data.extend(mask);
        result.extend([size as u8, size as u8, 0, 0]);
        result.extend(1u16.to_le_bytes());
        result.extend(32u16.to_le_bytes());
        result.extend((data.len() as u32).to_le_bytes());
        result.extend((offset as u32).to_le_bytes());
        offset += data.len();
        images.extend(data);
    }
    result.extend(images);
    result
}
