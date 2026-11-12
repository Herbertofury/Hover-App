use crate::geometry::{Point, Rect, Size};

#[derive(Debug, Clone, Copy)]
pub struct PlacementConfig {
    pub cursor_gap_px: f32,
    pub max_monitor_width_ratio: f32,
    pub max_monitor_height_ratio: f32,
    pub min_width_px: f32,
    pub min_height_px: f32,
}

impl Default for PlacementConfig {
    fn default() -> Self {
        Self {
            cursor_gap_px: 16.0,
            max_monitor_width_ratio: 0.80,
            max_monitor_height_ratio: 0.80,
            min_width_px: 160.0,
            min_height_px: 160.0,
        }
    }
}

pub fn place_preview(
    cursor: Point,
    monitor_work_area: Rect,
    image_size: Size,
    config: PlacementConfig,
) -> Rect {
    if !monitor_work_area.size().is_valid()
        || !monitor_work_area.left.is_finite()
        || !monitor_work_area.top.is_finite()
        || !monitor_work_area.right.is_finite()
        || !monitor_work_area.bottom.is_finite()
    {
        return Rect::new(0.0, 0.0, 1.0, 1.0);
    }
    let cursor = Point::new(
        if cursor.x.is_finite() {
            cursor.x
        } else {
            monitor_work_area.left
        },
        if cursor.y.is_finite() {
            cursor.y
        } else {
            monitor_work_area.top
        },
    );
    let mut config = config;
    let finite_or = |value: f32, fallback| if value.is_finite() { value } else { fallback };
    config.cursor_gap_px = finite_or(config.cursor_gap_px, 16.0).clamp(0.0, 256.0);
    config.max_monitor_width_ratio = finite_or(config.max_monitor_width_ratio, 0.8).clamp(0.1, 1.0);
    config.max_monitor_height_ratio =
        finite_or(config.max_monitor_height_ratio, 0.8).clamp(0.1, 1.0);
    config.min_width_px = finite_or(config.min_width_px, 160.0)
        .max(1.0)
        .min(monitor_work_area.width());
    config.min_height_px = finite_or(config.min_height_px, 160.0)
        .max(1.0)
        .min(monitor_work_area.height());
    let max_width =
        (monitor_work_area.width() * config.max_monitor_width_ratio).max(config.min_width_px);
    let max_height =
        (monitor_work_area.height() * config.max_monitor_height_ratio).max(config.min_height_px);
    let desired = fit_size(image_size, Size::new(max_width, max_height), config);

    let mut left = cursor.x + config.cursor_gap_px;
    let mut top = cursor.y + config.cursor_gap_px;
    if left + desired.width > monitor_work_area.right {
        left = cursor.x - config.cursor_gap_px - desired.width;
    }
    if top + desired.height > monitor_work_area.bottom {
        top = cursor.y - config.cursor_gap_px - desired.height;
    }

    left = left.clamp(
        monitor_work_area.left,
        monitor_work_area.right - desired.width,
    );
    top = top.clamp(
        monitor_work_area.top,
        monitor_work_area.bottom - desired.height,
    );
    Rect::new(left, top, left + desired.width, top + desired.height)
}

fn fit_size(image: Size, max: Size, config: PlacementConfig) -> Size {
    if !image.is_valid() {
        return Size::new(config.min_width_px, config.min_height_px);
    }
    let scale = (max.width / image.width)
        .min(max.height / image.height)
        .min(1.0);
    let width = (image.width * scale)
        .clamp(1.0f32.min(max.width), max.width)
        .max(config.min_width_px.min(max.width));
    let height = (image.height * scale)
        .clamp(1.0f32.min(max.height), max.height)
        .max(config.min_height_px.min(max.height));
    Size::new(width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placement_flips_near_monitor_edges() {
        let monitor = Rect::new(0.0, 0.0, 1000.0, 800.0);
        let rect = place_preview(
            Point::new(980.0, 780.0),
            monitor,
            Size::new(300.0, 200.0),
            PlacementConfig::default(),
        );
        assert!(rect.right <= monitor.right);
        assert!(rect.bottom <= monitor.bottom);
        assert!(rect.left < 980.0);
        assert!(rect.top < 780.0);
    }

    #[test]
    fn placement_stays_inside_negative_secondary_and_tiny_work_areas() {
        for monitor in [
            Rect::new(-2560.0, 0.0, 0.0, 1440.0),
            Rect::new(0.0, -1080.0, 1920.0, 0.0),
            Rect::new(1920.0, 120.0, 3840.0, 1040.0),
            Rect::new(10.0, 20.0, 60.0, 55.0),
        ] {
            for cursor in [
                Point::new(monitor.left, monitor.top),
                Point::new(monitor.right - 1.0, monitor.bottom - 1.0),
            ] {
                for image in [Size::new(8000.0, 6000.0), Size::new(0.0, 0.0)] {
                    let rect = place_preview(cursor, monitor, image, PlacementConfig::default());
                    assert!(rect.size().is_valid());
                    assert!(rect.left >= monitor.left && rect.top >= monitor.top);
                    assert!(rect.right <= monitor.right && rect.bottom <= monitor.bottom);
                }
            }
        }
    }

    #[test]
    fn invalid_numeric_inputs_do_not_panic_or_create_invalid_bounds() {
        let monitor = Rect::new(-500.0, -200.0, 0.0, 100.0);
        let rect = place_preview(
            Point::new(f32::NAN, f32::INFINITY),
            monitor,
            Size::new(f32::NAN, 0.0),
            PlacementConfig {
                min_width_px: f32::NAN,
                max_monitor_height_ratio: f32::INFINITY,
                ..PlacementConfig::default()
            },
        );
        assert!(rect.size().is_valid());
        assert!(rect.left >= monitor.left && rect.right <= monitor.right);
        assert!(rect.top >= monitor.top && rect.bottom <= monitor.bottom);
    }
}
