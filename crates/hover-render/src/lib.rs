use hover_core::{Point, Rect, Size, ZoomState};
use hover_decode::DecodedImage;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderQuality {
    Fast,
    High,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DirtyFlags {
    pub source: bool,
    pub zoom: bool,
    pub pan: bool,
    pub viewport: bool,
    pub frame: bool,
}

impl DirtyFlags {
    pub fn any(self) -> bool {
        self.source || self.zoom || self.pan || self.viewport || self.frame
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[derive(Debug, Clone)]
pub struct RenderPlan {
    pub source_rect: Rect,
    pub dest_rect: Rect,
    pub quality: RenderQuality,
}

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("renderer is hidden")]
    Hidden,
    #[error("no source bitmap is loaded")]
    NoSource,
}

#[derive(Debug, Default)]
pub struct DirtyRendererState {
    visible: bool,
    bitmap_uploaded: bool,
    pub dirty: DirtyFlags,
    pub render_presents: u64,
    pub bitmap_uploads: u64,
}

impl DirtyRendererState {
    pub fn show(&mut self) {
        self.visible = true;
        self.dirty.viewport = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.dirty.clear();
    }

    pub fn set_source(&mut self, _image: &DecodedImage) {
        self.bitmap_uploaded = true;
        self.bitmap_uploads += 1;
        self.dirty.source = true;
    }

    pub fn mark_zoom_dirty(&mut self) {
        self.dirty.zoom = true;
    }

    pub fn present_if_dirty(&mut self) -> Result<bool, RenderError> {
        if !self.visible {
            return Err(RenderError::Hidden);
        }
        if !self.bitmap_uploaded {
            return Err(RenderError::NoSource);
        }
        if self.dirty.any() {
            self.render_presents += 1;
            self.dirty.clear();
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

pub fn build_render_plan(image: &DecodedImage, zoom: ZoomState) -> RenderPlan {
    let top_left = zoom.image_top_left();
    let scaled = Size::new(
        image.width as f32 * zoom.scale,
        image.height as f32 * zoom.scale,
    );
    RenderPlan {
        source_rect: Rect::new(0.0, 0.0, image.width as f32, image.height as f32),
        dest_rect: Rect::new(
            top_left.x,
            top_left.y,
            top_left.x + scaled.width,
            top_left.y + scaled.height,
        ),
        quality: if zoom.high_quality {
            RenderQuality::High
        } else {
            RenderQuality::Fast
        },
    }
}

pub fn cursor_inside_preview(cursor: Point, preview_rect: Rect) -> bool {
    preview_rect.contains(cursor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hover_core::{ZoomConfig, ZoomState};
    use hover_decode::ImageFormat;

    fn image() -> DecodedImage {
        DecodedImage {
            width: 100,
            height: 100,
            stride: 400,
            pixels_bgra: vec![0; 40_000],
            format: ImageFormat::Png,
        }
    }

    #[test]
    fn dirty_renderer_presents_only_when_dirty() {
        let mut renderer = DirtyRendererState::default();
        renderer.show();
        renderer.set_source(&image());
        assert!(renderer.present_if_dirty().unwrap());
        assert!(!renderer.present_if_dirty().unwrap());
        renderer.mark_zoom_dirty();
        assert!(renderer.present_if_dirty().unwrap());
        assert_eq!(renderer.render_presents, 2);
        assert_eq!(renderer.bitmap_uploads, 1);
    }

    #[test]
    fn render_plan_tracks_zoom_quality() {
        let config = ZoomConfig::default();
        let mut zoom = ZoomState::new(Size::new(100.0, 100.0), Size::new(50.0, 50.0), config);
        zoom.high_quality = false;
        let plan = build_render_plan(&image(), zoom);
        assert_eq!(plan.quality, RenderQuality::Fast);
    }
}
