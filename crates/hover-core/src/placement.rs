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
        .clamp(1.0, max.width)
        .max(config.min_width_px.min(max.width));
    let height = (image.height * scale)
        .clamp(1.0, max.height)
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
}
