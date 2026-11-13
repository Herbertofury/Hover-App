//! Bounded comparison math. Pixel, displayed-line and byte equality are
//! deliberately separate observations; none implies dependency/asset fidelity.
use crate::{CancellationToken, ScaleMode, ZoomConfig, ZoomState};

pub const MAX_DIFF_PIXELS: usize = 8 * 1024 * 1024;
pub const MAX_LINE_CELLS: usize = 4_100_000;

#[derive(Debug, Clone, PartialEq)]
pub struct PixelDifference {
    pub pixels: usize,
    pub changed_pixels: usize,
    pub rgb_changed_pixels: usize,
    pub alpha_changed_pixels: usize,
    pub maximum_channel_delta: u8,
    pub mean_absolute_channel_delta: f64,
}

/// Compare tightly packed decoded BGRA channels at matching pixel coordinates.
/// The visualization retains dim context and marks changed pixels pink. It is
/// a difference mask, not a colorimetric/perceptual similarity score.
pub fn compare_pixels(
    width: u32,
    height: u32,
    left: &[u8],
    right: &[u8],
    token: &CancellationToken,
) -> Result<(PixelDifference, Vec<u8>), &'static str> {
    let count = (width as usize)
        .checked_mul(height as usize)
        .ok_or("Invalid pixel dimensions")?;
    let bytes = count.checked_mul(4).ok_or("Invalid pixel buffer size")?;
    if count == 0 || count > MAX_DIFF_PIXELS {
        return Err("Pixel difference exceeds the 8 Mi-pixel budget");
    }
    if left.len() != bytes || right.len() != bytes {
        return Err("Pixel buffers do not match the declared dimensions");
    }
    let mut mask = Vec::new();
    mask.try_reserve_exact(bytes)
        .map_err(|_| "Not enough memory for the difference mask")?;
    let mut stats = PixelDifference {
        pixels: count,
        changed_pixels: 0,
        rgb_changed_pixels: 0,
        alpha_changed_pixels: 0,
        maximum_channel_delta: 0,
        mean_absolute_channel_delta: 0.0,
    };
    let mut absolute_sum = 0u64;
    for (index, (a, b)) in left
        .as_chunks::<4>()
        .0
        .iter()
        .zip(right.as_chunks::<4>().0.iter())
        .enumerate()
    {
        if index % 4096 == 0 && token.is_cancelled() {
            return Err("Comparison cancelled");
        }
        let rgb = a[..3] != b[..3];
        let alpha = a[3] != b[3];
        stats.changed_pixels += usize::from(rgb || alpha);
        stats.rgb_changed_pixels += usize::from(rgb);
        stats.alpha_changed_pixels += usize::from(alpha);
        for channel in 0..4 {
            let delta = a[channel].abs_diff(b[channel]);
            absolute_sum += u64::from(delta);
            stats.maximum_channel_delta = stats.maximum_channel_delta.max(delta);
        }
        if rgb || alpha {
            mask.extend_from_slice(&[179, 79, 255, 255]);
        } else {
            let gray = ((u32::from(a[0]) + u32::from(a[1]) + u32::from(a[2])) / 9 + 18) as u8;
            mask.extend_from_slice(&[gray, gray, gray, 255]);
        }
    }
    if token.is_cancelled() {
        return Err("Comparison cancelled");
    }
    stats.mean_absolute_channel_delta = absolute_sum as f64 / (count * 4) as f64;
    Ok((stats, mask))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinePair {
    pub left: Option<usize>,
    pub right: Option<usize>,
    pub changed: bool,
}

/// Exact LCS alignment of the *displayed* lines. Allocation and CPU work are
/// bounded; callers retain source truncation flags and never infer full-file
/// equality from a matching displayed prefix.
pub fn compare_lines(
    left: &[String],
    right: &[String],
    token: &CancellationToken,
) -> Result<Vec<LinePair>, &'static str> {
    if token.is_cancelled() {
        return Err("Comparison cancelled");
    }
    let text_bytes = left
        .iter()
        .chain(right)
        .try_fold(0usize, |sum, line| sum.checked_add(line.len()))
        .ok_or("Displayed text size overflow")?;
    if text_bytes > 768 * 1024 {
        return Err("Displayed-line text exceeds its comparison byte budget");
    }
    let columns = right.len().checked_add(1).ok_or("Too many lines")?;
    let cells = left
        .len()
        .checked_add(1)
        .and_then(|rows| rows.checked_mul(columns))
        .ok_or("Too many lines")?;
    if cells > MAX_LINE_CELLS || left.len() > u16::MAX as usize || right.len() > u16::MAX as usize {
        return Err("Displayed-line difference exceeds its alignment budget");
    }
    let mut table = Vec::new();
    table
        .try_reserve_exact(cells)
        .map_err(|_| "Not enough memory to align displayed lines")?;
    table.resize(cells, 0u16);
    for row in (0..left.len()).rev() {
        if token.is_cancelled() {
            return Err("Comparison cancelled");
        }
        for col in (0..right.len()).rev() {
            table[row * columns + col] = if left[row] == right[col] {
                table[(row + 1) * columns + col + 1] + 1
            } else {
                table[(row + 1) * columns + col].max(table[row * columns + col + 1])
            };
        }
    }
    let (mut a, mut b) = (0, 0);
    let mut result = Vec::with_capacity(left.len() + right.len());
    while a < left.len() || b < right.len() {
        if token.is_cancelled() {
            return Err("Comparison cancelled");
        }
        if a < left.len() && b < right.len() && left[a] == right[b] {
            result.push(LinePair {
                left: Some(a),
                right: Some(b),
                changed: false,
            });
            a += 1;
            b += 1;
        } else if a < left.len()
            && (b == right.len() || table[(a + 1) * columns + b] >= table[a * columns + b + 1])
        {
            result.push(LinePair {
                left: Some(a),
                right: None,
                changed: true,
            });
            a += 1;
        } else {
            result.push(LinePair {
                left: None,
                right: Some(b),
                changed: true,
            });
            b += 1;
        }
    }
    Ok(result)
}

/// Match actual image-pixel scale and normalized image center. The target may
/// clamp to its own bounds; callers expose its effective state, not a promise
/// that differently sized images are geometrically identical.
pub fn link_view(source: ZoomState, target: &mut ZoomState) {
    if source.image_size.width <= 0.0
        || source.image_size.height <= 0.0
        || !source.scale.is_finite()
        || source.scale <= 0.0
    {
        return;
    }
    target.scale = source.scale;
    target.mode = ScaleMode::Custom;
    target.pan_x = source.pan_x / (source.image_size.width * source.scale)
        * target.image_size.width
        * source.scale;
    target.pan_y = source.pan_y / (source.image_size.height * source.scale)
        * target.image_size.height
        * source.scale;
    target.high_quality = true;
    target.clamp(ZoomConfig::default());
}

pub fn frame_at(elapsed_ms: u64, delays: &[u32]) -> usize {
    let duration = delays
        .iter()
        .map(|delay| u64::from((*delay).max(20)))
        .sum::<u64>();
    if duration == 0 {
        return 0;
    }
    let mut time = elapsed_ms % duration;
    for (index, delay) in delays.iter().enumerate() {
        let delay = u64::from((*delay).max(20));
        if time < delay {
            return index;
        }
        time -= delay;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Size;
    #[test]
    fn reports_rgb_and_alpha_changes_separately_and_masks_only_changed_pixels() {
        let a = [10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255];
        let b = [10, 20, 30, 255, 41, 50, 60, 255, 70, 80, 90, 128];
        let (s, mask) = compare_pixels(3, 1, &a, &b, &CancellationToken::new()).unwrap();
        assert_eq!(
            (
                s.changed_pixels,
                s.rgb_changed_pixels,
                s.alpha_changed_pixels,
                s.maximum_channel_delta
            ),
            (2, 1, 1, 127)
        );
        assert_ne!(&mask[..4], &[179, 79, 255, 255]);
        assert_eq!(&mask[4..8], &[179, 79, 255, 255]);
        assert_eq!(&mask[8..], &[179, 79, 255, 255]);
        assert!((s.mean_absolute_channel_delta - 128.0 / 12.0).abs() < 0.0001);
    }
    #[test]
    fn refuses_malformed_over_budget_and_cancelled_differences() {
        let token = CancellationToken::new();
        assert!(compare_pixels(1, 1, &[0; 3], &[0; 4], &token).is_err());
        assert!(compare_pixels(u32::MAX, u32::MAX, &[], &[], &token).is_err());
        token.cancel();
        assert!(compare_pixels(1, 1, &[0; 4], &[0; 4], &token).is_err());
        assert!(compare_lines(&["a".into()], &["b".into()], &token).is_err());
        assert!(
            compare_lines(
                &vec!["a".into(); 3000],
                &vec!["a".into(); 3000],
                &CancellationToken::new()
            )
            .is_err()
        );
    }
    #[test]
    fn aligns_insertions_without_mislabeling_the_remaining_lines() {
        let l = ["one", "three"].map(String::from);
        let r = ["one", "two", "three"].map(String::from);
        assert_eq!(
            compare_lines(&l, &r, &CancellationToken::new()).unwrap(),
            vec![
                LinePair {
                    left: Some(0),
                    right: Some(0),
                    changed: false
                },
                LinePair {
                    left: None,
                    right: Some(1),
                    changed: true
                },
                LinePair {
                    left: Some(1),
                    right: Some(2),
                    changed: false
                }
            ]
        );
        assert!(
            compare_lines(&[], &[], &CancellationToken::new())
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn links_pixel_scale_and_normalized_center_across_different_dimensions() {
        let c = ZoomConfig::default();
        let mut a = ZoomState::new(Size::new(800.0, 600.0), Size::new(200.0, 200.0), c);
        a.set_mode(ScaleMode::Native, c);
        a.pan_by(80.0, 60.0, c);
        let mut b = ZoomState::new(Size::new(1600.0, 1200.0), Size::new(200.0, 200.0), c);
        link_view(a, &mut b);
        assert_eq!((b.scale, b.pan_x, b.pan_y), (1.0, 160.0, 120.0));
    }
    #[test]
    fn animation_uses_elapsed_time_and_exact_frame_boundaries() {
        assert_eq!(
            [0, 99, 100, 299, 300].map(|time| frame_at(time, &[100, 200])),
            [0, 0, 1, 1, 0]
        );
        assert_eq!(frame_at(19, &[0, 100]), 0);
        assert_eq!(frame_at(20, &[0, 100]), 1);
        assert_eq!(frame_at(0, &[]), 0);
    }
}
