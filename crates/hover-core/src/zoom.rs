use crate::geometry::{Point, Size};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleMode {
    FitAll,
    FitWidth,
    FitHeight,
    Native,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WheelModifier {
    Plain,
    Shift,
    Ctrl,
}

#[derive(Debug, Clone, Copy)]
pub struct ZoomConfig {
    pub wheel_step: f32,
    pub shift_wheel_step: f32,
    pub ctrl_wheel_step: f32,
    pub min_zoom_percent: f32,
    pub max_zoom_percent: f32,
    pub advanced_max_zoom_percent: f32,
    pub snap_epsilon: f32,
}

impl Default for ZoomConfig {
    fn default() -> Self {
        Self {
            wheel_step: 0.125,
            shift_wheel_step: 0.025,
            ctrl_wheel_step: 0.25,
            min_zoom_percent: 5.0,
            max_zoom_percent: 800.0,
            advanced_max_zoom_percent: 3200.0,
            snap_epsilon: 0.018,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ZoomState {
    pub image_size: Size,
    pub viewport_size: Size,
    pub fit_scale: f32,
    pub scale: f32,
    pub pan_x: f32,
    pub pan_y: f32,
    pub mode: ScaleMode,
    pub high_quality: bool,
}

impl ZoomState {
    pub fn new(image_size: Size, viewport_size: Size, config: ZoomConfig) -> Self {
        let fit_scale = fit_all_scale(image_size, viewport_size);
        let min_scale = minimum_scale(fit_scale, config);
        let scale = fit_scale.max(min_scale);
        Self {
            image_size,
            viewport_size,
            fit_scale,
            scale,
            pan_x: 0.0,
            pan_y: 0.0,
            mode: ScaleMode::FitAll,
            high_quality: true,
        }
    }

    pub fn set_viewport(&mut self, viewport_size: Size, config: ZoomConfig) {
        self.viewport_size = viewport_size;
        self.fit_scale = fit_all_scale(self.image_size, viewport_size);
        if matches!(self.mode, ScaleMode::FitAll) {
            self.scale = self.fit_scale;
        }
        self.clamp(config);
    }

    pub fn set_mode(&mut self, mode: ScaleMode, config: ZoomConfig) {
        self.mode = mode;
        self.scale = match mode {
            ScaleMode::FitAll => self.fit_scale,
            ScaleMode::FitWidth => self.viewport_size.width / self.image_size.width,
            ScaleMode::FitHeight => self.viewport_size.height / self.image_size.height,
            ScaleMode::Native => 1.0,
            ScaleMode::Custom => self.scale,
        };
        if matches!(mode, ScaleMode::FitAll) {
            self.pan_x = 0.0;
            self.pan_y = 0.0;
        }
        self.clamp(config);
    }

    pub fn apply_wheel(
        &mut self,
        cursor: Point,
        wheel_notches: f32,
        modifier: WheelModifier,
        config: ZoomConfig,
    ) {
        if wheel_notches == 0.0 || !wheel_notches.is_finite() {
            return;
        }

        let old_scale = self.scale;
        if old_scale <= 0.0 || !old_scale.is_finite() {
            self.scale = self.fit_scale.max(minimum_scale(self.fit_scale, config));
            self.pan_x = 0.0;
            self.pan_y = 0.0;
            return;
        }

        let old_top_left = self.image_top_left();
        let anchor_image_x = (cursor.x - old_top_left.x) / old_scale;
        let anchor_image_y = (cursor.y - old_top_left.y) / old_scale;

        let step = match modifier {
            WheelModifier::Plain => config.wheel_step,
            WheelModifier::Shift => config.shift_wheel_step,
            WheelModifier::Ctrl => config.ctrl_wheel_step,
        };

        let notch_count = wheel_notches.abs();
        let per_notch = 1.0 + step;
        let factor = per_notch.powf(notch_count);
        let proposed = if wheel_notches > 0.0 {
            old_scale * factor
        } else {
            old_scale / factor
        };

        self.scale = snap_scale(old_scale, proposed, self.fit_scale, config);
        self.mode = if (self.scale - self.fit_scale).abs() <= config.snap_epsilon {
            ScaleMode::FitAll
        } else if (self.scale - 1.0).abs() <= config.snap_epsilon {
            ScaleMode::Native
        } else {
            ScaleMode::Custom
        };

        let new_scaled_width = self.image_size.width * self.scale;
        let new_scaled_height = self.image_size.height * self.scale;
        self.pan_x = cursor.x - anchor_image_x * self.scale + new_scaled_width * 0.5
            - self.viewport_size.width * 0.5;
        self.pan_y = cursor.y - anchor_image_y * self.scale + new_scaled_height * 0.5
            - self.viewport_size.height * 0.5;
        self.high_quality = false;
        self.clamp(config);
    }

    pub fn pan_by(&mut self, dx: f32, dy: f32, config: ZoomConfig) {
        self.pan_x += dx;
        self.pan_y += dy;
        self.clamp(config);
    }

    pub fn settle_quality(&mut self) {
        self.high_quality = true;
    }

    pub fn image_top_left(self) -> Point {
        let scaled_width = self.image_size.width * self.scale;
        let scaled_height = self.image_size.height * self.scale;
        Point::new(
            self.viewport_size.width * 0.5 + self.pan_x - scaled_width * 0.5,
            self.viewport_size.height * 0.5 + self.pan_y - scaled_height * 0.5,
        )
    }

    pub fn clamp(&mut self, config: ZoomConfig) {
        let min_scale = minimum_scale(self.fit_scale, config);
        let max_scale = config.max_zoom_percent.max(1.0) / 100.0;
        if !self.scale.is_finite() || self.scale <= 0.0 {
            self.scale = self.fit_scale.max(min_scale);
        }
        self.scale = self.scale.clamp(min_scale, max_scale);

        let scaled_width = self.image_size.width * self.scale;
        let scaled_height = self.image_size.height * self.scale;

        self.pan_x = clamp_axis(self.pan_x, scaled_width, self.viewport_size.width);
        self.pan_y = clamp_axis(self.pan_y, scaled_height, self.viewport_size.height);

        if !self.pan_x.is_finite() {
            self.pan_x = 0.0;
        }
        if !self.pan_y.is_finite() {
            self.pan_y = 0.0;
        }
    }
}

pub fn fit_all_scale(image: Size, viewport: Size) -> f32 {
    if !image.is_valid() || !viewport.is_valid() {
        return 1.0;
    }
    (viewport.width / image.width)
        .min(viewport.height / image.height)
        .max(0.0001)
}

pub fn fit_width_scale(image: Size, viewport: Size) -> f32 {
    if !image.is_valid() || !viewport.is_valid() {
        return 1.0;
    }
    (viewport.width / image.width).max(0.0001)
}

pub fn fit_height_scale(image: Size, viewport: Size) -> f32 {
    if !image.is_valid() || !viewport.is_valid() {
        return 1.0;
    }
    (viewport.height / image.height).max(0.0001)
}

fn minimum_scale(fit_scale: f32, config: ZoomConfig) -> f32 {
    fit_scale.min(config.min_zoom_percent / 100.0).max(0.0001)
}

fn snap_scale(old_scale: f32, proposed: f32, fit_scale: f32, config: ZoomConfig) -> f32 {
    let mut scale = proposed;
    for snap in [fit_scale, 1.0] {
        let crossed =
            (old_scale <= snap && proposed >= snap) || (old_scale >= snap && proposed <= snap);
        if crossed && (proposed - snap).abs() <= config.snap_epsilon.max(snap * config.snap_epsilon)
        {
            scale = snap;
        }
    }
    scale
}

fn clamp_axis(pan: f32, scaled: f32, viewport: f32) -> f32 {
    if scaled <= viewport {
        0.0
    } else {
        let max_pan = (scaled - viewport) * 0.5;
        pan.clamp(-max_pan, max_pan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.75
    }

    #[test]
    fn fit_scales_are_correct() {
        let image = Size::new(4000.0, 2000.0);
        let viewport = Size::new(1000.0, 1000.0);
        assert!((fit_all_scale(image, viewport) - 0.25).abs() < 0.001);
        assert!((fit_width_scale(image, viewport) - 0.25).abs() < 0.001);
        assert!((fit_height_scale(image, viewport) - 0.5).abs() < 0.001);
    }

    #[test]
    fn wheel_zoom_preserves_cursor_anchor() {
        let config = ZoomConfig::default();
        let mut zoom = ZoomState::new(Size::new(1000.0, 1000.0), Size::new(500.0, 500.0), config);
        let cursor = Point::new(375.0, 250.0);
        let old_left = zoom.image_top_left();
        let image_x = (cursor.x - old_left.x) / zoom.scale;
        let image_y = (cursor.y - old_left.y) / zoom.scale;

        zoom.apply_wheel(cursor, 1.0, WheelModifier::Plain, config);

        let new_left = zoom.image_top_left();
        let anchored_x = new_left.x + image_x * zoom.scale;
        let anchored_y = new_left.y + image_y * zoom.scale;
        assert!(approx_eq(anchored_x, cursor.x));
        assert!(approx_eq(anchored_y, cursor.y));
    }

    #[test]
    fn zoom_never_becomes_invalid() {
        let config = ZoomConfig::default();
        let mut zoom = ZoomState::new(Size::new(1000.0, 800.0), Size::new(500.0, 400.0), config);
        for _ in 0..200 {
            zoom.apply_wheel(Point::new(250.0, 200.0), -1.0, WheelModifier::Ctrl, config);
        }
        assert!(zoom.scale.is_finite());
        assert!(zoom.scale > 0.0);
        for _ in 0..400 {
            zoom.apply_wheel(Point::new(250.0, 200.0), 1.0, WheelModifier::Ctrl, config);
        }
        assert!(zoom.scale <= 8.0);
        assert!(zoom.pan_x.is_finite());
        assert!(zoom.pan_y.is_finite());
    }

    #[test]
    fn pan_clamps_to_image_edges() {
        let config = ZoomConfig::default();
        let mut zoom = ZoomState::new(Size::new(1000.0, 1000.0), Size::new(500.0, 500.0), config);
        zoom.set_mode(ScaleMode::Native, config);
        zoom.pan_by(10_000.0, -10_000.0, config);
        assert_eq!(zoom.pan_x, 250.0);
        assert_eq!(zoom.pan_y, -250.0);
    }
}
