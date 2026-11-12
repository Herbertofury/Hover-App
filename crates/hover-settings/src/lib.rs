use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Settings {
    pub general: GeneralSettings,
    pub hover: HoverSettings,
    pub preview: PreviewSettings,
    pub zoom: ZoomSettings,
    pub pan: PanSettings,
    pub decode: DecodeSettings,
    pub cache: CacheSettings,
    pub power: PowerSettings,
    pub diagnostics: DiagnosticsSettings,
    pub browser: BrowserSettings,
    pub preview_types: PreviewTypeSettings,
    pub video: VideoSettings,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeneralSettings {
    pub enabled: bool,
    pub start_with_windows: bool,
    pub show_tray_icon: bool,
    pub telemetry: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            start_with_windows: false,
            show_tray_icon: true,
            telemetry: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HoverSettings {
    pub delay_ms: u64,
    pub movement_threshold_px: i32,
    pub eligible_poll_hz: u32,
    pub idle_poll_hz: u32,
    pub foreground_detection: String,
    pub hide_on_escape: bool,
    pub hide_on_click: bool,
    pub hide_in_fullscreen: bool,
}

impl Default for HoverSettings {
    fn default() -> Self {
        Self {
            delay_ms: 250,
            movement_threshold_px: 3,
            eligible_poll_hz: 30,
            idle_poll_hz: 0,
            foreground_detection: "winevent".to_string(),
            hide_on_escape: true,
            hide_on_click: true,
            hide_in_fullscreen: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreviewSettings {
    pub initial_mode: String,
    pub max_monitor_width_ratio: f32,
    pub max_monitor_height_ratio: f32,
    pub min_width_px: u32,
    pub min_height_px: u32,
    pub cursor_gap_px: u32,
    pub metadata_footer: bool,
    pub rounded_corners: bool,
    pub shadow: bool,
    pub fade_animation: bool,
}

impl Default for PreviewSettings {
    fn default() -> Self {
        Self {
            initial_mode: "fit_all".to_string(),
            max_monitor_width_ratio: 0.80,
            max_monitor_height_ratio: 0.80,
            min_width_px: 160,
            min_height_px: 160,
            cursor_gap_px: 16,
            metadata_footer: false,
            rounded_corners: true,
            shadow: true,
            fade_animation: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZoomSettings {
    pub wheel_zoom_enabled: bool,
    pub plain_wheel_zooms: bool,
    pub cursor_centered_zoom: bool,
    pub wheel_zoom_step: f32,
    pub shift_wheel_zoom_step: f32,
    pub ctrl_wheel_zoom_step: f32,
    pub min_zoom_percent: f32,
    pub max_zoom_percent: f32,
    pub advanced_max_zoom_percent: f32,
    pub snap_to_fit: bool,
    pub snap_to_100: bool,
    pub wheel_browse_enabled: bool,
    pub alt_wheel_browse_enabled: bool,
}

impl Default for ZoomSettings {
    fn default() -> Self {
        Self {
            wheel_zoom_enabled: true,
            plain_wheel_zooms: true,
            cursor_centered_zoom: true,
            wheel_zoom_step: 0.125,
            shift_wheel_zoom_step: 0.025,
            ctrl_wheel_zoom_step: 0.25,
            min_zoom_percent: 5.0,
            max_zoom_percent: 800.0,
            advanced_max_zoom_percent: 3200.0,
            snap_to_fit: true,
            snap_to_100: true,
            wheel_browse_enabled: false,
            alt_wheel_browse_enabled: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PanSettings {
    pub enabled: bool,
    pub middle_drag_pan: bool,
    pub space_left_drag_pan: bool,
    pub arrow_key_pan: bool,
}

impl Default for PanSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            middle_drag_pan: true,
            space_left_drag_pan: true,
            arrow_key_pan: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecodeSettings {
    pub wic_first: bool,
    pub wic_native_downscale: bool,
    pub shell_thumbnail_fallback: bool,
    pub shell_thumbnail_cache_only_first: bool,
    pub rust_fallback_decoders: bool,
    pub lazy_initialize_decoders: bool,
    pub max_local_file_size_mb: u64,
    pub max_decoded_megapixels: u64,
    pub max_worker_threads: u32,
    pub allow_second_worker_when_visible: bool,
    pub cancel_stale_decodes: bool,
}

impl Default for DecodeSettings {
    fn default() -> Self {
        Self {
            wic_first: true,
            wic_native_downscale: true,
            shell_thumbnail_fallback: true,
            shell_thumbnail_cache_only_first: true,
            rust_fallback_decoders: true,
            lazy_initialize_decoders: true,
            max_local_file_size_mb: 512,
            max_decoded_megapixels: 200,
            max_worker_threads: 1,
            allow_second_worker_when_visible: true,
            cancel_stale_decodes: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CacheSettings {
    pub decoded_cache_mb: u64,
    pub remote_cache_enabled: bool,
    pub remote_decoded_cache_mb: u64,
    pub evict_on_memory_pressure: bool,
    pub hard_fail_when_cache_limit_exceeded: bool,
}

impl Default for CacheSettings {
    fn default() -> Self {
        Self {
            decoded_cache_mb: 64,
            remote_cache_enabled: false,
            remote_decoded_cache_mb: 128,
            evict_on_memory_pressure: true,
            hard_fail_when_cache_limit_exceeded: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PowerSettings {
    pub force_dormant_on_session_lock: bool,
    pub force_dormant_on_sleep: bool,
    pub disable_on_battery_saver: bool,
}

impl Default for PowerSettings {
    fn default() -> Self {
        Self {
            force_dormant_on_session_lock: true,
            force_dormant_on_sleep: true,
            disable_on_battery_saver: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DiagnosticsSettings {
    pub enabled: bool,
    pub hot_path_logging: bool,
    pub perf_counters: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BrowserSettings {
    pub enabled: bool,
    pub native_messaging: bool,
    pub remote_fetch_timeout_seconds: u64,
    pub remote_max_size_mb: u64,
    pub memory_only_remote_cache: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreviewTypeSettings {
    pub images: bool,
    pub videos: bool,
    pub audio: bool,
    pub folders: bool,
    pub zips: bool,
    pub text: bool,
    pub sims_packages: bool,
}

impl Default for PreviewTypeSettings {
    fn default() -> Self {
        Self {
            images: true,
            videos: true,
            audio: true,
            folders: true,
            zips: true,
            text: true,
            sims_packages: true,
        }
    }
}

impl Default for BrowserSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            native_messaging: true,
            remote_fetch_timeout_seconds: 10,
            remote_max_size_mb: 50,
            memory_only_remote_cache: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoSettings {
    pub audio_enabled: bool,
    pub seek_seconds: u32,
}

impl Default for VideoSettings {
    fn default() -> Self {
        Self {
            audio_enabled: true,
            seek_seconds: 5,
        }
    }
}

impl Settings {
    pub fn parse_lossy(input: &str) -> Self {
        let mut settings = Settings::default();
        let values = parse_sections(input);

        settings.general.enabled =
            bool_value(&values, "general", "enabled", settings.general.enabled);
        settings.general.start_with_windows = bool_value(
            &values,
            "general",
            "start_with_windows",
            settings.general.start_with_windows,
        );
        settings.general.show_tray_icon = bool_value(
            &values,
            "general",
            "show_tray_icon",
            settings.general.show_tray_icon,
        );
        settings.general.telemetry = false;

        settings.hover.delay_ms = u64_value(&values, "hover", "delay_ms", settings.hover.delay_ms);
        settings.hover.movement_threshold_px = i32_value(
            &values,
            "hover",
            "movement_threshold_px",
            settings.hover.movement_threshold_px,
        );
        settings.hover.eligible_poll_hz = u32_value(
            &values,
            "hover",
            "eligible_poll_hz",
            settings.hover.eligible_poll_hz,
        )
        .min(30);
        settings.hover.idle_poll_hz = 0;
        settings.hover.foreground_detection = string_value(
            &values,
            "hover",
            "foreground_detection",
            &settings.hover.foreground_detection,
        );
        settings.hover.hide_on_escape = bool_value(
            &values,
            "hover",
            "hide_on_escape",
            settings.hover.hide_on_escape,
        );
        settings.hover.hide_on_click = bool_value(
            &values,
            "hover",
            "hide_on_click",
            settings.hover.hide_on_click,
        );
        settings.hover.hide_in_fullscreen = bool_value(
            &values,
            "hover",
            "hide_in_fullscreen",
            settings.hover.hide_in_fullscreen,
        );

        settings.preview.initial_mode = string_value(
            &values,
            "preview",
            "initial_mode",
            &settings.preview.initial_mode,
        );
        settings.preview.max_monitor_width_ratio = f32_value(
            &values,
            "preview",
            "max_monitor_width_ratio",
            settings.preview.max_monitor_width_ratio,
        )
        .clamp(0.1, 0.8);
        settings.preview.max_monitor_height_ratio = f32_value(
            &values,
            "preview",
            "max_monitor_height_ratio",
            settings.preview.max_monitor_height_ratio,
        )
        .clamp(0.1, 0.8);
        settings.preview.min_width_px = u32_value(
            &values,
            "preview",
            "min_width_px",
            settings.preview.min_width_px,
        );
        settings.preview.min_height_px = u32_value(
            &values,
            "preview",
            "min_height_px",
            settings.preview.min_height_px,
        );
        settings.preview.cursor_gap_px = u32_value(
            &values,
            "preview",
            "cursor_gap_px",
            settings.preview.cursor_gap_px,
        );

        settings.zoom.wheel_zoom_enabled = bool_value(
            &values,
            "zoom",
            "wheel_zoom_enabled",
            settings.zoom.wheel_zoom_enabled,
        );
        settings.zoom.plain_wheel_zooms = bool_value(
            &values,
            "zoom",
            "plain_wheel_zooms",
            settings.zoom.plain_wheel_zooms,
        );
        settings.zoom.cursor_centered_zoom = bool_value(
            &values,
            "zoom",
            "cursor_centered_zoom",
            settings.zoom.cursor_centered_zoom,
        );
        settings.zoom.wheel_zoom_step = f32_value(
            &values,
            "zoom",
            "wheel_zoom_step",
            settings.zoom.wheel_zoom_step,
        );
        settings.zoom.shift_wheel_zoom_step = f32_value(
            &values,
            "zoom",
            "shift_wheel_zoom_step",
            settings.zoom.shift_wheel_zoom_step,
        );
        settings.zoom.ctrl_wheel_zoom_step = f32_value(
            &values,
            "zoom",
            "ctrl_wheel_zoom_step",
            settings.zoom.ctrl_wheel_zoom_step,
        );
        settings.zoom.min_zoom_percent = f32_value(
            &values,
            "zoom",
            "min_zoom_percent",
            settings.zoom.min_zoom_percent,
        )
        .max(1.0);
        settings.zoom.max_zoom_percent = f32_value(
            &values,
            "zoom",
            "max_zoom_percent",
            settings.zoom.max_zoom_percent,
        )
        .clamp(100.0, 800.0);
        settings.zoom.advanced_max_zoom_percent = f32_value(
            &values,
            "zoom",
            "advanced_max_zoom_percent",
            settings.zoom.advanced_max_zoom_percent,
        )
        .clamp(800.0, 3200.0);
        settings.zoom.wheel_browse_enabled = false;
        settings.zoom.alt_wheel_browse_enabled = false;

        settings.decode.max_local_file_size_mb = u64_value(
            &values,
            "decode",
            "max_local_file_size_mb",
            settings.decode.max_local_file_size_mb,
        );
        settings.decode.max_decoded_megapixels = u64_value(
            &values,
            "decode",
            "max_decoded_megapixels",
            settings.decode.max_decoded_megapixels,
        );
        settings.decode.max_worker_threads = u32_value(
            &values,
            "decode",
            "max_worker_threads",
            settings.decode.max_worker_threads,
        )
        .clamp(1, 2);

        settings.cache.decoded_cache_mb = u64_value(
            &values,
            "cache",
            "decoded_cache_mb",
            settings.cache.decoded_cache_mb,
        );
        settings.cache.remote_cache_enabled = bool_value(
            &values,
            "cache",
            "remote_cache_enabled",
            settings.cache.remote_cache_enabled,
        );
        settings.cache.remote_decoded_cache_mb = u64_value(
            &values,
            "cache",
            "remote_decoded_cache_mb",
            settings.cache.remote_decoded_cache_mb,
        );

        settings.browser.enabled =
            bool_value(&values, "browser", "enabled", settings.browser.enabled);
        settings.browser.remote_fetch_timeout_seconds = u64_value(
            &values,
            "browser",
            "remote_fetch_timeout_seconds",
            settings.browser.remote_fetch_timeout_seconds,
        );
        settings.browser.remote_max_size_mb = u64_value(
            &values,
            "browser",
            "remote_max_size_mb",
            settings.browser.remote_max_size_mb,
        );
        settings.preview_types.images = bool_value(
            &values,
            "preview_types",
            "images",
            settings.preview_types.images,
        );
        settings.preview_types.videos = bool_value(
            &values,
            "preview_types",
            "videos",
            settings.preview_types.videos,
        );
        settings.preview_types.audio = bool_value(
            &values,
            "preview_types",
            "audio",
            settings.preview_types.audio,
        );
        settings.preview_types.folders = bool_value(
            &values,
            "preview_types",
            "folders",
            settings.preview_types.folders,
        );
        settings.preview_types.zips = bool_value(
            &values,
            "preview_types",
            "zips",
            settings.preview_types.zips,
        );
        settings.preview_types.text = bool_value(
            &values,
            "preview_types",
            "text",
            settings.preview_types.text,
        );
        settings.preview_types.sims_packages = bool_value(
            &values,
            "preview_types",
            "sims_packages",
            settings.preview_types.sims_packages,
        );
        settings.video.audio_enabled = bool_value(
            &values,
            "video",
            "audio_enabled",
            settings.video.audio_enabled,
        );
        settings.video.seek_seconds = u32_value(
            &values,
            "video",
            "seek_seconds",
            settings.video.seek_seconds,
        )
        .clamp(1, 60);
        settings
    }
}

type SectionMap = HashMap<String, HashMap<String, String>>;

fn parse_sections(input: &str) -> SectionMap {
    let mut current = String::new();
    let mut values = SectionMap::new();
    for raw_line in input.lines() {
        let line = raw_line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            current = line[1..line.len() - 1].trim().to_ascii_lowercase();
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            values.entry(current.clone()).or_default().insert(
                key.trim().to_ascii_lowercase(),
                value.trim().trim_matches('"').to_string(),
            );
        }
    }
    values
}

fn raw<'a>(values: &'a SectionMap, section: &str, key: &str) -> Option<&'a str> {
    values.get(section)?.get(key).map(String::as_str)
}

fn bool_value(values: &SectionMap, section: &str, key: &str, default: bool) -> bool {
    match raw(values, section, key)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("true") => true,
        Some("false") => false,
        _ => default,
    }
}

fn string_value(values: &SectionMap, section: &str, key: &str, default: &str) -> String {
    raw(values, section, key).unwrap_or(default).to_string()
}

fn u64_value(values: &SectionMap, section: &str, key: &str, default: u64) -> u64 {
    raw(values, section, key)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn u32_value(values: &SectionMap, section: &str, key: &str, default: u32) -> u32 {
    raw(values, section, key)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn i32_value(values: &SectionMap, section: &str, key: &str, default: i32) -> i32 {
    raw(values, section, key)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn f32_value(values: &SectionMap, section: &str, key: &str, default: f32) -> f32 {
    raw(values, section, key)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_parse_known_values() {
        let settings = Settings::parse_lossy(
            r#"
            [hover]
            delay_ms = 300
            eligible_poll_hz = 90
            [zoom]
            wheel_zoom_step = 0.2
            wheel_browse_enabled = true
            [browser]
            enabled = true
            [preview_types]
            videos = false
            sims_packages = false
            [video]
            audio_enabled = false
            seek_seconds = 7
            "#,
        );
        assert_eq!(settings.hover.delay_ms, 300);
        assert_eq!(settings.hover.eligible_poll_hz, 30);
        assert_eq!(settings.zoom.wheel_zoom_step, 0.2);
        assert!(!settings.zoom.wheel_browse_enabled);
        assert!(settings.browser.enabled);
        assert!(!settings.preview_types.videos);
        assert!(!settings.preview_types.sims_packages);
        assert!(settings.preview_types.images);
        assert!(!settings.video.audio_enabled);
        assert_eq!(settings.video.seek_seconds, 7);
    }

    #[test]
    fn invalid_settings_fall_back() {
        let settings = Settings::parse_lossy(
            r#"
            [hover]
            delay_ms = nope
            idle_poll_hz = 99
            [general]
            telemetry = true
            "#,
        );
        assert_eq!(settings.hover.delay_ms, 250);
        assert_eq!(settings.hover.idle_poll_hz, 0);
        assert!(!settings.general.telemetry);
    }
}
