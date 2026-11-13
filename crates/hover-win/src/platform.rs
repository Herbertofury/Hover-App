#![allow(unsafe_op_in_unsafe_fn)]

mod agent_control;

use crate::resolver::resolve_hover_path;
use hover_core::{
    CancellationToken, HoverState, PlacementConfig, Point as HoverPoint, PreviewWorker,
    Rect as HoverRect, ResourceCounters, Size, WheelModifier, ZoomConfig, ZoomState, place_preview,
};
use hover_decode::DecodedFrame;
use hover_decode::local_io::{local_metadata, open_preview_file};
use hover_settings::{PreviewTypeSettings, Settings, VideoSettings};
use std::collections::HashMap;
use std::env;
use std::ffi::c_void;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{ERROR_SUCCESS, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BeginPaint, COLORONCOLOR, CreateSolidBrush,
    DIB_RGB_COLORS, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE, DT_TOP, DT_WORDBREAK,
    DeleteObject, DrawTextW, EndPaint, FillRect, FrameRect, GetMonitorInfoW, HALFTONE,
    InvalidateRect, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint, PAINTSTRUCT, SRCCOPY,
    ScreenToClient, SetBkMode, SetBrushOrgEx, SetStretchBltMode, SetTextColor, StretchDIBits,
    TRANSPARENT,
};
use windows_sys::Win32::Graphics::Gdi::{
    ClientToScreen, DC_BRUSH, DEFAULT_GUI_FONT, GetStockObject, IntersectClipRect, RestoreDC,
    SaveDC, SetBkColor, SetDCBrushColor,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_SZ, RegCloseKey, RegDeleteValueW,
    RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
};
use windows_sys::Win32::System::SystemServices::{SS_CENTERIMAGE, SS_ENDELLIPSIS, SS_NOPREFIX};
use windows_sys::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, GetFocus, IsWindowEnabled, SetFocus,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyState, VK_CONTROL, VK_DOWN, VK_ESCAPE, VK_LBUTTON, VK_LEFT, VK_RIGHT,
    VK_SHIFT, VK_SPACE, VK_UP,
};
use windows_sys::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, SHFILEINFOW, SHGFI_ICON,
    SHGFI_SMALLICON, SHGetFileInfoW, Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, BM_GETCHECK, BM_SETCHECK, BS_AUTOCHECKBOX, BS_PUSHBUTTON, CS_HREDRAW, CS_VREDRAW,
    CW_USEDEFAULT, CallNextHookEx, CreateIcon, CreatePopupMenu, CreateWindowExW, DI_NORMAL,
    DefWindowProcW, DestroyIcon, DestroyMenu, DestroyWindow, DispatchMessageW, DrawIconEx,
    EVENT_SYSTEM_FOREGROUND, EnumWindows, GW_OWNER, GWL_EXSTYLE, GetClassNameW, GetClientRect,
    GetCursorPos, GetDlgItem, GetForegroundWindow, GetMessageW, GetSystemMetrics, GetWindow,
    GetWindowLongPtrW, GetWindowRect, GetWindowThreadProcessId, HC_ACTION, HHOOK, HICON,
    HWND_TOPMOST, IDI_APPLICATION, IsWindowVisible, KBDLLHOOKSTRUCT, KillTimer, LWA_ALPHA,
    LWA_COLORKEY, LoadIconW, MF_CHECKED, MF_SEPARATOR, MF_STRING, MF_UNCHECKED, MSG,
    MSLLHOOKSTRUCT, PostMessageW, PostQuitMessage, RegisterClassExW, SM_CXSCREEN, SM_CYSCREEN,
    SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
    SendMessageW, SetForegroundWindow, SetLayeredWindowAttributes, SetTimer, SetWindowLongPtrW,
    SetWindowPos, SetWindowTextW, SetWindowsHookExW, ShowWindow, TPM_BOTTOMALIGN, TPM_LEFTALIGN,
    TrackPopupMenu, TranslateMessage, UnhookWindowsHookEx, WH_KEYBOARD_LL, WH_MOUSE_LL,
    WHEEL_DELTA, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_APP, WM_CLOSE, WM_COMMAND,
    WM_DESTROY, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONUP, WM_MOUSEWHEEL,
    WM_PAINT, WM_POWERBROADCAST, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_TIMER, WNDCLASSEXW,
    WS_CAPTION, WS_CHILD, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_POPUP, WS_SYSMENU, WS_VISIBLE,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GA_ROOT, GWL_STYLE, GetAncestor, IsDialogMessageW, IsIconic, IsWindow, MA_NOACTIVATE,
    MF_GRAYED, SWP_FRAMECHANGED, TPM_RETURNCMD, WM_CTLCOLORSTATIC, WM_LBUTTONDOWN,
    WM_MOUSEACTIVATE, WM_MOVE, WM_SETFONT, WM_SIZE, WS_CLIPCHILDREN, WS_EX_APPWINDOW,
    WS_OVERLAPPEDWINDOW, WS_TABSTOP,
};
use windows_sys::core::BOOL;
use zip::ZipArchive;
use zip::read::{ArchiveOffset, Config as ZipConfig};

pub(crate) const CONTROL_CLASS: &str = "HoverControlWindow";
const OVERLAY_CLASS: &str = "HoverOverlayWindow";
const OPTIONS_CLASS: &str = "HoverOptionsWindow";
const WM_HOVER_FOREGROUND_CHANGED: u32 = WM_APP + 10;
const WM_TRAYICON: u32 = WM_APP + 11;
const WM_PREVIEW_READY: u32 = WM_APP + 12;
const WM_MEDIA_PLAY: u32 = WM_APP + 13;
const WM_PREVIEW_ACTION: u32 = WM_APP + 14;
const ID_PREVIEW_PIN: usize = 1201;
const ID_PREVIEW_RELOAD: usize = 1202;
const ID_PREVIEW_CLOSE: usize = 1203;
const ID_PREVIEW_TITLE: i32 = 1204;
const ID_PREVIEW_SYNC: usize = 1205;
const PREVIEW_CHROME_HEIGHT: i32 = 40;
const TIMER_MEDIA: usize = 4;
const ID_TRAY_ENABLE: usize = 1001;
const ID_TRAY_STARTUP: usize = 1002;
const ID_TRAY_OPTIONS: usize = 1003;
const ID_TRAY_EXIT: usize = 1004;
const ID_OPTION_IMAGES: i32 = 1101;
const ID_OPTION_VIDEOS: i32 = 1102;
const ID_OPTION_AUDIO: i32 = 1103;
const ID_OPTION_FOLDERS: i32 = 1104;
const ID_OPTION_ZIPS: i32 = 1105;
const ID_OPTION_TEXT: i32 = 1106;
const ID_OPTION_SIMS: i32 = 1107;
const ID_OPTION_VIDEO_AUDIO: i32 = 1108;
const ID_OPTION_SEEK_MINUS: i32 = 1109;
const ID_OPTION_SEEK_PLUS: i32 = 1110;
const ID_OPTION_SEEK_LABEL: i32 = 1111;
const ID_OPTION_CLOSE: i32 = 1199;
const BST_CHECKED_VALUE: usize = 1;
const TIMER_CURSOR: usize = 1;
const TIMER_ZOOM_SETTLE: usize = 2;
const TIMER_ANIMATION: usize = 3;
const ELIGIBLE_CURSOR_TIMER_MS: u32 = 33;
const ZOOM_SETTLE_TIMER_MS: u32 = 100;
const HOVER_STABLE_MS: u64 = 180;
const VIDEO_STARTUP_GRACE_MS: u64 = 1_500;
const VIDEO_ATTACH_RETRY_MS: u64 = 50;
const VIDEO_STEADY_SYNC_MS: u64 = 1_500;
const DEFAULT_VIDEO_SEEK_SECONDS: u32 = 5;
const MAX_STATIC_IMAGE_PREVIEW_DIMENSION: u32 = 8192;
const MAX_ANIMATED_IMAGE_PREVIEW_WIDTH: u32 = 1600;
const MAX_ANIMATED_IMAGE_PREVIEW_HEIGHT: u32 = 1200;
const MAX_PREVIEW_ENTRIES: usize = 2_000;
const MAX_TEXT_PREVIEW_BYTES: u64 = 384 * 1024;
const MAX_TEXT_PREVIEW_LINES: usize = 2_000;
const MAX_TEXT_LINE_CHARS: usize = 520;
const MAX_SIM_PACKAGE_INDEX_ENTRIES: usize = 5_000;
const TEXT_LINE_HEIGHT: i32 = 20;
const LIST_ROW_HEIGHT: i32 = 32;
const WHEEL_SCROLL_ROWS: usize = 3;
const CREATE_NO_WINDOW: u32 = 0x08000000;
const TRANSPARENT_IMAGE_COLOR: u32 = 0x00030201;
const STARTUP_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const STARTUP_VALUE_NAME: &str = "Hover";

static CONTROL_HWND: AtomicIsize = AtomicIsize::new(0);
static OVERLAY_HWND: AtomicIsize = AtomicIsize::new(0);
static OPTIONS_HWND: AtomicIsize = AtomicIsize::new(0);
static VIDEO_HWND: AtomicIsize = AtomicIsize::new(0);
static VIDEO_PID: AtomicU32 = AtomicU32::new(0);
static VIDEO_RECT_LEFT: AtomicI32 = AtomicI32::new(0);
static VIDEO_RECT_TOP: AtomicI32 = AtomicI32::new(0);
static VIDEO_RECT_RIGHT: AtomicI32 = AtomicI32::new(0);
static VIDEO_RECT_BOTTOM: AtomicI32 = AtomicI32::new(0);
static VIDEO_SEEK_SECONDS: AtomicU32 = AtomicU32::new(DEFAULT_VIDEO_SEEK_SECONDS);
static OVERLAY_MODEL: OnceLock<Mutex<Option<OverlayModel>>> = OnceLock::new();
static PREVIEW_PINNED: AtomicBool = AtomicBool::new(false);
static TRAY_MENU_OPEN: AtomicBool = AtomicBool::new(false);

#[derive(Clone)]
struct OverlayFrame {
    width: u32,
    height: u32,
    stride: u32,
    pixels_bgra: Vec<u8>,
    delay_ms: u32,
}

struct OverlayModel {
    content: OverlayContent,
    current_frame: usize,
    scroll_offset: usize,
}

#[derive(Clone, Copy)]
enum SavedView {
    Image { zoom: ZoomState, frame: usize },
    Text { scroll: usize },
    List { scroll: usize },
}

impl OverlayModel {
    fn save_view(&self) -> SavedView {
        match &self.content {
            OverlayContent::Frames { zoom, .. } => SavedView::Image {
                zoom: *zoom,
                frame: self.current_frame,
            },
            OverlayContent::Text { .. } => SavedView::Text {
                scroll: self.scroll_offset,
            },
            OverlayContent::List { .. } => SavedView::List {
                scroll: self.scroll_offset,
            },
        }
    }

    fn restore_view(&mut self, saved: SavedView, viewport: Size, visible_rows: usize) -> bool {
        match (&mut self.content, saved) {
            (OverlayContent::Frames { frames, zoom }, SavedView::Image { zoom: old, frame })
                if frames.first().is_some_and(|image| {
                    old.image_size == Size::new(image.width as f32, image.height as f32)
                }) =>
            {
                *zoom = old;
                zoom.set_viewport(viewport, ZoomConfig::default());
                self.current_frame = frame.min(frames.len().saturating_sub(1));
                true
            }
            (
                OverlayContent::Text {
                    lines, truncated, ..
                },
                SavedView::Text { scroll },
            ) => {
                self.scroll_offset = scroll
                    .min(text_virtual_line_count(lines, *truncated).saturating_sub(visible_rows));
                true
            }
            (OverlayContent::List { entries, .. }, SavedView::List { scroll }) => {
                self.scroll_offset = scroll.min(entries.len().saturating_sub(visible_rows));
                true
            }
            _ => false,
        }
    }
}

enum OverlayContent {
    Frames {
        frames: Vec<OverlayFrame>,
        zoom: ZoomState,
    },
    Text {
        title: String,
        subtitle: String,
        lines: Vec<String>,
        truncated: bool,
    },
    List {
        title: String,
        subtitle: String,
        entries: Vec<ListEntry>,
        omitted: usize,
    },
}

#[derive(Clone)]
struct ListEntry {
    name: String,
    detail: String,
    path: Option<PathBuf>,
    kind: EntryKind,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum EntryKind {
    Directory,
    File,
    ZipDirectory,
    ZipFile,
}

enum PreviewPayload {
    Frames {
        frames: Vec<OverlayFrame>,
    },
    Text {
        title: String,
        subtitle: String,
        lines: Vec<String>,
        truncated: bool,
    },
    List {
        title: String,
        subtitle: String,
        entries: Vec<ListEntry>,
        omitted: usize,
    },
    Video {
        path: PathBuf,
    },
    Audio {
        path: PathBuf,
        title: String,
        subtitle: String,
    },
}

struct PreviewResult {
    path: PathBuf,
    result: Result<PreviewPayload, String>,
}

struct VideoPlayback {
    process: Child,
    rect: HoverRect,
    started: Instant,
    last_window_sync: Instant,
    window_attached: bool,
}

struct AudioPlayback {
    process: Child,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MediaBackend {
    Ffplay,
    Mpv,
    Vlc,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OverlayWheelAction {
    None,
    Repaint,
    Zoom,
}

pub fn run() -> Result<(), i32> {
    run_inner(None)
}

pub(crate) fn run_agent(launch: crate::agent::Launch) -> Result<(), i32> {
    run_inner(Some(launch))
}

fn run_inner(agent: Option<crate::agent::Launch>) -> Result<(), i32> {
    // Normal launches share one resident per installation and user. Private
    // stdio sessions remain independent and never signal a normal session.
    let scope = if agent.is_none() {
        Some(crate::resident::Scope::current().map_err(|error| {
            eprintln!("{error}");
            1
        })?)
    } else {
        None
    };
    let _resident_guard = if let Some(scope) = &scope {
        match scope.claim().map_err(|error| {
            eprintln!("{error}");
            1
        })? {
            Some(guard) => Some(guard),
            None => {
                // A simultaneous first launch may still be creating its window.
                for _ in 0..50 {
                    if scope.signal(true).map_err(|error| {
                        eprintln!("{error}");
                        1
                    })? {
                        return Ok(());
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                eprintln!("Hover is already starting or its lifecycle window is unavailable.");
                return Err(1);
            }
        }
    } else {
        None
    };
    unsafe {
        let control_hwnd = match create_app_window() {
            Ok(hwnd) => hwnd,
            Err(code) => {
                if let Some(agent) = agent {
                    let _ = agent.ready.try_send(Err(code));
                }
                return Err(code);
            }
        };
        if let Some(scope) = scope {
            windows_sys::Win32::UI::WindowsAndMessaging::SetWindowTextW(
                control_hwnd,
                wide(&scope.title).as_ptr(),
            );
        }
        if let Some(agent) = agent {
            let ptr = GetWindowLongPtrW(
                control_hwnd,
                windows_sys::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
            ) as *mut App;
            (*ptr).agent_controlled = true;
            (*ptr).agent_requests = Some(agent.receiver);
            let _ = agent.ready.try_send(Ok(control_hwnd as isize));
        }

        let hook = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            null_mut(),
            Some(foreground_event_proc),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );
        // Launching with Explorer already active needs initial evaluation too.
        PostMessageW(control_hwnd, WM_HOVER_FOREGROUND_CHANGED, 0, 0);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            if preprocess_preview_message(&msg) {
                continue;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        if !hook.is_null() {
            UnhookWinEvent(hook);
        }
        if IsWindow(control_hwnd) != 0 {
            DestroyWindow(control_hwnd);
        }
        Ok(())
    }
}

unsafe fn create_app_window() -> Result<HWND, i32> {
    let _ = windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID(
        windows::core::PCWSTR(wide(crate::cli::APP_USER_MODEL_ID).as_ptr()),
    );
    let hinstance = GetModuleHandleW(null());
    if hinstance.is_null() {
        return Err(1);
    }

    register_window_class(hinstance, CONTROL_CLASS, Some(control_window_proc))?;
    register_window_class(hinstance, OVERLAY_CLASS, Some(overlay_window_proc))?;
    register_window_class(hinstance, OPTIONS_CLASS, Some(options_window_proc))?;

    let control_hwnd = CreateWindowExW(
        WS_EX_TOOLWINDOW,
        wide(CONTROL_CLASS).as_ptr(),
        wide("Hover Control").as_ptr(),
        WS_POPUP,
        0,
        0,
        0,
        0,
        null_mut(),
        null_mut(),
        hinstance,
        null_mut(),
    );
    if control_hwnd.is_null() {
        return Err(1);
    }
    CONTROL_HWND.store(control_hwnd as isize, Ordering::Release);

    let app = Box::new(App::new(control_hwnd));
    windows_sys::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(
        control_hwnd,
        windows_sys::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
        Box::into_raw(app) as isize,
    );

    Ok(control_hwnd)
}

struct App {
    hwnd: HWND,
    overlay: HWND,
    settings: Settings,
    state: HoverState,
    counters: ResourceCounters,
    tray_added: bool,
    candidate_path: Option<PathBuf>,
    dismissed_path: Option<PathBuf>,
    candidate_since: Instant,
    current_path: Option<PathBuf>,
    pending_path: Option<PathBuf>,
    decode_generation: u64,
    decode_worker: Option<PreviewWorker<(PathBuf, PreviewTypeSettings), PreviewResult>>,
    video: Option<VideoPlayback>,
    audio: Option<AudioPlayback>,
    media_ready: Option<PreviewPayload>,
    mouse_hook: HHOOK,
    keyboard_hook: HHOOK,
    tray_icon: HICON,
    tray_icon_owned: bool,
    pinned: bool,
    origin_foreground: HWND,
    preview_anchor: Option<HoverPoint>,
    // Last successful same-file view while reloading or displaying a retryable
    // error. Error panels never replace this small snapshot with their scroll.
    reload_view: Option<(PathBuf, SavedView)>,
    glance_rect: Option<HoverRect>,
    agent_controlled: bool,
    agent_requests: Option<std::sync::mpsc::Receiver<crate::agent::Request>>,
    preview_error: Option<String>,
}

impl App {
    unsafe fn new(hwnd: HWND) -> Self {
        let overlay = create_overlay_window();
        let mut settings = load_settings();
        settings.general.start_with_windows = startup_enabled();
        let (tray_icon, tray_icon_owned) = create_hover_tray_icon()
            .map(|icon| (icon, true))
            .unwrap_or_else(|| (LoadIconW(null_mut(), IDI_APPLICATION), false));
        let mut app = Self {
            hwnd,
            overlay,
            settings,
            state: HoverState::Dormant,
            counters: ResourceCounters::default(),
            tray_added: false,
            candidate_path: None,
            dismissed_path: None,
            candidate_since: Instant::now(),
            current_path: None,
            pending_path: None,
            decode_generation: 0,
            decode_worker: None,
            video: None,
            audio: None,
            media_ready: None,
            mouse_hook: null_mut(),
            keyboard_hook: null_mut(),
            tray_icon,
            tray_icon_owned,
            pinned: false,
            origin_foreground: null_mut(),
            preview_anchor: None,
            reload_view: None,
            glance_rect: None,
            agent_controlled: false,
            agent_requests: None,
            preview_error: None,
        };
        if app.settings.general.show_tray_icon {
            app.tray_added = add_tray_icon(hwnd, app.tray_icon);
        }
        app
    }

    unsafe fn on_foreground_changed(&mut self) {
        if self.agent_controlled {
            return;
        }
        if !self.settings.general.enabled {
            self.enter_dormant();
            return;
        }
        if self.pinned {
            return;
        }
        if self.keep_active_preview_during_media_foreground() {
            return;
        }
        if fullscreen_or_ineligible_foreground() {
            self.enter_dormant();
            return;
        }
        if foreground_is_eligible() {
            self.enter_foreground_eligible();
        } else {
            self.enter_dormant();
        }
    }

    unsafe fn enter_dormant(&mut self) {
        KillTimer(self.hwnd, TIMER_CURSOR);
        KillTimer(self.hwnd, TIMER_ZOOM_SETTLE);
        self.cancel_decode();
        self.hide_preview();
        self.candidate_path = None;
        self.state = HoverState::Dormant;
        self.counters.active_timers = 0;
        self.counters.wheel_hooks = 0;
        self.counters.raw_input_captures = 0;
    }

    unsafe fn enter_foreground_eligible(&mut self) {
        self.origin_foreground = GetForegroundWindow();
        if self.state == HoverState::ForegroundEligible {
            return;
        }
        KillTimer(self.hwnd, TIMER_CURSOR);
        let timer = SetTimer(self.hwnd, TIMER_CURSOR, ELIGIBLE_CURSOR_TIMER_MS, None);
        self.counters.active_timers = if timer == 0 { 0 } else { 1 };
        self.state = HoverState::ForegroundEligible;
    }

    unsafe fn on_timer(&mut self, timer_id: usize) {
        match timer_id {
            TIMER_MEDIA if self.pinned => {
                self.maintain_active_video();
                self.maintain_active_audio();
                self.sync_pinned_media();
            }
            TIMER_CURSOR => {
                if self.pinned || TRAY_MENU_OPEN.load(Ordering::Acquire) {
                    return;
                }
                if !self.state.allows_cursor_timer()
                    || (!foreground_is_eligible()
                        && !self.keep_active_preview_during_media_foreground())
                {
                    self.enter_dormant();
                    return;
                }
                if self.handle_preview_cancel_input() {
                    return;
                }
                self.maintain_active_video();
                self.maintain_active_audio();
                self.update_hover_target();
                self.poll_decode_result();
            }
            TIMER_ZOOM_SETTLE => {
                KillTimer(self.hwnd, TIMER_ZOOM_SETTLE);
                if settle_overlay_zoom_quality() {
                    InvalidateRect(self.overlay, null(), 0);
                }
                if self.counters.active_timers > 0 {
                    self.counters.active_timers -= 1;
                }
            }
            TIMER_ANIMATION => self.advance_animation_frame(),
            _ => {}
        }
    }

    unsafe fn update_hover_target(&mut self) {
        if self.pinned || self.agent_controlled {
            return;
        }
        if self.current_path.is_some() && self.cursor_inside_active_preview() {
            return;
        }
        if self.video.is_some() && self.keep_active_preview_during_media_foreground() {
            return;
        }

        let target =
            resolve_hover_path().or_else(|| env::var_os("HOVER_PREVIEW_PATH").map(PathBuf::from));
        let Some(path) = target else {
            self.dismissed_path = None;
            if self.current_path.is_some() || self.pending_path.is_some() {
                self.cancel_decode();
                self.hide_preview();
            }
            self.candidate_path = None;
            if self.state != HoverState::ForegroundEligible {
                self.state = HoverState::ForegroundEligible;
            }
            return;
        };

        if self.dismissed_path.as_ref() == Some(&path) {
            return;
        }
        self.dismissed_path = None;

        if self.candidate_path.as_ref() != Some(&path) {
            // Revoke old identity immediately, before the new dwell interval.
            self.cancel_decode();
            self.hide_preview();
            self.candidate_path = Some(path);
            self.candidate_since = Instant::now();
            self.state = HoverState::HoverCandidate;
            return;
        }

        if self.candidate_since.elapsed() < Duration::from_millis(HOVER_STABLE_MS) {
            self.state = HoverState::HoverCandidate;
            return;
        }

        let path = self.candidate_path.clone().unwrap();
        if self.current_path.as_ref() == Some(&path) || self.pending_path.as_ref() == Some(&path) {
            return;
        }
        self.cancel_decode();
        self.hide_preview();
        self.start_decode(path);
    }

    unsafe fn start_decode(&mut self, path: PathBuf) {
        self.preview_error = None;
        if self.reload_view.is_none() {
            self.preview_anchor = Some(current_cursor_point());
        }
        if self.decode_worker.is_none() {
            match PreviewWorker::new(
                |(path, preview_types): (PathBuf, PreviewTypeSettings), token| PreviewResult {
                    result: build_preview_payload(&path, token, &preview_types),
                    path,
                },
                || {
                    let hwnd = CONTROL_HWND.load(Ordering::Acquire) as HWND;
                    if !hwnd.is_null() {
                        PostMessageW(hwnd, WM_PREVIEW_READY, 0, 0);
                    }
                },
            ) {
                Ok(worker) => self.decode_worker = Some(worker),
                Err(error) => {
                    self.preview_error = Some(error.to_string());
                    self.show_payload(
                        path,
                        PreviewPayload::Text {
                            title: "Preview unavailable".into(),
                            subtitle: "Worker could not start".into(),
                            lines: vec![error.to_string()],
                            truncated: false,
                        },
                    );
                    return;
                }
            }
        }
        if self.reload_view.is_none() {
            self.show_payload(
                path.clone(),
                PreviewPayload::Text {
                    title: path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    subtitle: "Loading preview".into(),
                    lines: vec!["Reading this file. Move away or press Escape to cancel.".into()],
                    truncated: false,
                },
            );
        }
        update_preview_chrome(self.overlay, &path, self.pinned, true, "");
        self.decode_generation = self
            .decode_worker
            .as_ref()
            .unwrap()
            .submit((path.clone(), self.settings.preview_types.clone()))
            .0;
        self.pending_path = Some(path);
        self.counters.decode_jobs = 1;
        self.counters.worker_threads_awake = 1;
        self.state = HoverState::DecodePending;
    }

    unsafe fn poll_decode_result(&mut self) {
        let Some(worker) = self.decode_worker.as_ref() else {
            return;
        };
        let busy = u32::from(worker.is_busy());
        self.counters.decode_jobs = busy;
        self.counters.worker_threads_awake = busy;
        let Some(completed) = worker.take_result() else {
            return;
        };
        if completed.generation.0 != self.decode_generation {
            return;
        }
        let Some(path) = self.pending_path.take() else {
            return;
        };
        let result = match completed.value {
            Ok(result) if result.path == path => result.result,
            Ok(_) => Err(
                "Preview identity changed; the result was discarded. Reload to try again.".into(),
            ),
            Err(_) => Err("The preview decoder failed. This file was not changed.".into()),
        };
        match result {
            Ok(payload) => {
                self.show_payload(path.clone(), payload);
                if let Some((identity, saved)) = self.reload_view.take()
                    && identity == path
                {
                    let restored = restore_overlay_view(self.overlay, saved);
                    update_preview_chrome(
                        self.overlay,
                        &path,
                        self.pinned,
                        false,
                        if restored {
                            "View restored"
                        } else {
                            "View reset: preview type or dimensions changed"
                        },
                    );
                    InvalidateRect(self.overlay, null(), 0);
                }
            }
            Err(message) => {
                self.preview_error = Some(message.clone());
                let mut lines = vec![message];
                if self
                    .reload_view
                    .as_ref()
                    .is_some_and(|(identity, _)| identity == &path)
                {
                    lines.push("Your previous view is saved. Reload to retry.".into());
                }
                self.show_payload(
                    path.clone(),
                    PreviewPayload::Text {
                        title: path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned(),
                        subtitle: "Preview unavailable".into(),
                        lines,
                        truncated: false,
                    },
                );
            }
        }
    }

    unsafe fn cancel_decode(&mut self) {
        if let Some(worker) = &self.decode_worker {
            worker.cancel();
            let busy = u32::from(worker.is_busy());
            self.counters.decode_jobs = busy;
            self.counters.worker_threads_awake = busy;
        }
        self.pending_path = None;
        self.reload_view = None;
    }

    unsafe fn show_payload(&mut self, path: PathBuf, payload: PreviewPayload) {
        self.media_ready = None;
        if matches!(
            payload,
            PreviewPayload::Video { .. } | PreviewPayload::Audio { .. }
        ) {
            let label = if matches!(payload, PreviewPayload::Video { .. }) {
                "Video"
            } else {
                "Audio"
            };
            self.show_payload_inner(path.clone(), PreviewPayload::Text {
                title: path.file_name().unwrap_or_default().to_string_lossy().into_owned(),
                subtitle: format!("{label} · Playback paused"),
                lines: vec!["Click to play, or press Enter/Space with pinned content focused.".into(),
                    "Playback uses your installed media player. Close the preview or press Escape to stop.".into()],
                truncated: false,
            });
            self.media_ready = Some(payload);
            return;
        }
        self.show_payload_inner(path, payload);
    }

    unsafe fn play_ready_media(&mut self) {
        if let (Some(payload), Some(path)) = (self.media_ready.take(), self.current_path.clone()) {
            self.show_payload_inner(path, payload);
        }
    }

    unsafe fn show_payload_inner(&mut self, path: PathBuf, payload: PreviewPayload) {
        self.stop_video();
        self.stop_audio();
        self.uninstall_preview_hooks();
        KillTimer(self.hwnd, TIMER_ANIMATION);
        let cursor = self.preview_anchor.unwrap_or_else(current_cursor_point);
        let mut desired_size = payload_size(&payload);
        desired_size.width = desired_size.width.max(380.0);
        desired_size.height += PREVIEW_CHROME_HEIGHT as f32;
        let rect = if self.pinned && IsWindowVisible(self.overlay) != 0 {
            overlay_screen_rect(self.overlay)
        } else {
            place_preview(
                cursor,
                screen_work_area(cursor),
                desired_size,
                PlacementConfig::default(),
            )
        };
        match payload {
            PreviewPayload::Frames { frames } => {
                configure_overlay_for_image(self.overlay);
                let is_animated = frames.len() > 1;
                let image_size = frames
                    .first()
                    .map(|frame| Size::new(frame.width as f32, frame.height as f32))
                    .unwrap_or_else(|| Size::new(1.0, 1.0));
                let zoom = ZoomState::new(
                    image_size,
                    Size::new(
                        rect.width(),
                        (rect.height() - PREVIEW_CHROME_HEIGHT as f32).max(1.0),
                    ),
                    ZoomConfig::default(),
                );
                set_overlay_model(Some(OverlayModel {
                    content: OverlayContent::Frames { frames, zoom },
                    current_frame: 0,
                    scroll_offset: 0,
                }));
                self.place_overlay(rect);
                self.install_mouse_hook();
                if is_animated {
                    let delay = current_overlay_frame_delay_ms().unwrap_or(33);
                    SetTimer(self.hwnd, TIMER_ANIMATION, delay, None);
                    self.counters.active_timers = self.counters.active_timers.max(2);
                }
            }
            PreviewPayload::Text {
                title,
                subtitle,
                lines,
                truncated,
            } => {
                configure_overlay_for_panel(self.overlay);
                set_overlay_model(Some(OverlayModel {
                    content: OverlayContent::Text {
                        title,
                        subtitle,
                        lines,
                        truncated,
                    },
                    current_frame: 0,
                    scroll_offset: 0,
                }));
                self.place_overlay(rect);
                self.install_mouse_hook();
            }
            PreviewPayload::List {
                title,
                subtitle,
                entries,
                omitted,
            } => {
                configure_overlay_for_panel(self.overlay);
                set_overlay_model(Some(OverlayModel {
                    content: OverlayContent::List {
                        title,
                        subtitle,
                        entries,
                        omitted,
                    },
                    current_frame: 0,
                    scroll_offset: 0,
                }));
                self.place_overlay(rect);
                self.install_mouse_hook();
            }
            PreviewPayload::Video { path: video_path } => {
                configure_overlay_for_panel(self.overlay);
                if self.pinned {
                    self.place_overlay(rect);
                }
                let player_rect = if self.pinned {
                    overlay_media_rect(self.overlay)
                } else {
                    rect
                };
                match start_video_playback(&video_path, player_rect, &self.settings.video) {
                    Ok(video) => {
                        self.video = Some(video);
                        self.install_mouse_hook();
                        if !self.pinned {
                            self.install_keyboard_hook();
                            ShowWindow(self.overlay, SW_HIDE);
                        }
                    }
                    Err(message) => {
                        set_overlay_model(Some(OverlayModel {
                            content: OverlayContent::Text {
                                title: "Video preview unavailable".to_string(),
                                subtitle: String::new(),
                                lines: vec![message],
                                truncated: false,
                            },
                            current_frame: 0,
                            scroll_offset: 0,
                        }));
                        self.place_overlay(rect);
                        self.install_mouse_hook();
                    }
                }
            }
            PreviewPayload::Audio {
                path: audio_path,
                title,
                subtitle,
            } => {
                configure_overlay_for_panel(self.overlay);
                match start_audio_playback(&audio_path) {
                    Ok(audio) => {
                        self.audio = Some(audio);
                        set_overlay_model(Some(OverlayModel {
                            content: OverlayContent::Text {
                                title,
                                subtitle,
                                lines: vec![
                                    "Audio preview playing".to_string(),
                                    "Close the preview or press Escape to stop playback."
                                        .to_string(),
                                ],
                                truncated: false,
                            },
                            current_frame: 0,
                            scroll_offset: 0,
                        }));
                        self.place_overlay(rect);
                        self.install_mouse_hook();
                    }
                    Err(message) => {
                        set_overlay_model(Some(OverlayModel {
                            content: OverlayContent::Text {
                                title: "Audio preview unavailable".to_string(),
                                subtitle: String::new(),
                                lines: vec![message],
                                truncated: false,
                            },
                            current_frame: 0,
                            scroll_offset: 0,
                        }));
                        self.place_overlay(rect);
                        self.install_mouse_hook();
                    }
                }
            }
        }
        // Pinning adds a caption/border. A same-outer-size payload replacement
        // need not generate WM_SIZE, so use the actual client viewport here.
        resize_overlay_view(self.overlay);
        update_preview_chrome(self.overlay, &path, self.pinned, false, "");
        self.current_path = Some(path);
        self.state = if self.pinned {
            HoverState::PreviewLocked
        } else {
            HoverState::PreviewVisible
        };
        self.sync_pinned_media();
    }

    unsafe fn place_overlay(&self, rect: HoverRect) {
        if self.pinned && IsIconic(self.overlay) != 0 {
            // Completion and player-exit notifications must not restore a
            // window the user minimized while work was pending.
            InvalidateRect(self.overlay, null(), 0);
            return;
        }
        SetWindowPos(
            self.overlay,
            HWND_TOPMOST,
            rect.left.round() as i32,
            rect.top.round() as i32,
            rect.width().round().max(1.0) as i32,
            rect.height().round().max(1.0) as i32,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
        ShowWindow(self.overlay, SW_SHOWNOACTIVATE);
        InvalidateRect(self.overlay, null(), 1);
    }

    unsafe fn hide_preview(&mut self) {
        self.preview_error = None;
        self.pinned = false;
        PREVIEW_PINNED.store(false, Ordering::Release);
        self.preview_anchor = None;
        self.reload_view = None;
        self.glance_rect = None;
        set_overlay_pinned_style(self.overlay, false);
        self.media_ready = None;
        KillTimer(self.hwnd, TIMER_MEDIA);
        KillTimer(self.hwnd, TIMER_ZOOM_SETTLE);
        KillTimer(self.hwnd, TIMER_ANIMATION);
        self.uninstall_preview_hooks();
        self.stop_video();
        self.stop_audio();
        self.current_path = None;
        set_overlay_model(None);
        ShowWindow(self.overlay, SW_HIDE);
    }

    unsafe fn advance_animation_frame(&mut self) {
        if let Some(delay) = advance_overlay_frame_and_delay_ms() {
            InvalidateRect(self.overlay, null(), 0);
            SetTimer(self.hwnd, TIMER_ANIMATION, delay, None);
        } else {
            KillTimer(self.hwnd, TIMER_ANIMATION);
        }
    }

    unsafe fn stop_video(&mut self) {
        if let Some(mut video) = self.video.take() {
            let _ = video.process.kill();
            let _ = video.process.wait();
        }
        clear_video_window_state();
    }

    unsafe fn stop_audio(&mut self) {
        if let Some(mut audio) = self.audio.take() {
            let _ = audio.process.kill();
            let _ = audio.process.wait();
        }
    }

    unsafe fn maintain_active_video(&mut self) {
        let stopped = if let Some(video) = self.video.as_mut() {
            match video.process.try_wait() {
                Ok(Some(status)) => Some((video.rect, format!("Video player exited: {status}"))),
                Err(error) => Some((video.rect, format!("Video player error: {error}"))),
                Ok(None) => {
                    // A minimized pinned preview must not repeatedly reveal the
                    // external player before sync_pinned_media hides it again.
                    if !self.pinned || IsIconic(self.overlay) == 0 {
                        sync_video_window(video);
                    }
                    None
                }
            }
        } else {
            None
        };

        if let Some((rect, message)) = stopped {
            self.video = None;
            clear_video_window_state();
            self.uninstall_keyboard_hook();
            self.show_player_stopped(rect, "Video preview stopped", message);
        }
    }

    unsafe fn show_player_stopped(&mut self, rect: HoverRect, title: &str, message: String) {
        configure_overlay_for_panel(self.overlay);
        set_overlay_model(Some(OverlayModel {
            content: OverlayContent::Text {
                title: title.to_string(),
                subtitle: String::new(),
                lines: vec![message, "Reload the preview to retry playback.".to_string()],
                truncated: false,
            },
            current_frame: 0,
            scroll_offset: 0,
        }));
        self.place_overlay(if self.pinned {
            overlay_screen_rect(self.overlay)
        } else {
            rect
        });
        self.install_mouse_hook();
    }

    unsafe fn maintain_active_audio(&mut self) {
        let Some(audio) = self.audio.as_mut() else {
            return;
        };
        match audio.process.try_wait() {
            Ok(Some(_)) | Err(_) => {
                self.audio = None;
            }
            Ok(None) => {}
        }
    }

    unsafe fn install_mouse_hook(&mut self) {
        if !self.mouse_hook.is_null() {
            return;
        }
        let module = GetModuleHandleW(null());
        let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), module, 0);
        if !hook.is_null() {
            self.mouse_hook = hook;
            self.counters.wheel_hooks = 1;
        }
    }

    unsafe fn install_keyboard_hook(&mut self) {
        if !self.keyboard_hook.is_null() {
            return;
        }
        let module = GetModuleHandleW(null());
        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), module, 0);
        if !hook.is_null() {
            self.keyboard_hook = hook;
        }
    }

    unsafe fn uninstall_keyboard_hook(&mut self) {
        if !self.keyboard_hook.is_null() {
            UnhookWindowsHookEx(self.keyboard_hook);
            self.keyboard_hook = null_mut();
        }
    }

    unsafe fn uninstall_preview_hooks(&mut self) {
        if !self.mouse_hook.is_null() {
            UnhookWindowsHookEx(self.mouse_hook);
            self.mouse_hook = null_mut();
        }
        self.uninstall_keyboard_hook();
        self.counters.wheel_hooks = 0;
    }

    unsafe fn cursor_inside_active_preview(&self) -> bool {
        cursor_inside_window(self.overlay)
            || self
                .video
                .as_ref()
                .map(|video| cursor_inside_rect(video.rect))
                .unwrap_or(false)
            || cursor_inside_video_window()
    }

    unsafe fn keep_active_preview_during_media_foreground(&self) -> bool {
        let Some(video) = self.video.as_ref() else {
            return false;
        };
        if video.started.elapsed() < Duration::from_millis(VIDEO_STARTUP_GRACE_MS) {
            return true;
        }
        foreground_process_id().is_some_and(|pid| pid == video.process.id())
    }

    unsafe fn handle_preview_cancel_input(&mut self) -> bool {
        if self.current_path.is_none() {
            return false;
        }
        if key_pressed_once(VK_ESCAPE) {
            self.dismissed_path = self.current_path.clone();
            self.cancel_decode();
            self.hide_preview();
            self.candidate_path = None;
            self.state = HoverState::ForegroundEligible;
            return true;
        }
        if key_pressed_once(VK_LBUTTON) && !self.cursor_inside_active_preview() {
            self.dismissed_path = self.current_path.clone();
            self.cancel_decode();
            self.hide_preview();
            self.candidate_path = None;
            self.state = HoverState::ForegroundEligible;
            return true;
        }
        false
    }

    unsafe fn toggle_enabled(&mut self) {
        self.settings.general.enabled = !self.settings.general.enabled;
        if self.settings.general.enabled {
            self.on_foreground_changed();
        } else {
            self.enter_dormant();
        }
    }

    unsafe fn toggle_pin(&mut self) {
        self.toggle_pin_with_activation(true);
    }

    unsafe fn toggle_pin_with_activation(&mut self, activate: bool) {
        let Some(path) = self.current_path.clone() else {
            return;
        };
        if !self.pinned {
            self.glance_rect = Some(overlay_screen_rect(self.overlay));
        }
        self.pinned = !self.pinned;
        PREVIEW_PINNED.store(self.pinned, Ordering::Release);
        set_overlay_pinned_style(self.overlay, self.pinned);
        update_preview_chrome(
            self.overlay,
            &path,
            self.pinned,
            self.pending_path.is_some(),
            "",
        );
        if self.pinned {
            KillTimer(self.hwnd, TIMER_CURSOR);
            self.uninstall_keyboard_hook();
            self.state = HoverState::PreviewLocked;
            ShowWindow(self.overlay, SW_SHOWNOACTIVATE);
            if activate {
                SetForegroundWindow(self.overlay);
                SetFocus(self.overlay);
            }
            self.sync_pinned_media();
        } else {
            KillTimer(self.hwnd, TIMER_MEDIA);
            if let Some(rect) = self.glance_rect.take() {
                let cursor = current_cursor_point();
                self.place_overlay(place_preview(
                    cursor,
                    screen_work_area(cursor),
                    rect.size(),
                    PlacementConfig::default(),
                ));
            }
            if GetAncestor(GetForegroundWindow(), GA_ROOT) == self.overlay
                && IsWindow(self.origin_foreground) != 0
            {
                SetForegroundWindow(self.origin_foreground);
            }
            self.on_foreground_changed();
            if self.agent_controlled {
                self.state = HoverState::PreviewVisible;
            }
        }
    }

    unsafe fn reload_current(&mut self) {
        if self.pending_path.is_some() {
            return;
        }
        let Some(path) = self.current_path.clone() else {
            return;
        };
        let saved = if self.preview_error.is_some() {
            self.reload_view
                .take()
                .filter(|(identity, _)| identity == &path)
                .map(|(_, view)| view)
        } else {
            OVERLAY_MODEL
                .get_or_init(|| Mutex::new(None))
                .lock()
                .unwrap()
                .as_ref()
                .map(OverlayModel::save_view)
        };
        self.cancel_decode();
        if let Some(saved) = saved {
            self.reload_view = Some((path.clone(), saved));
        }
        self.start_decode(path);
        if self.pinned {
            KillTimer(self.hwnd, TIMER_CURSOR);
        }
    }

    unsafe fn dismiss_current(&mut self) {
        self.dismissed_path = self.current_path.clone();
        self.cancel_decode();
        self.hide_preview();
        self.candidate_path = None;
        self.on_foreground_changed();
        if self.agent_controlled {
            self.state = HoverState::Dormant;
        }
    }

    unsafe fn sync_pinned_media(&mut self) {
        if !self.pinned {
            return;
        }
        if let Some(video) = self.video.as_mut() {
            if IsIconic(self.overlay) != 0 {
                let hwnd = current_video_window();
                if !hwnd.is_null() {
                    ShowWindow(hwnd, SW_HIDE);
                }
            } else {
                video.rect = overlay_media_rect(self.overlay);
                store_video_control_state(video.rect, self.settings.video.seek_seconds);
                // Re-evaluate placement on move/resize, without a cursor poll.
                ensure_video_window_topmost(video.rect);
            }
        }
        if self.video.is_some() || self.audio.is_some() {
            SetTimer(self.hwnd, TIMER_MEDIA, 250, None);
        } else {
            KillTimer(self.hwnd, TIMER_MEDIA);
        }
        if IsIconic(self.overlay) != 0 {
            KillTimer(self.hwnd, TIMER_ANIMATION);
        } else if let Some(delay) = current_overlay_frame_delay_ms() {
            SetTimer(self.hwnd, TIMER_ANIMATION, delay, None);
        }
    }

    unsafe fn reposition_pinned(&mut self) {
        if !self.pinned {
            self.enter_dormant();
            return;
        }
        let old = overlay_screen_rect(self.overlay);
        let center = HoverPoint::new((old.left + old.right) * 0.5, (old.top + old.bottom) * 0.5);
        let work = screen_work_area(center);
        let width = old.width().min(work.width()).max(1.0);
        let height = old.height().min(work.height()).max(1.0);
        let left = old.left.clamp(work.left, work.right - width);
        let top = old.top.clamp(work.top, work.bottom - height);
        self.place_overlay(HoverRect::new(left, top, left + width, top + height));
        self.sync_pinned_media();
    }

    unsafe fn toggle_startup(&mut self) {
        let enabled = !self.settings.general.start_with_windows;
        self.settings.general.start_with_windows = set_startup_enabled(enabled);
    }

    unsafe fn show_options(&mut self) {
        show_options_window(self.hwnd, &self.settings);
    }

    unsafe fn destroy(&mut self) {
        CONTROL_HWND.store(0, Ordering::Release);
        self.enter_dormant();
        OVERLAY_HWND.store(0, Ordering::Release);
        if IsWindow(self.overlay) != 0 {
            DestroyWindow(self.overlay);
        }
        let options = OPTIONS_HWND.swap(0, Ordering::AcqRel) as HWND;
        if !options.is_null() {
            DestroyWindow(options);
        }
        if self.tray_added {
            remove_tray_icon(self.hwnd);
            self.tray_added = false;
        }
        if self.tray_icon_owned && !self.tray_icon.is_null() {
            DestroyIcon(self.tray_icon);
            self.tray_icon = null_mut();
            self.tray_icon_owned = false;
        }
    }
}

unsafe fn post_preview_action(action: usize) {
    let control = CONTROL_HWND.load(Ordering::Acquire) as HWND;
    if !control.is_null() {
        PostMessageW(control, WM_PREVIEW_ACTION, action, 0);
    }
}

unsafe fn content_client_rect(hwnd: HWND) -> RECT {
    let mut rect = RECT::default();
    GetClientRect(hwnd, &mut rect);
    rect.right = rect.right.max(1);
    rect.bottom = rect.bottom.max(1);
    rect.top = PREVIEW_CHROME_HEIGHT.min(rect.bottom - 1);
    rect
}

unsafe fn overlay_screen_rect(hwnd: HWND) -> HoverRect {
    let mut rect = RECT::default();
    GetWindowRect(hwnd, &mut rect);
    HoverRect::new(
        rect.left as f32,
        rect.top as f32,
        rect.right.max(rect.left + 1) as f32,
        rect.bottom.max(rect.top + 1) as f32,
    )
}

unsafe fn overlay_media_rect(hwnd: HWND) -> HoverRect {
    let rect = content_client_rect(hwnd);
    let mut origin = POINT {
        x: rect.left,
        y: rect.top,
    };
    ClientToScreen(hwnd, &mut origin);
    HoverRect::new(
        origin.x as f32,
        origin.y as f32,
        (origin.x + rect.right - rect.left) as f32,
        (origin.y + rect.bottom - rect.top) as f32,
    )
}

unsafe fn set_overlay_pinned_style(hwnd: HWND, pinned: bool) {
    if IsWindow(hwnd) == 0 {
        return;
    }
    let visible = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 & WS_VISIBLE;
    let style = if pinned {
        WS_OVERLAPPEDWINDOW
    } else {
        WS_POPUP
    } | visible
        | WS_CLIPCHILDREN;
    let extended = WS_EX_LAYERED
        | WS_EX_TOPMOST
        | if pinned {
            WS_EX_APPWINDOW
        } else {
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE
        };
    SetWindowLongPtrW(hwnd, GWL_STYLE, style as isize);
    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, extended as isize);
    SetWindowPos(
        hwnd,
        HWND_TOPMOST,
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
    );
}

unsafe fn create_preview_chrome(hwnd: HWND) {
    let font = GetStockObject(DEFAULT_GUI_FONT);
    for (id, class, text, extra) in [
        (
            ID_PREVIEW_TITLE as usize,
            "STATIC",
            "Hover",
            SS_CENTERIMAGE | SS_ENDELLIPSIS | SS_NOPREFIX,
        ),
        (
            ID_PREVIEW_PIN,
            "BUTTON",
            "Pin",
            BS_PUSHBUTTON as u32 | WS_TABSTOP,
        ),
        (
            ID_PREVIEW_RELOAD,
            "BUTTON",
            "Reload",
            BS_PUSHBUTTON as u32 | WS_TABSTOP,
        ),
        (
            ID_PREVIEW_CLOSE,
            "BUTTON",
            "Close",
            BS_PUSHBUTTON as u32 | WS_TABSTOP,
        ),
    ] {
        let child = CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | extra,
            0,
            6,
            1,
            28,
            hwnd,
            id as *mut _,
            GetModuleHandleW(null()),
            null_mut(),
        );
        if !child.is_null() {
            SendMessageW(child, WM_SETFONT, font as usize, 1);
        }
    }
}

unsafe fn layout_preview_chrome(hwnd: HWND) {
    let mut rect = RECT::default();
    GetClientRect(hwnd, &mut rect);
    let mut right = rect.right - 8;
    for (id, width) in [
        (ID_PREVIEW_CLOSE, 52),
        (ID_PREVIEW_RELOAD, 66),
        (ID_PREVIEW_PIN, 66),
    ] {
        let child = GetDlgItem(hwnd, id as i32);
        let left = right - width;
        if left < 8 {
            ShowWindow(child, SW_HIDE);
        } else {
            SetWindowPos(child, null_mut(), left, 6, width, 28, SWP_NOACTIVATE);
            ShowWindow(child, SW_SHOWNOACTIVATE);
            right = left - 6;
        }
    }
    let title = GetDlgItem(hwnd, ID_PREVIEW_TITLE);
    SetWindowPos(
        title,
        null_mut(),
        12,
        6,
        (right - 20).max(1),
        28,
        SWP_NOACTIVATE,
    );
    ShowWindow(
        title,
        if right > 32 {
            SW_SHOWNOACTIVATE
        } else {
            SW_HIDE
        },
    );
}

unsafe fn update_preview_chrome(hwnd: HWND, path: &Path, pinned: bool, busy: bool, status: &str) {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let detail = if busy { "Reloading / loading" } else { status };
    let title = if detail.is_empty() {
        name.to_string()
    } else {
        format!("{name} · {detail}")
    };
    SetWindowTextW(
        hwnd,
        wide(&format!(
            "{title} · Hover{}",
            if pinned { " (Pinned)" } else { "" }
        ))
        .as_ptr(),
    );
    SetWindowTextW(GetDlgItem(hwnd, ID_PREVIEW_TITLE), wide(&title).as_ptr());
    SetWindowTextW(
        GetDlgItem(hwnd, ID_PREVIEW_PIN as i32),
        wide(if pinned { "Unpin" } else { "Pin" }).as_ptr(),
    );
    EnableWindow(GetDlgItem(hwnd, ID_PREVIEW_RELOAD as i32), i32::from(!busy));
    layout_preview_chrome(hwnd);
}

unsafe fn resize_overlay_view(hwnd: HWND) {
    if IsIconic(hwnd) != 0 {
        return;
    }
    let rect = content_client_rect(hwnd);
    if let Some(model) = OVERLAY_MODEL
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap()
        .as_mut()
    {
        match &mut model.content {
            OverlayContent::Frames { zoom, .. } => zoom.set_viewport(
                Size::new(
                    (rect.right - rect.left) as f32,
                    (rect.bottom - rect.top) as f32,
                ),
                ZoomConfig::default(),
            ),
            OverlayContent::Text {
                lines, truncated, ..
            } => {
                model.scroll_offset = model.scroll_offset.min(
                    text_virtual_line_count(lines, *truncated)
                        .saturating_sub(text_visible_rows(&rect)),
                )
            }
            OverlayContent::List { entries, .. } => {
                model.scroll_offset = model
                    .scroll_offset
                    .min(entries.len().saturating_sub(list_visible_rows(&rect)))
            }
        }
    }
    InvalidateRect(hwnd, null(), 0);
}

unsafe fn restore_overlay_view(hwnd: HWND, saved: SavedView) -> bool {
    let rect = content_client_rect(hwnd);
    let mut guard = OVERLAY_MODEL
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap();
    let Some(model) = guard.as_mut() else {
        return false;
    };
    let rows = if matches!(model.content, OverlayContent::List { .. }) {
        list_visible_rows(&rect)
    } else {
        text_visible_rows(&rect)
    };
    let viewport = match saved {
        SavedView::Image { zoom, .. } if IsIconic(hwnd) != 0 => zoom.viewport_size,
        _ => Size::new(
            (rect.right - rect.left) as f32,
            (rect.bottom - rect.top) as f32,
        ),
    };
    model.restore_view(saved, viewport, rows)
}

unsafe fn preprocess_preview_message(msg: &MSG) -> bool {
    let hwnd = OVERLAY_HWND.load(Ordering::Acquire) as HWND;
    if hwnd.is_null()
        || !PREVIEW_PINNED.load(Ordering::Acquire)
        || GetAncestor(msg.hwnd, GA_ROOT) != hwnd
    {
        return false;
    }
    if msg.message == WM_KEYDOWN {
        if msg.wParam == 0x09 {
            // Include content in the tab order; IsDialogMessage otherwise
            // cycles only the native child buttons, trapping keyboard users.
            cycle_preview_focus(hwnd, GetKeyState(VK_SHIFT as i32) < 0);
            return true;
        }
        let control = GetKeyState(VK_CONTROL as i32) < 0;
        let action = match msg.wParam {
            key if key == VK_ESCAPE as usize => Some(ID_PREVIEW_CLOSE),
            0x74 => Some(ID_PREVIEW_RELOAD), // F5
            0x52 if control => Some(ID_PREVIEW_RELOAD),
            0x50 if control => Some(ID_PREVIEW_PIN),
            _ => None,
        };
        if let Some(action) = action {
            post_preview_action(action);
            return true;
        }
        if msg.hwnd == hwnd && preview_content_key(hwnd, msg.wParam as u32) {
            return true;
        }
    }
    IsDialogMessageW(hwnd, msg) != 0
}

unsafe fn cycle_preview_focus(hwnd: HWND, backwards: bool) {
    let targets = [
        hwnd,
        GetDlgItem(hwnd, ID_PREVIEW_PIN as i32),
        GetDlgItem(hwnd, ID_PREVIEW_RELOAD as i32),
        GetDlgItem(hwnd, ID_PREVIEW_CLOSE as i32),
    ];
    let focused = GetFocus();
    let current = targets
        .iter()
        .position(|target| *target == focused)
        .unwrap_or(0);
    for step in 1..=targets.len() {
        let index = if backwards {
            (current + targets.len() - step) % targets.len()
        } else {
            (current + step) % targets.len()
        };
        let target = targets[index];
        if !target.is_null() && IsWindowVisible(target) != 0 && IsWindowEnabled(target) != 0 {
            SetFocus(target);
            return;
        }
    }
}

unsafe fn preview_content_key(hwnd: HWND, key: u32) -> bool {
    if VIDEO_PID.load(Ordering::Acquire) != 0 && is_video_control_key(key) {
        return post_video_key_tap(key);
    }
    if matches!(key, 0x0d | 0x20) {
        let control = CONTROL_HWND.load(Ordering::Acquire) as HWND;
        if !control.is_null() {
            PostMessageW(control, WM_MEDIA_PLAY, 0, 0);
            return true;
        }
    }
    let rect = content_client_rect(hwnd);
    let mut guard = OVERLAY_MODEL
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap();
    let Some(model) = guard.as_mut() else {
        return false;
    };
    let rows = match &mut model.content {
        OverlayContent::Frames { zoom, .. } => {
            match key {
                0x25 => zoom.pan_by(48.0, 0.0, ZoomConfig::default()),
                0x27 => zoom.pan_by(-48.0, 0.0, ZoomConfig::default()),
                0x26 => zoom.pan_by(0.0, 48.0, ZoomConfig::default()),
                0x28 => zoom.pan_by(0.0, -48.0, ZoomConfig::default()),
                0x30 | 0x60 => zoom.set_mode(hover_core::ScaleMode::FitAll, ZoomConfig::default()),
                0x31 | 0x61 => zoom.set_mode(hover_core::ScaleMode::Native, ZoomConfig::default()),
                0xbb | 0x6b | 0xbd | 0x6d => zoom.apply_wheel(
                    HoverPoint::new(
                        zoom.viewport_size.width * 0.5,
                        zoom.viewport_size.height * 0.5,
                    ),
                    if matches!(key, 0xbb | 0x6b) {
                        1.0
                    } else {
                        -1.0
                    },
                    WheelModifier::Plain,
                    ZoomConfig::default(),
                ),
                _ => return false,
            }
            zoom.settle_quality();
            InvalidateRect(hwnd, null(), 0);
            return true;
        }
        OverlayContent::Text {
            lines, truncated, ..
        } => (
            text_virtual_line_count(lines, *truncated),
            text_visible_rows(&rect),
        ),
        OverlayContent::List { entries, .. } => (entries.len(), list_visible_rows(&rect)),
    };
    let max = rows.0.saturating_sub(rows.1);
    model.scroll_offset = match key {
        0x26 => model.scroll_offset.saturating_sub(1),
        0x28 => model.scroll_offset.saturating_add(1).min(max),
        0x21 => model.scroll_offset.saturating_sub(rows.1.max(1)),
        0x22 => model.scroll_offset.saturating_add(rows.1.max(1)).min(max),
        0x24 => 0,
        0x23 => max,
        _ => return false,
    };
    InvalidateRect(hwnd, null(), 0);
    true
}

unsafe extern "system" fn foreground_event_proc(
    _hook: HWINEVENTHOOK,
    _event: u32,
    _hwnd: HWND,
    _object_id: i32,
    _child_id: i32,
    _thread_id: u32,
    _event_time: u32,
) {
    let hwnd = CONTROL_HWND.load(Ordering::Acquire) as HWND;
    if !hwnd.is_null() {
        windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW(
            hwnd,
            WM_HOVER_FOREGROUND_CHANGED,
            0,
            0,
        );
    }
}

unsafe extern "system" fn control_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let ptr = windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
        hwnd,
        windows_sys::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
    ) as *mut App;
    match msg {
        crate::resident::MESSAGE => {
            if let Some(app) = ptr.as_mut()
                && !app.agent_controlled
            {
                if wparam == 0 {
                    PostQuitMessage(0);
                } else if wparam == 1 {
                    app.show_options();
                }
            }
            0
        }
        crate::agent::WM_AGENT_REQUEST => {
            if let Some(app) = ptr.as_mut() {
                let request = app
                    .agent_requests
                    .as_ref()
                    .and_then(|rx| rx.try_recv().ok());
                if let Some(request) = request {
                    let result = app.execute_agent_command(request.command);
                    let _ = request.reply.try_send(result);
                }
            }
            0
        }
        WM_MEDIA_PLAY => {
            if let Some(app) = ptr.as_mut() {
                app.play_ready_media();
            }
            0
        }
        WM_PREVIEW_ACTION => {
            if let Some(app) = ptr.as_mut() {
                match wparam {
                    ID_PREVIEW_PIN => app.toggle_pin(),
                    ID_PREVIEW_RELOAD => app.reload_current(),
                    ID_PREVIEW_CLOSE => app.dismiss_current(),
                    ID_PREVIEW_SYNC => app.sync_pinned_media(),
                    _ => {}
                }
            }
            0
        }
        WM_PREVIEW_READY => {
            if let Some(app) = ptr.as_mut() {
                if app.state != HoverState::Dormant {
                    app.update_hover_target();
                }
                app.poll_decode_result();
            }
            0
        }
        WM_HOVER_FOREGROUND_CHANGED => {
            if let Some(app) = ptr.as_mut() {
                app.on_foreground_changed();
            }
            0
        }
        WM_TIMER => {
            if let Some(app) = ptr.as_mut() {
                app.on_timer(wparam);
            }
            0
        }
        WM_TRAYICON => {
            let event = lparam as u32;
            if (event == WM_RBUTTONUP || event == WM_LBUTTONUP)
                && !ptr.is_null()
                && let Some(app) = ptr.as_ref()
            {
                let snapshot = (
                    app.settings.general.enabled,
                    app.settings.general.start_with_windows,
                    app.pinned,
                    app.current_path.is_some(),
                );
                show_tray_menu(hwnd, snapshot);
            }
            0
        }
        WM_COMMAND => {
            if let Some(app) = ptr.as_mut() {
                match wparam & 0xffff {
                    ID_TRAY_ENABLE => app.toggle_enabled(),
                    ID_TRAY_STARTUP => app.toggle_startup(),
                    ID_TRAY_OPTIONS => app.show_options(),
                    ID_PREVIEW_PIN => app.toggle_pin(),
                    ID_PREVIEW_RELOAD => app.reload_current(),
                    ID_PREVIEW_CLOSE => app.dismiss_current(),
                    ID_TRAY_EXIT => PostQuitMessage(0),
                    _ => {}
                }
            }
            0
        }
        WM_DISPLAYCHANGE | WM_DPICHANGED => {
            if let Some(app) = ptr.as_mut() {
                app.reposition_pinned();
            }
            0
        }
        WM_POWERBROADCAST => {
            if let Some(app) = ptr.as_mut() {
                app.enter_dormant();
            }
            0
        }
        WM_DESTROY => {
            if !ptr.is_null() {
                SetWindowLongPtrW(
                    hwnd,
                    windows_sys::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
                    0,
                );
                let mut app = Box::from_raw(ptr);
                app.destroy();
            }
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe extern "system" fn overlay_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_COMMAND => {
            if matches!(
                wparam & 0xffff,
                ID_PREVIEW_PIN | ID_PREVIEW_RELOAD | ID_PREVIEW_CLOSE
            ) {
                post_preview_action(wparam & 0xffff);
                return 0;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_CLOSE => {
            post_preview_action(ID_PREVIEW_CLOSE);
            0
        }
        WM_SIZE | WM_MOVE => {
            layout_preview_chrome(hwnd);
            resize_overlay_view(hwnd);
            post_preview_action(ID_PREVIEW_SYNC);
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_MOUSEACTIVATE if !PREVIEW_PINNED.load(Ordering::Acquire) => MA_NOACTIVATE as LRESULT,
        WM_LBUTTONDOWN if PREVIEW_PINNED.load(Ordering::Acquire) => {
            SetFocus(hwnd);
            0
        }
        WM_CTLCOLORSTATIC => {
            let hdc = wparam as windows_sys::Win32::Graphics::Gdi::HDC;
            SetTextColor(hdc, rgb(241, 242, 244));
            SetBkColor(hdc, rgb(20, 22, 25));
            SetDCBrushColor(hdc, rgb(20, 22, 25));
            GetStockObject(DC_BRUSH) as LRESULT
        }
        WM_LBUTTONUP => {
            if signed_hiword_isize(lparam) < PREVIEW_CHROME_HEIGHT {
                return 0;
            }
            let control = CONTROL_HWND.load(Ordering::Acquire) as HWND;
            if !control.is_null() {
                PostMessageW(control, WM_MEDIA_PLAY, 0, 0);
            }
            0
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            paint_overlay(hwnd, hdc, &ps.rcPaint);
            EndPaint(hwnd, &ps);
            0
        }
        WM_MOUSEWHEEL => match apply_overlay_wheel(hwnd, wparam, lparam) {
            OverlayWheelAction::Zoom => {
                schedule_zoom_settle(hwnd);
                0
            }
            OverlayWheelAction::Repaint => {
                InvalidateRect(hwnd, null(), 0);
                0
            }
            OverlayWheelAction::None => DefWindowProcW(hwnd, msg, wparam, lparam),
        },
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe extern "system" fn mouse_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 && wparam as u32 == WM_MOUSEWHEEL && lparam != 0 {
        let mouse = &*(lparam as *const MSLLHOOKSTRUCT);
        let wheel_delta = signed_hiword_u32(mouse.mouseData);
        if VIDEO_PID.load(Ordering::Acquire) != 0
            && point_inside_video_rect(mouse.pt)
            && seek_active_video_by_wheel(wheel_delta.into())
        {
            return 1;
        }
        let overlay = OVERLAY_HWND.load(Ordering::Acquire) as HWND;
        if !overlay.is_null() {
            match apply_overlay_wheel_at_screen(overlay, mouse.pt, wheel_delta) {
                OverlayWheelAction::Zoom => {
                    schedule_zoom_settle(overlay);
                    return 1;
                }
                OverlayWheelAction::Repaint => {
                    InvalidateRect(overlay, null(), 0);
                    return 1;
                }
                OverlayWheelAction::None => {}
            }
        }
    }
    CallNextHookEx(null_mut(), code, wparam, lparam)
}

unsafe extern "system" fn keyboard_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 && lparam != 0 {
        let message = wparam as u32;
        if matches!(message, WM_KEYDOWN | WM_KEYUP | WM_SYSKEYDOWN | WM_SYSKEYUP) {
            let keyboard = &*(lparam as *const KBDLLHOOKSTRUCT);
            if is_video_control_key(keyboard.vkCode) && forward_video_key(message, keyboard) {
                return 1;
            }
        }
    }
    CallNextHookEx(null_mut(), code, wparam, lparam)
}

unsafe fn schedule_zoom_settle(hwnd: HWND) {
    let control_hwnd = CONTROL_HWND.load(Ordering::Acquire) as HWND;
    if !control_hwnd.is_null() {
        SetTimer(control_hwnd, TIMER_ZOOM_SETTLE, ZOOM_SETTLE_TIMER_MS, None);
    }
    InvalidateRect(hwnd, null(), 0);
}

unsafe extern "system" fn options_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let id = (wparam & 0xffff) as i32;
            if id == ID_OPTION_CLOSE {
                DestroyWindow(hwnd);
                return 0;
            }
            if id == ID_OPTION_SEEK_MINUS {
                adjust_video_seek_interval(hwnd, -1);
                return 0;
            }
            if id == ID_OPTION_SEEK_PLUS {
                adjust_video_seek_interval(hwnd, 1);
                return 0;
            }
            if is_option_toggle_id(id) {
                apply_options_from_window(hwnd);
                return 0;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            OPTIONS_HWND.store(0, Ordering::Release);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn show_options_window(owner: HWND, settings: &Settings) {
    let existing = OPTIONS_HWND.load(Ordering::Acquire) as HWND;
    if !existing.is_null() {
        ShowWindow(existing, SW_SHOWNOACTIVATE);
        SetForegroundWindow(existing);
        sync_options_window(existing, settings);
        return;
    }
    let hinstance = GetModuleHandleW(null());
    let hwnd = CreateWindowExW(
        WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
        wide(OPTIONS_CLASS).as_ptr(),
        wide("Hover Options").as_ptr(),
        WS_POPUP | WS_CAPTION | WS_SYSMENU,
        240,
        180,
        460,
        500,
        owner,
        null_mut(),
        hinstance,
        null_mut(),
    );
    if hwnd.is_null() {
        return;
    }
    OPTIONS_HWND.store(hwnd as isize, Ordering::Release);
    create_static_label(hwnd, "Hover preview types", 24, 22, 320, 24);
    create_static_label(
        hwnd,
        "Toggle exactly what Hover is allowed to preview on stable hover.",
        24,
        48,
        360,
        22,
    );
    create_option_checkbox(hwnd, ID_OPTION_IMAGES, "Images and animated GIF", 34, 88);
    create_option_checkbox(hwnd, ID_OPTION_VIDEOS, "Videos", 34, 120);
    create_option_checkbox(hwnd, ID_OPTION_AUDIO, "Audio and music", 34, 152);
    create_option_checkbox(hwnd, ID_OPTION_FOLDERS, "Folders", 34, 184);
    create_option_checkbox(hwnd, ID_OPTION_ZIPS, "ZIP archives", 34, 216);
    create_option_checkbox(hwnd, ID_OPTION_TEXT, "Text, Markdown, README, NFO", 34, 248);
    create_option_checkbox(hwnd, ID_OPTION_SIMS, "Sims 4 .package warehouse", 34, 280);
    create_static_label(hwnd, "Video playback", 24, 318, 320, 24);
    create_option_checkbox(hwnd, ID_OPTION_VIDEO_AUDIO, "Play video audio", 34, 350);
    create_static_label_with_id(
        hwnd,
        ID_OPTION_SEEK_LABEL,
        "Video seek step: 5 seconds",
        34,
        388,
        230,
        24,
    );
    CreateWindowExW(
        0,
        wide("BUTTON").as_ptr(),
        wide("-").as_ptr(),
        WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON as u32,
        272,
        384,
        44,
        30,
        hwnd,
        ID_OPTION_SEEK_MINUS as isize as _,
        hinstance,
        null_mut(),
    );
    CreateWindowExW(
        0,
        wide("BUTTON").as_ptr(),
        wide("+").as_ptr(),
        WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON as u32,
        326,
        384,
        44,
        30,
        hwnd,
        ID_OPTION_SEEK_PLUS as isize as _,
        hinstance,
        null_mut(),
    );
    CreateWindowExW(
        0,
        wide("BUTTON").as_ptr(),
        wide("Close").as_ptr(),
        WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON as u32,
        324,
        430,
        96,
        30,
        hwnd,
        ID_OPTION_CLOSE as isize as _,
        hinstance,
        null_mut(),
    );
    sync_options_window(hwnd, settings);
    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    SetForegroundWindow(hwnd);
}

unsafe fn create_static_label_with_id(
    hwnd: HWND,
    id: i32,
    text: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) {
    CreateWindowExW(
        0,
        wide("STATIC").as_ptr(),
        wide(text).as_ptr(),
        WS_CHILD | WS_VISIBLE,
        x,
        y,
        width,
        height,
        hwnd,
        id as isize as _,
        GetModuleHandleW(null()),
        null_mut(),
    );
}

unsafe fn create_static_label(hwnd: HWND, text: &str, x: i32, y: i32, width: i32, height: i32) {
    CreateWindowExW(
        0,
        wide("STATIC").as_ptr(),
        wide(text).as_ptr(),
        WS_CHILD | WS_VISIBLE,
        x,
        y,
        width,
        height,
        hwnd,
        null_mut(),
        GetModuleHandleW(null()),
        null_mut(),
    );
}

unsafe fn create_option_checkbox(hwnd: HWND, id: i32, text: &str, x: i32, y: i32) {
    CreateWindowExW(
        0,
        wide("BUTTON").as_ptr(),
        wide(text).as_ptr(),
        WS_CHILD | WS_VISIBLE | BS_AUTOCHECKBOX as u32,
        x,
        y,
        320,
        24,
        hwnd,
        id as isize as _,
        GetModuleHandleW(null()),
        null_mut(),
    );
}

unsafe fn sync_options_window(hwnd: HWND, settings: &Settings) {
    set_checkbox(hwnd, ID_OPTION_IMAGES, settings.preview_types.images);
    set_checkbox(hwnd, ID_OPTION_VIDEOS, settings.preview_types.videos);
    set_checkbox(hwnd, ID_OPTION_AUDIO, settings.preview_types.audio);
    set_checkbox(hwnd, ID_OPTION_FOLDERS, settings.preview_types.folders);
    set_checkbox(hwnd, ID_OPTION_ZIPS, settings.preview_types.zips);
    set_checkbox(hwnd, ID_OPTION_TEXT, settings.preview_types.text);
    set_checkbox(hwnd, ID_OPTION_SIMS, settings.preview_types.sims_packages);
    set_checkbox(hwnd, ID_OPTION_VIDEO_AUDIO, settings.video.audio_enabled);
    set_seek_label(hwnd, settings.video.seek_seconds);
}

unsafe fn set_checkbox(hwnd: HWND, id: i32, checked: bool) {
    let control = GetDlgItem(hwnd, id);
    if !control.is_null() {
        SendMessageW(
            control,
            BM_SETCHECK,
            if checked { BST_CHECKED_VALUE } else { 0 },
            0,
        );
    }
}

unsafe fn checkbox_checked(hwnd: HWND, id: i32) -> bool {
    let control = GetDlgItem(hwnd, id);
    !control.is_null() && SendMessageW(control, BM_GETCHECK, 0, 0) == BST_CHECKED_VALUE as isize
}

fn is_option_toggle_id(id: i32) -> bool {
    matches!(
        id,
        ID_OPTION_IMAGES
            | ID_OPTION_VIDEOS
            | ID_OPTION_AUDIO
            | ID_OPTION_FOLDERS
            | ID_OPTION_ZIPS
            | ID_OPTION_TEXT
            | ID_OPTION_SIMS
            | ID_OPTION_VIDEO_AUDIO
    )
}

unsafe fn apply_options_from_window(hwnd: HWND) {
    let Some(app) = control_app_mut() else {
        return;
    };
    app.settings.preview_types.images = checkbox_checked(hwnd, ID_OPTION_IMAGES);
    app.settings.preview_types.videos = checkbox_checked(hwnd, ID_OPTION_VIDEOS);
    app.settings.preview_types.audio = checkbox_checked(hwnd, ID_OPTION_AUDIO);
    app.settings.preview_types.folders = checkbox_checked(hwnd, ID_OPTION_FOLDERS);
    app.settings.preview_types.zips = checkbox_checked(hwnd, ID_OPTION_ZIPS);
    app.settings.preview_types.text = checkbox_checked(hwnd, ID_OPTION_TEXT);
    app.settings.preview_types.sims_packages = checkbox_checked(hwnd, ID_OPTION_SIMS);
    app.settings.video.audio_enabled = checkbox_checked(hwnd, ID_OPTION_VIDEO_AUDIO);
    save_settings(&app.settings);
}

unsafe fn adjust_video_seek_interval(hwnd: HWND, delta: i32) {
    apply_options_from_window(hwnd);
    let Some(app) = control_app_mut() else {
        return;
    };
    let next = (app.settings.video.seek_seconds as i32 + delta).clamp(1, 60) as u32;
    app.settings.video.seek_seconds = next;
    save_settings(&app.settings);
    sync_options_window(hwnd, &app.settings);
}

unsafe fn set_seek_label(hwnd: HWND, seek_seconds: u32) {
    let label = GetDlgItem(hwnd, ID_OPTION_SEEK_LABEL);
    if !label.is_null() {
        let text = if seek_seconds == 1 {
            "Video seek step: 1 second".to_string()
        } else {
            format!("Video seek step: {seek_seconds} seconds")
        };
        SetWindowTextW(label, wide(&text).as_ptr());
    }
}

unsafe fn control_app_mut() -> Option<&'static mut App> {
    let control = CONTROL_HWND.load(Ordering::Acquire) as HWND;
    if control.is_null() {
        return None;
    }
    let ptr = windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
        control,
        windows_sys::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
    ) as *mut App;
    ptr.as_mut()
}

unsafe fn add_tray_icon(hwnd: HWND, icon: HICON) -> bool {
    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: WM_TRAYICON,
        hIcon: icon,
        ..Default::default()
    };
    copy_wide_fixed("Hover", &mut nid.szTip);
    Shell_NotifyIconW(NIM_ADD, &nid) != 0
}

unsafe fn create_hover_tray_icon() -> Option<HICON> {
    const SIZE: usize = 64;
    let (pixels, mask) = crate::branding::icon_image(SIZE);

    let icon = CreateIcon(
        null_mut(),
        SIZE as i32,
        SIZE as i32,
        1,
        32,
        mask.as_ptr(),
        pixels.as_ptr(),
    );
    (!icon.is_null()).then_some(icon)
}

unsafe fn remove_tray_icon(hwnd: HWND) {
    let nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        ..Default::default()
    };
    Shell_NotifyIconW(NIM_DELETE, &nid);
}

unsafe fn show_tray_menu(hwnd: HWND, snapshot: (bool, bool, bool, bool)) {
    let menu = CreatePopupMenu();
    if menu.is_null() {
        return;
    }
    let enabled_flags = MF_STRING | if snapshot.0 { MF_CHECKED } else { MF_UNCHECKED };
    AppendMenuW(
        menu,
        enabled_flags,
        ID_TRAY_ENABLE,
        wide("Enable Hover").as_ptr(),
    );
    let startup_flags = MF_STRING | if snapshot.1 { MF_CHECKED } else { MF_UNCHECKED };
    AppendMenuW(
        menu,
        startup_flags,
        ID_TRAY_STARTUP,
        wide("Run at Startup").as_ptr(),
    );
    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    let preview_flags = MF_STRING | if snapshot.3 { 0 } else { MF_GRAYED };
    AppendMenuW(
        menu,
        preview_flags,
        ID_PREVIEW_PIN,
        wide(if snapshot.2 {
            "Unpin preview"
        } else {
            "Pin current preview"
        })
        .as_ptr(),
    );
    AppendMenuW(
        menu,
        preview_flags,
        ID_PREVIEW_RELOAD,
        wide("Reload preview").as_ptr(),
    );
    AppendMenuW(
        menu,
        preview_flags,
        ID_PREVIEW_CLOSE,
        wide("Close preview").as_ptr(),
    );
    AppendMenuW(
        menu,
        MF_STRING,
        ID_TRAY_OPTIONS,
        wide("Options...").as_ptr(),
    );
    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    AppendMenuW(menu, MF_STRING, ID_TRAY_EXIT, wide("Exit").as_ptr());

    let mut point = POINT::default();
    GetCursorPos(&mut point);
    TRAY_MENU_OPEN.store(true, Ordering::Release);
    SetForegroundWindow(hwnd);
    let command = TrackPopupMenu(
        menu,
        TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RETURNCMD,
        point.x,
        point.y,
        0,
        hwnd,
        null(),
    );
    TRAY_MENU_OPEN.store(false, Ordering::Release);
    DestroyMenu(menu);
    if command != 0 {
        PostMessageW(hwnd, WM_COMMAND, command as usize, 0);
    }
    PostMessageW(hwnd, WM_HOVER_FOREGROUND_CHANGED, 0, 0);
}

unsafe fn create_overlay_window() -> HWND {
    let hinstance = GetModuleHandleW(null());
    let hwnd = CreateWindowExW(
        WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
        wide(OVERLAY_CLASS).as_ptr(),
        wide("Hover Overlay").as_ptr(),
        WS_POPUP | WS_CLIPCHILDREN,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        1,
        1,
        null_mut(),
        null_mut(),
        hinstance,
        null_mut(),
    );
    if !hwnd.is_null() {
        create_preview_chrome(hwnd);
        OVERLAY_HWND.store(hwnd as isize, Ordering::Release);
        SetLayeredWindowAttributes(hwnd, 0, 245, LWA_ALPHA);
        ShowWindow(hwnd, SW_HIDE);
        SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            0,
            0,
            1,
            1,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
        ShowWindow(hwnd, SW_HIDE);
    }
    hwnd
}

unsafe fn configure_overlay_for_image(hwnd: HWND) {
    SetLayeredWindowAttributes(hwnd, TRANSPARENT_IMAGE_COLOR, 255, LWA_COLORKEY);
}

unsafe fn configure_overlay_for_panel(hwnd: HWND) {
    SetLayeredWindowAttributes(hwnd, 0, 245, LWA_ALPHA);
}

fn build_preview_payload(
    path: &Path,
    token: &CancellationToken,
    preview_types: &PreviewTypeSettings,
) -> Result<PreviewPayload, String> {
    if token.is_cancelled() {
        return Err("Preview was cancelled.".to_string());
    }
    let metadata = local_metadata(path).map_err(|error| error.to_string())?;
    if metadata.is_dir() {
        if !preview_types.folders {
            return Err("Folder previews are disabled in Hover options.".to_string());
        }
        return folder_preview(path, token);
    }
    if is_zip_path(path) {
        if !preview_types.zips {
            return Err("ZIP previews are disabled in Hover options.".to_string());
        }
        return zip_preview(path, token);
    }
    if is_sims_package_path(path) {
        if !preview_types.sims_packages {
            return Err("Sims package previews are disabled in Hover options.".to_string());
        }
        return sims_package_preview(path, token);
    }
    if is_video_path(path) {
        if !preview_types.videos {
            return Err("Video previews are disabled in Hover options.".to_string());
        }
        return Ok(PreviewPayload::Video {
            path: path.to_path_buf(),
        });
    }
    if is_audio_path(path) {
        if !preview_types.audio {
            return Err("Audio previews are disabled in Hover options.".to_string());
        }
        let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
        return Ok(PreviewPayload::Audio {
            path: path.to_path_buf(),
            title: path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Audio preview")
                .to_string(),
            subtitle: format!("Audio file - {}", format_bytes(metadata.len())),
        });
    }
    if is_text_preview_path(path) {
        if !preview_types.text {
            return Err("Text previews are disabled in Hover options.".to_string());
        }
        return text_preview(path);
    }
    if !preview_types.images {
        return Err("Image previews are disabled in Hover options.".to_string());
    }
    let animated = is_gif_path(path);
    let (width, height) = if animated {
        (
            MAX_ANIMATED_IMAGE_PREVIEW_WIDTH,
            MAX_ANIMATED_IMAGE_PREVIEW_HEIGHT,
        )
    } else {
        (
            MAX_STATIC_IMAGE_PREVIEW_DIMENSION,
            MAX_STATIC_IMAGE_PREVIEW_DIMENSION,
        )
    };
    let frames = crate::decoder_process::decode_file(path, animated, width, height, token)?;
    Ok(PreviewPayload::Frames {
        frames: frames.into_iter().map(frame_from_decoded_frame).collect(),
    })
}

fn folder_preview(path: &Path, token: &CancellationToken) -> Result<PreviewPayload, String> {
    let mut entries = Vec::new();
    let mut folder_count = 0usize;
    let mut file_count = 0usize;
    let mut total = 0usize;
    let mut has_more = false;
    for entry in fs::read_dir(path).map_err(|error| error.to_string())? {
        if token.is_cancelled() {
            return Err("Preview was cancelled.".into());
        }
        if entries.len() >= MAX_PREVIEW_ENTRIES {
            has_more = true;
            break;
        }
        let entry = entry.map_err(|error| error.to_string())?;
        total += 1;
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry
            .file_type()
            .map(|file_type| file_type.is_dir())
            .unwrap_or(false);
        if is_dir {
            folder_count += 1;
        } else {
            file_count += 1;
        }
        let detail = if is_dir {
            "Folder".to_string()
        } else {
            entry
                .metadata()
                .map(|metadata| format_bytes(metadata.len()))
                .unwrap_or_else(|_| "File".to_string())
        };
        entries.push(ListEntry {
            name,
            detail,
            path: Some(entry.path()),
            kind: if is_dir {
                EntryKind::Directory
            } else {
                EntryKind::File
            },
        });
    }
    entries.sort_by(|left, right| {
        left.kind.cmp(&right.kind).then_with(|| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
        })
    });
    if total == 0 {
        entries.push(ListEntry {
            name: "Empty folder".to_string(),
            detail: String::new(),
            path: None,
            kind: EntryKind::Directory,
        });
    }
    Ok(PreviewPayload::List {
        title: path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Folder")
            .to_string(),
        subtitle: format!(
            "{} folders, {} files{}",
            folder_count,
            file_count,
            if has_more {
                " shown · additional entries were not scanned"
            } else {
                ""
            }
        ),
        entries,
        omitted: 0,
    })
}

fn zip_preview(path: &Path, token: &CancellationToken) -> Result<PreviewPayload, String> {
    let mut file = open_preview_file(path).map_err(|error| error.to_string())?;
    crate::zip_limits::preflight(&mut file, token)?;
    let mut archive = ZipArchive::with_config(
        ZipConfig {
            archive_offset: ArchiveOffset::Known(0),
        },
        file,
    )
    .map_err(|error| error.to_string())?;
    let total = archive.len();
    let mut entries = Vec::new();
    let mut folder_count = 0usize;
    let mut file_count = 0usize;
    for index in 0..total.min(MAX_PREVIEW_ENTRIES) {
        if token.is_cancelled() {
            return Err("Preview was cancelled.".into());
        }
        let entry = archive.by_index(index).map_err(|error| error.to_string())?;
        let mut name = entry
            .name()
            .map_err(|error| error.to_string())?
            .replace('/', "\\");
        let is_dir = entry.is_dir();
        if is_dir {
            folder_count += 1;
            name = format!("[Folder] {}", name.trim_end_matches('\\'));
        } else {
            file_count += 1;
        }
        entries.push(ListEntry {
            name,
            detail: if is_dir {
                "Folder in archive".to_string()
            } else {
                format_bytes(entry.size())
            },
            path: None,
            kind: if is_dir {
                EntryKind::ZipDirectory
            } else {
                EntryKind::ZipFile
            },
        });
    }
    if total == 0 {
        entries.push(ListEntry {
            name: "Empty zip archive".to_string(),
            detail: String::new(),
            path: None,
            kind: EntryKind::ZipFile,
        });
    }
    Ok(PreviewPayload::List {
        title: path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Zip archive")
            .to_string(),
        subtitle: format!(
            "ZIP · {} entries total · showing {} folders, {} files",
            total, folder_count, file_count
        ),
        entries,
        omitted: total.saturating_sub(MAX_PREVIEW_ENTRIES),
    })
}

type DbpfEntry = hover_formats::dbpf::Entry;

fn sims_package_preview(path: &Path, token: &CancellationToken) -> Result<PreviewPayload, String> {
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    let index = read_dbpf_entries(path, token)?;
    let entries = &index.entries;
    let mut type_counts: HashMap<u32, usize> = HashMap::new();
    for entry in entries {
        *type_counts.entry(entry.type_id).or_default() += 1;
    }

    let mut rows = Vec::new();
    let package_tags = infer_sims_package_tags(entries, &type_counts);
    rows.push(ListEntry {
        name: "Resource index · metadata only".to_string(),
        detail: package_tags.join(", "),
        path: None,
        kind: EntryKind::File,
    });
    for (type_id, count) in sorted_sims_resource_counts(&type_counts)
        .into_iter()
        .take(14)
    {
        rows.push(ListEntry {
            name: sims_resource_label(type_id).to_string(),
            detail: format!("{} resource{}", count, if count == 1 { "" } else { "s" }),
            path: None,
            kind: EntryKind::File,
        });
    }
    let summary_rows = rows.len();
    for entry in entries
        .iter()
        .take(MAX_PREVIEW_ENTRIES.saturating_sub(rows.len()))
    {
        rows.push(ListEntry {
            name: format!(
                "{} 0x{:016X}",
                sims_resource_label(entry.type_id),
                entry.instance_id
            ),
            detail: format!(
                "G 0x{:08X} - @0x{:08X} - {} / {}{}",
                entry.group_id,
                entry.position,
                format_bytes(entry.size as u64),
                format_bytes(entry.decompressed_size as u64),
                entry
                    .compression
                    .map(|value| format!(" - cmp 0x{value:04X}"))
                    .unwrap_or_default()
            ),
            path: None,
            kind: EntryKind::File,
        });
    }

    let omitted = (index.declared_count as usize).saturating_sub(rows.len() - summary_rows);
    Ok(PreviewPayload::List {
        title: path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Sims 4 package")
            .to_string(),
        subtitle: format!(
            "DBPF 2.1 metadata · {}/{} index entries inspected{} · {}",
            entries.len(),
            index.declared_count,
            if index.truncated { " (partial)" } else { "" },
            format_bytes(metadata.len())
        ),
        entries: rows,
        omitted,
    })
}

fn read_dbpf_entries(
    path: &Path,
    token: &CancellationToken,
) -> Result<hover_formats::dbpf::Index, String> {
    let mut file = open_preview_file(path).map_err(|error| error.to_string())?;
    hover_formats::dbpf::read_index(
        &mut file,
        hover_formats::dbpf::Limits {
            max_preview_entries: MAX_SIM_PACKAGE_INDEX_ENTRIES,
            ..Default::default()
        },
        token,
    )
    .map_err(|error| error.to_string())
}

fn infer_sims_package_tags(
    entries: &[DbpfEntry],
    type_counts: &HashMap<u32, usize>,
) -> Vec<String> {
    let mut tags = Vec::new();
    if type_counts.contains_key(&0x034A_EECB) {
        tags.push("CAS Part".to_string());
    }
    if type_counts.contains_key(&0x3C1A_F1F2) {
        tags.push("CAS thumbnail".to_string());
    }
    if type_counts.contains_key(&0xC0DB_5AE7) || type_counts.contains_key(&0x319E_4F1D) {
        tags.push("Build/Buy object".to_string());
    }
    if type_counts.contains_key(&0x0166_1233) || type_counts.contains_key(&0x01D1_0F34) {
        tags.push("Mesh/model".to_string());
    }
    if entries.len() >= MAX_SIM_PACKAGE_INDEX_ENTRIES {
        tags.push("Index sample capped at 5,000 entries".to_string());
    }
    if tags.is_empty() {
        tags.push("Warehouse".to_string());
    }
    tags
}

fn sorted_sims_resource_counts(type_counts: &HashMap<u32, usize>) -> Vec<(u32, usize)> {
    let mut pairs = type_counts
        .iter()
        .map(|(type_id, count)| (*type_id, *count))
        .collect::<Vec<_>>();
    pairs.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    pairs
}

fn sims_resource_label(type_id: u32) -> &'static str {
    match type_id {
        0x034A_EECB => "CAS Part",
        0x3C1A_F1F2 => "CAS Part Thumbnail",
        0xEAA3_2ADD => "CAS Preset",
        0x2F7D_0004 => "PNG Image",
        0xB6C8_B6A0 => "DDS Image",
        0x00B2_D882 => "DST Image",
        0x3453_CF95 => "RLE2 Image",
        0xBA85_6C78 => "RLES Image",
        0x0166_1233 => "Model",
        0x01D1_0F34 => "Model LOD",
        0x8EAF_13DE => "Rig",
        0xD304_4521 => "Slot",
        0xC0DB_5AE7 => "Object Definition",
        0x319E_4F1D => "Object Catalog",
        0xFF56_010C => "Object Catalog Set",
        0x545A_C67A => "SimData",
        0x2205_57DA => "String Table",
        0x62E9_4D38 => "Combined Tuning",
        0x02D5_DF13 => "Animation State Machine",
        0x0166_038C => "Name Map",
        0xD382_BF57 => "Footprint",
        0x03B4_C61D => "Light",
        0x2579_6DCA => "OpenType Font",
        0x276C_A4B9 => "TrueType Font",
        0x2A8A_5E22 => "Tray Item",
        0x025E_D6F4 => "Sim Info",
        _ => "Warehouse Resource",
    }
}

fn text_preview(path: &Path) -> Result<PreviewPayload, String> {
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    if !metadata.is_file() {
        return Err("Not a text file.".to_string());
    }
    let mut file = open_preview_file(path).map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    let mut handle = std::io::Read::by_ref(&mut file).take(MAX_TEXT_PREVIEW_BYTES);
    handle
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    let truncated = metadata.len() > bytes.len() as u64;
    let text = decode_text_bytes(&bytes)?;
    let mut lines = Vec::new();
    for raw_line in text.replace("\r\n", "\n").replace('\r', "\n").lines() {
        if lines.len() >= MAX_TEXT_PREVIEW_LINES {
            break;
        }
        lines.push(clean_text_line(raw_line));
    }
    if lines.is_empty() {
        lines.push("Empty text file".to_string());
    }
    let title = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Text preview")
        .to_string();
    Ok(PreviewPayload::Text {
        title,
        subtitle: format!(
            "{} - {}",
            text_kind_label(path),
            format_bytes(metadata.len())
        ),
        lines,
        truncated: truncated || text.lines().count() > MAX_TEXT_PREVIEW_LINES,
    })
}

fn decode_text_bytes(bytes: &[u8]) -> Result<String, String> {
    if bytes.iter().take(4096).filter(|byte| **byte == 0).count() > 64
        && !(bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]))
    {
        return Err("File looks binary, not text.".to_string());
    }
    if bytes.starts_with(&[0xff, 0xfe]) {
        let words = bytes[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        return String::from_utf16(&words).map_err(|_| "Text is not valid UTF-16.".to_string());
    }
    if bytes.starts_with(&[0xfe, 0xff]) {
        let words = bytes[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        return String::from_utf16(&words).map_err(|_| "Text is not valid UTF-16.".to_string());
    }
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    Ok(String::from_utf8_lossy(bytes).into_owned())
}

fn clean_text_line(line: &str) -> String {
    let mut output = String::new();
    for character in line.chars().take(MAX_TEXT_LINE_CHARS) {
        match character {
            '\t' => output.push_str("    "),
            character if character.is_control() => output.push(' '),
            character => output.push(character),
        }
    }
    if line.chars().count() > MAX_TEXT_LINE_CHARS {
        output.push_str(" ...");
    }
    output
}

fn text_kind_label(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(|ext| ext.to_ascii_lowercase())
    {
        Some(ext) if matches!(ext.as_str(), "md" | "markdown" | "mdown" | "mkd") => "Markdown",
        Some(ext) if ext == "nfo" => "NFO text",
        Some(ext) if ext == "log" => "Log file",
        Some(ext)
            if matches!(
                ext.as_str(),
                "json" | "yaml" | "yml" | "toml" | "ini" | "cfg" | "conf"
            ) =>
        {
            "Config text"
        }
        Some(ext)
            if matches!(
                ext.as_str(),
                "rs" | "js" | "ts" | "css" | "html" | "xml" | "py" | "ps1" | "bat" | "cmd" | "sql"
            ) =>
        {
            "Source text"
        }
        _ => "Text file",
    }
}

fn frame_from_decoded_frame(frame: DecodedFrame) -> OverlayFrame {
    OverlayFrame {
        width: frame.width,
        height: frame.height,
        stride: frame.stride,
        pixels_bgra: frame.pixels_bgra,
        delay_ms: frame.delay_ms.max(33),
    }
}

fn payload_size(payload: &PreviewPayload) -> Size {
    match payload {
        PreviewPayload::Frames { frames } => frames
            .first()
            .map(|frame| Size::new(frame.width as f32, frame.height as f32))
            .unwrap_or_else(|| Size::new(520.0, 320.0)),
        PreviewPayload::Text { lines, .. } => {
            let rows = lines.len().min(18) as f32;
            Size::new(760.0, (108.0 + rows * 20.0).clamp(260.0, 700.0))
        }
        PreviewPayload::List { entries, .. } => {
            let rows = entries.len().min(16) as f32;
            Size::new(640.0, (104.0 + rows * 32.0).clamp(220.0, 680.0))
        }
        PreviewPayload::Video { .. } => Size::new(720.0, 405.0),
        PreviewPayload::Audio { .. } => Size::new(620.0, 220.0),
    }
}

fn set_overlay_model(model: Option<OverlayModel>) {
    let mut guard = OVERLAY_MODEL
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap();
    *guard = model;
}

fn current_overlay_frame_delay_ms() -> Option<u32> {
    let guard = OVERLAY_MODEL
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap();
    let model = guard.as_ref()?;
    match &model.content {
        OverlayContent::Frames { frames, .. } if frames.len() > 1 => frames
            .get(model.current_frame)
            .map(|frame| frame.delay_ms.max(33)),
        _ => None,
    }
}

fn advance_overlay_frame_and_delay_ms() -> Option<u32> {
    let mut guard = OVERLAY_MODEL
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap();
    let model = guard.as_mut()?;
    match &model.content {
        OverlayContent::Frames { frames, .. } if frames.len() > 1 => {
            model.current_frame = (model.current_frame + 1) % frames.len();
            frames
                .get(model.current_frame)
                .map(|frame| frame.delay_ms.max(33))
        }
        _ => None,
    }
}

unsafe fn apply_overlay_wheel(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) -> OverlayWheelAction {
    let screen_point = POINT {
        x: signed_loword_isize(lparam),
        y: signed_hiword_isize(lparam),
    };
    apply_overlay_wheel_at_screen(hwnd, screen_point, signed_hiword_usize(wparam))
}

unsafe fn apply_overlay_wheel_at_screen(
    hwnd: HWND,
    screen_point: POINT,
    delta: i16,
) -> OverlayWheelAction {
    let wheel_delta = delta as f32 / WHEEL_DELTA as f32;
    if wheel_delta == 0.0 || !wheel_delta.is_finite() {
        return OverlayWheelAction::None;
    }

    let mut point = screen_point;
    if ScreenToClient(hwnd, &mut point) == 0 {
        return OverlayWheelAction::None;
    }

    let client = content_client_rect(hwnd);
    if point.x < client.left
        || point.x >= client.right
        || point.y < client.top
        || point.y >= client.bottom
    {
        return OverlayWheelAction::None;
    }

    let modifier = wheel_modifier_from_keyboard();
    let mut guard = OVERLAY_MODEL
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap();
    let Some(model) = guard.as_mut() else {
        return OverlayWheelAction::None;
    };
    let scroll_request = match &mut model.content {
        OverlayContent::Frames { frames, zoom } if !frames.is_empty() => {
            zoom.set_viewport(
                Size::new(
                    (client.right - client.left) as f32,
                    (client.bottom - client.top) as f32,
                ),
                ZoomConfig::default(),
            );
            zoom.apply_wheel(
                HoverPoint::new(
                    (point.x - client.left) as f32,
                    (point.y - client.top) as f32,
                ),
                wheel_delta,
                modifier,
                ZoomConfig::default(),
            );
            return OverlayWheelAction::Zoom;
        }
        OverlayContent::Text {
            lines, truncated, ..
        } => Some((
            text_virtual_line_count(lines, *truncated),
            text_visible_rows(&client),
        )),
        OverlayContent::List { entries, .. } => Some((entries.len(), list_visible_rows(&client))),
        _ => None,
    };
    if let Some((total_rows, visible_rows)) = scroll_request {
        scroll_overlay_content(
            &mut model.scroll_offset,
            total_rows,
            visible_rows,
            wheel_delta,
        )
    } else {
        OverlayWheelAction::None
    }
}

fn scroll_overlay_content(
    offset: &mut usize,
    total_rows: usize,
    visible_rows: usize,
    wheel_delta: f32,
) -> OverlayWheelAction {
    if total_rows <= visible_rows || visible_rows == 0 {
        *offset = 0;
        return OverlayWheelAction::None;
    }
    let max_offset = total_rows.saturating_sub(visible_rows);
    let step = ((wheel_delta.abs().ceil() as usize).max(1)).saturating_mul(WHEEL_SCROLL_ROWS);
    let next = if wheel_delta < 0.0 {
        (*offset).saturating_add(step).min(max_offset)
    } else {
        (*offset).saturating_sub(step)
    };
    *offset = next;
    OverlayWheelAction::Repaint
}

fn text_virtual_line_count(lines: &[String], truncated: bool) -> usize {
    lines.len() + if truncated { 2 } else { 0 }
}

fn text_visible_rows(client: &RECT) -> usize {
    ((client.bottom - (client.top + 84) - 24) / TEXT_LINE_HEIGHT).max(0) as usize
}

fn list_visible_rows(client: &RECT) -> usize {
    ((client.bottom - (client.top + 76) - 14) / LIST_ROW_HEIGHT).max(0) as usize
}

fn settle_overlay_zoom_quality() -> bool {
    let mut guard = OVERLAY_MODEL
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap();
    let Some(model) = guard.as_mut() else {
        return false;
    };
    match &mut model.content {
        OverlayContent::Frames { zoom, .. } => {
            zoom.settle_quality();
            true
        }
        OverlayContent::Text { .. } | OverlayContent::List { .. } => false,
    }
}

unsafe fn wheel_modifier_from_keyboard() -> WheelModifier {
    if GetKeyState(VK_CONTROL as i32) < 0 {
        WheelModifier::Ctrl
    } else if GetKeyState(VK_SHIFT as i32) < 0 {
        WheelModifier::Shift
    } else {
        WheelModifier::Plain
    }
}

unsafe fn key_pressed_once(key: u16) -> bool {
    (GetAsyncKeyState(key as i32) & 1) != 0
}

unsafe fn cursor_outside_window(hwnd: HWND) -> bool {
    if hwnd.is_null() {
        return true;
    }
    let mut cursor = POINT::default();
    if GetCursorPos(&mut cursor) == 0 {
        return true;
    }
    let mut rect = RECT::default();
    if GetWindowRect(hwnd, &mut rect) == 0 {
        return true;
    }
    cursor.x < rect.left || cursor.x >= rect.right || cursor.y < rect.top || cursor.y >= rect.bottom
}

fn signed_loword_isize(value: isize) -> i32 {
    (value as u32 & 0xffff) as u16 as i16 as i32
}

fn signed_hiword_isize(value: isize) -> i32 {
    ((value as u32 >> 16) & 0xffff) as u16 as i16 as i32
}

fn signed_hiword_usize(value: usize) -> i16 {
    ((value >> 16) & 0xffff) as u16 as i16
}

fn signed_hiword_u32(value: u32) -> i16 {
    ((value >> 16) & 0xffff) as u16 as i16
}

unsafe fn cursor_inside_window(hwnd: HWND) -> bool {
    !cursor_outside_window(hwnd)
}

unsafe fn cursor_inside_rect(rect: HoverRect) -> bool {
    let mut cursor = POINT::default();
    if GetCursorPos(&mut cursor) == 0 {
        return false;
    }
    (cursor.x as f32) >= rect.left
        && (cursor.x as f32) < rect.right
        && (cursor.y as f32) >= rect.top
        && (cursor.y as f32) < rect.bottom
}

unsafe fn cursor_inside_video_window() -> bool {
    let hwnd = current_video_window();
    !hwnd.is_null() && cursor_inside_window(hwnd)
}

unsafe fn paint_overlay(
    hwnd: HWND,
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    _paint_rect: &RECT,
) {
    let mut client = RECT::default();
    GetClientRect(hwnd, &mut client);
    let guard = OVERLAY_MODEL
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap();
    let Some(model) = guard.as_ref() else {
        return;
    };
    let background = match model.content {
        OverlayContent::Frames { .. } => TRANSPARENT_IMAGE_COLOR,
        OverlayContent::Text { .. } | OverlayContent::List { .. } => rgb(20, 22, 25),
    };
    let brush = CreateSolidBrush(background);
    FillRect(hdc, &client, brush);
    DeleteObject(brush);
    fill_rect(
        hdc,
        client.left,
        client.top,
        client.right,
        PREVIEW_CHROME_HEIGHT.min(client.bottom),
        rgb(20, 22, 25),
    );
    client = content_client_rect(hwnd);
    let saved_dc = SaveDC(hdc);
    IntersectClipRect(hdc, client.left, client.top, client.right, client.bottom);
    match &model.content {
        OverlayContent::Frames { frames, zoom } => {
            if let Some(frame) = frames.get(model.current_frame) {
                paint_frame(hdc, frame, *zoom, client.top);
            }
        }
        OverlayContent::Text {
            title,
            subtitle,
            lines,
            truncated,
        } => paint_text_preview(
            hdc,
            &client,
            title,
            subtitle,
            lines,
            *truncated,
            model.scroll_offset,
        ),
        OverlayContent::List {
            title,
            subtitle,
            entries,
            omitted,
        } => paint_list_preview(
            hdc,
            &client,
            title,
            subtitle,
            entries,
            *omitted,
            model.scroll_offset,
        ),
    }
    if saved_dc != 0 {
        RestoreDC(hdc, saved_dc);
    }
}

unsafe fn paint_frame(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    frame: &OverlayFrame,
    zoom: ZoomState,
    origin_y: i32,
) {
    let tight_stride = frame.width.saturating_mul(4);
    let expected_pixels = frame.stride as usize * frame.height as usize;
    if frame.width == 0
        || frame.height == 0
        || frame.stride != tight_stride
        || frame.pixels_bgra.len() < expected_pixels
    {
        return;
    }

    let top_left = zoom.image_top_left();
    let dst_width = (frame.width as f32 * zoom.scale).round().max(1.0) as i32;
    let dst_height = (frame.height as f32 * zoom.scale).round().max(1.0) as i32;
    let bitmap_info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: frame.width as i32,
            biHeight: -(frame.height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [Default::default(); 1],
    };
    if zoom.high_quality {
        SetStretchBltMode(hdc, HALFTONE);
        SetBrushOrgEx(hdc, 0, 0, null_mut());
    } else {
        SetStretchBltMode(hdc, COLORONCOLOR);
    }
    StretchDIBits(
        hdc,
        top_left.x.round() as i32,
        top_left.y.round() as i32 + origin_y,
        dst_width,
        dst_height,
        0,
        0,
        frame.width as i32,
        frame.height as i32,
        frame.pixels_bgra.as_ptr() as *const c_void,
        &bitmap_info,
        DIB_RGB_COLORS,
        SRCCOPY,
    );
}

unsafe fn paint_text_preview(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    client: &RECT,
    title: &str,
    subtitle: &str,
    lines: &[String],
    truncated: bool,
    scroll_offset: usize,
) {
    SetBkMode(hdc, TRANSPARENT as i32);
    fill_rect(
        hdc,
        client.left,
        client.top,
        client.right,
        client.top + 4,
        rgb(93, 163, 255),
    );
    SetTextColor(hdc, rgb(246, 248, 250));
    let mut title_rect = RECT {
        left: client.left + 20,
        top: client.top + 14,
        right: client.right - 18,
        bottom: client.top + 40,
    };
    let title = wide(title);
    DrawTextW(
        hdc,
        title.as_ptr(),
        -1,
        &mut title_rect,
        DT_LEFT | DT_TOP | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
    );

    if !subtitle.is_empty() {
        SetTextColor(hdc, rgb(159, 174, 192));
        let subtitle = wide(subtitle);
        let mut subtitle_rect = RECT {
            left: client.left + 20,
            top: client.top + 42,
            right: client.right - 20,
            bottom: client.top + 64,
        };
        DrawTextW(
            hdc,
            subtitle.as_ptr(),
            -1,
            &mut subtitle_rect,
            DT_LEFT | DT_TOP | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
        );
    }

    fill_rect(
        hdc,
        client.left + 14,
        client.top + 74,
        client.right - 14,
        client.bottom - 14,
        rgb(15, 17, 20),
    );
    frame_rect(
        hdc,
        client.left + 14,
        client.top + 74,
        client.right - 14,
        client.bottom - 14,
        rgb(47, 52, 59),
    );
    SetTextColor(hdc, rgb(229, 233, 240));
    let visible_rows = text_visible_rows(client).max(1);
    let mut body_lines = lines.to_vec();
    if truncated {
        body_lines.push(String::new());
        body_lines.push("... preview truncated".to_string());
    }
    let total_rows = body_lines.len();
    let scroll_offset = scroll_offset.min(total_rows.saturating_sub(visible_rows));
    let body = body_lines
        .iter()
        .skip(scroll_offset)
        .take(visible_rows)
        .cloned()
        .collect::<Vec<_>>()
        .join("\r\n");
    let body = wide(&body);
    let mut body_rect = RECT {
        left: client.left + 24,
        top: client.top + 84,
        right: client.right - 24,
        bottom: client.bottom - 24,
    };
    DrawTextW(
        hdc,
        body.as_ptr(),
        -1,
        &mut body_rect,
        DT_LEFT | DT_TOP | DT_WORDBREAK | DT_NOPREFIX,
    );
    paint_scrollbar(
        hdc,
        client.right - 14,
        client.top + 84,
        client.bottom - 24,
        total_rows,
        visible_rows,
        scroll_offset,
        rgb(93, 163, 255),
    );
}

unsafe fn paint_list_preview(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    client: &RECT,
    title: &str,
    subtitle: &str,
    entries: &[ListEntry],
    omitted: usize,
    scroll_offset: usize,
) {
    SetBkMode(hdc, TRANSPARENT as i32);
    fill_rect(
        hdc,
        client.left,
        client.top,
        client.right,
        client.top + 4,
        rgb(255, 196, 87),
    );
    SetTextColor(hdc, rgb(247, 248, 250));
    let mut title_rect = RECT {
        left: client.left + 20,
        top: client.top + 14,
        right: client.right - 20,
        bottom: client.top + 40,
    };
    let title = wide(title);
    DrawTextW(
        hdc,
        title.as_ptr(),
        -1,
        &mut title_rect,
        DT_LEFT | DT_TOP | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
    );
    SetTextColor(hdc, rgb(166, 177, 191));
    let subtitle = if omitted > 0 {
        format!("{subtitle} - {} more hidden", omitted)
    } else {
        subtitle.to_string()
    };
    let subtitle = wide(&subtitle);
    let mut subtitle_rect = RECT {
        left: client.left + 20,
        top: client.top + 42,
        right: client.right - 20,
        bottom: client.top + 64,
    };
    DrawTextW(
        hdc,
        subtitle.as_ptr(),
        -1,
        &mut subtitle_rect,
        DT_LEFT | DT_TOP | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
    );

    let list_top = client.top + 76;
    let row_height = LIST_ROW_HEIGHT;
    let available_rows = list_visible_rows(client);
    let scroll_offset = scroll_offset.min(entries.len().saturating_sub(available_rows));
    for (index, entry) in entries
        .iter()
        .skip(scroll_offset)
        .take(available_rows)
        .enumerate()
    {
        let top = list_top + index as i32 * row_height;
        if index % 2 == 0 {
            fill_rect(
                hdc,
                client.left + 12,
                top,
                client.right - 12,
                top + row_height,
                rgb(25, 28, 32),
            );
        }
        paint_entry_icon(hdc, entry, client.left + 22, top + 7);
        SetTextColor(hdc, rgb(235, 238, 243));
        let mut name_rect = RECT {
            left: client.left + 50,
            top: top + 6,
            right: client.right - 132,
            bottom: top + 26,
        };
        let name = wide(&entry.name);
        DrawTextW(
            hdc,
            name.as_ptr(),
            -1,
            &mut name_rect,
            DT_LEFT | DT_TOP | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
        );
        SetTextColor(hdc, rgb(150, 163, 181));
        let mut detail_rect = RECT {
            left: client.right - 126,
            top: top + 6,
            right: client.right - 20,
            bottom: top + 26,
        };
        let detail = wide(&entry.detail);
        DrawTextW(
            hdc,
            detail.as_ptr(),
            -1,
            &mut detail_rect,
            DT_LEFT | DT_TOP | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
        );
    }
    paint_scrollbar(
        hdc,
        client.right - 12,
        list_top,
        client.bottom - 14,
        entries.len(),
        available_rows,
        scroll_offset,
        rgb(255, 196, 87),
    );
}

#[allow(clippy::too_many_arguments)] // Win32 drawing coordinates and colors.
unsafe fn paint_scrollbar(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    x: i32,
    top: i32,
    bottom: i32,
    total_rows: usize,
    visible_rows: usize,
    offset: usize,
    color: u32,
) {
    if total_rows <= visible_rows || visible_rows == 0 || bottom <= top {
        return;
    }
    let height = (bottom - top).max(1);
    fill_rect(hdc, x, top, x + 4, bottom, rgb(37, 42, 48));
    let thumb_height = ((height as f32 * visible_rows as f32 / total_rows as f32).round() as i32)
        .clamp(24, height);
    let max_offset = total_rows.saturating_sub(visible_rows).max(1);
    let travel = (height - thumb_height).max(0);
    let thumb_top = top + ((travel as f32 * offset as f32 / max_offset as f32).round() as i32);
    fill_rect(
        hdc,
        x - 1,
        thumb_top,
        x + 5,
        thumb_top + thumb_height,
        color,
    );
}

unsafe fn paint_entry_icon(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    entry: &ListEntry,
    left: i32,
    top: i32,
) {
    if let Some(path) = &entry.path
        && paint_shell_icon(hdc, path, left, top)
    {
        return;
    }
    match entry.kind {
        EntryKind::Directory | EntryKind::ZipDirectory => {
            paint_fallback_folder_icon(hdc, left, top)
        }
        EntryKind::File | EntryKind::ZipFile => paint_fallback_file_icon(hdc, left, top),
    }
}

unsafe fn paint_shell_icon(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    path: &Path,
    left: i32,
    top: i32,
) -> bool {
    let path_wide = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut info = SHFILEINFOW::default();
    let result = SHGetFileInfoW(
        path_wide.as_ptr(),
        0,
        &mut info,
        std::mem::size_of::<SHFILEINFOW>() as u32,
        SHGFI_ICON | SHGFI_SMALLICON,
    );
    if result == 0 || info.hIcon.is_null() {
        return false;
    }
    let drawn = DrawIconEx(hdc, left, top, info.hIcon, 16, 16, 0, null_mut(), DI_NORMAL) != 0;
    DestroyIcon(info.hIcon);
    drawn
}

unsafe fn paint_fallback_folder_icon(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    left: i32,
    top: i32,
) {
    fill_rect(hdc, left + 1, top + 3, left + 8, top + 7, rgb(237, 190, 83));
    fill_rect(hdc, left, top + 6, left + 17, top + 16, rgb(255, 210, 95));
    frame_rect(hdc, left, top + 6, left + 17, top + 16, rgb(159, 118, 45));
}

unsafe fn paint_fallback_file_icon(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    left: i32,
    top: i32,
) {
    fill_rect(hdc, left + 2, top, left + 14, top + 17, rgb(232, 237, 245));
    fill_rect(hdc, left + 10, top, left + 14, top + 5, rgb(190, 202, 219));
    frame_rect(hdc, left + 2, top, left + 14, top + 17, rgb(112, 126, 145));
}

unsafe fn fill_rect(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    color: u32,
) {
    let brush = CreateSolidBrush(color);
    let rect = RECT {
        left,
        top,
        right,
        bottom,
    };
    FillRect(hdc, &rect, brush);
    DeleteObject(brush);
}

unsafe fn frame_rect(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    color: u32,
) {
    let brush = CreateSolidBrush(color);
    let rect = RECT {
        left,
        top,
        right,
        bottom,
    };
    FrameRect(hdc, &rect, brush);
    DeleteObject(brush);
}

fn current_cursor_point() -> HoverPoint {
    unsafe {
        let mut point = POINT::default();
        GetCursorPos(&mut point);
        HoverPoint::new(point.x as f32, point.y as f32)
    }
}

fn screen_work_area(cursor: HoverPoint) -> HoverRect {
    unsafe {
        let monitor = MonitorFromPoint(
            POINT {
                x: cursor.x.round() as i32,
                y: cursor.y.round() as i32,
            },
            MONITOR_DEFAULTTONEAREST,
        );
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !monitor.is_null() && GetMonitorInfoW(monitor, &mut info) != 0 {
            let rect = info.rcWork;
            if rect.right > rect.left && rect.bottom > rect.top {
                return HoverRect::new(
                    rect.left as f32,
                    rect.top as f32,
                    rect.right as f32,
                    rect.bottom as f32,
                );
            }
        }
        HoverRect::new(
            0.0,
            0.0,
            GetSystemMetrics(SM_CXSCREEN).max(1) as f32,
            GetSystemMetrics(SM_CYSCREEN).max(1) as f32,
        )
    }
}

fn is_gif_path(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("gif"))
}

fn is_text_preview_path(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(
        file_name.as_str(),
        "readme"
            | "license"
            | "licence"
            | "copying"
            | "notice"
            | "authors"
            | "contributors"
            | "changelog"
            | "changes"
            | "makefile"
    ) || matches!(
        stem.as_str(),
        "readme" | "license" | "licence" | "changelog"
    ) {
        return true;
    }
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "txt"
                    | "text"
                    | "md"
                    | "markdown"
                    | "mdown"
                    | "mkd"
                    | "nfo"
                    | "log"
                    | "ini"
                    | "cfg"
                    | "conf"
                    | "json"
                    | "yaml"
                    | "yml"
                    | "toml"
                    | "csv"
                    | "tsv"
                    | "xml"
                    | "html"
                    | "htm"
                    | "css"
                    | "js"
                    | "ts"
                    | "rs"
                    | "py"
                    | "ps1"
                    | "bat"
                    | "cmd"
                    | "sql"
                    | "c"
                    | "cpp"
                    | "h"
                    | "hpp"
                    | "cs"
                    | "java"
                    | "go"
                    | "rb"
                    | "php"
            )
        })
}

fn is_zip_path(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
}

fn is_sims_package_path(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("package"))
}

fn is_video_path(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "mp4"
                    | "mkv"
                    | "mov"
                    | "webm"
                    | "avi"
                    | "wmv"
                    | "m4v"
                    | "mpeg"
                    | "mpg"
                    | "flv"
                    | "ogv"
                    | "3gp"
                    | "3g2"
                    | "ts"
                    | "m2ts"
                    | "mts"
                    | "divx"
                    | "vob"
                    | "asf"
                    | "rm"
                    | "rmvb"
                    | "mxf"
            )
        })
}

fn is_audio_path(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "mp3"
                    | "flac"
                    | "wav"
                    | "wave"
                    | "ogg"
                    | "oga"
                    | "opus"
                    | "m4a"
                    | "aac"
                    | "wma"
                    | "alac"
                    | "aiff"
                    | "aif"
                    | "ape"
                    | "mpc"
                    | "mka"
                    | "mid"
                    | "midi"
                    | "amr"
            )
        })
}

fn start_video_playback(
    path: &Path,
    rect: HoverRect,
    settings: &VideoSettings,
) -> Result<VideoPlayback, String> {
    let backends = locate_video_backends();
    if backends.is_empty() {
        return Err("No video backend was found. Install FFmpeg, mpv, or VLC, or set HOVER_FFPLAY_PATH, HOVER_MPV_PATH, or HOVER_VLC_PATH.".to_string());
    }
    let mut errors = Vec::new();
    for (backend, executable) in backends {
        match start_video_with_backend(backend, &executable, path, rect, settings) {
            Ok(video) => return Ok(video),
            Err(message) => errors.push(message),
        }
    }
    Err(format!(
        "No video backend could play this file: {}",
        errors.join("; ")
    ))
}

fn start_video_with_backend(
    backend: MediaBackend,
    executable: &Path,
    path: &Path,
    rect: HoverRect,
    settings: &VideoSettings,
) -> Result<VideoPlayback, String> {
    let mut command = Command::new(executable);
    configure_video_command(&mut command, backend, path, rect, settings);
    let child = spawn_media_command(command, backend)?;

    VIDEO_PID.store(child.id(), Ordering::Release);
    store_video_control_state(rect, settings.seek_seconds);
    for _ in 0..20 {
        if try_apply_video_window_style(child.id()) {
            break;
        }
        std::thread::yield_now();
    }
    ensure_video_window_topmost(rect);

    Ok(VideoPlayback {
        process: child,
        rect,
        started: Instant::now(),
        last_window_sync: Instant::now() - Duration::from_millis(VIDEO_ATTACH_RETRY_MS),
        window_attached: false,
    })
}

fn start_audio_playback(path: &Path) -> Result<AudioPlayback, String> {
    let backends = locate_audio_backends();
    if backends.is_empty() {
        return Err("No audio backend was found. Install FFmpeg, mpv, or VLC, or set HOVER_FFPLAY_PATH, HOVER_MPV_PATH, or HOVER_VLC_PATH.".to_string());
    }
    let mut errors = Vec::new();
    for (backend, executable) in backends {
        let mut command = Command::new(executable);
        configure_audio_command(&mut command, backend, path);
        match spawn_media_command(command, backend) {
            Ok(process) => return Ok(AudioPlayback { process }),
            Err(message) => errors.push(message),
        }
    }
    Err(format!(
        "No audio backend could play this file: {}",
        errors.join("; ")
    ))
}

fn spawn_media_command(mut command: Command, backend: MediaBackend) -> Result<Child, String> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|error| format!("Could not start {backend:?}: {error}"))?;
    if let Ok(Some(status)) = child.try_wait() {
        return Err(format!(
            "{backend:?} exited immediately with status {status}."
        ));
    }
    Ok(child)
}

fn configure_video_command(
    command: &mut Command,
    backend: MediaBackend,
    path: &Path,
    rect: HoverRect,
    settings: &VideoSettings,
) {
    match backend {
        MediaBackend::Ffplay => {
            if !settings.audio_enabled {
                command.arg("-an");
            }
            command
                .arg("-err_detect")
                .arg("ignore_err")
                .arg("-fflags")
                .arg("+genpts+discardcorrupt+igndts")
                .arg("-loglevel")
                .arg("quiet")
                .arg("-noborder")
                .arg("-autoexit")
                .arg("-loop")
                .arg("0")
                .arg("-framedrop")
                .arg("-seek_interval")
                .arg(settings.seek_seconds.to_string())
                .arg("-window_title")
                .arg("Hover Video Preview")
                .arg("-left")
                .arg(rect.left.round().to_string())
                .arg("-top")
                .arg(rect.top.round().to_string())
                .arg("-x")
                .arg(rect.width().round().max(1.0).to_string())
                .arg("-y")
                .arg(rect.height().round().max(1.0).to_string())
                .arg(path);
        }
        MediaBackend::Mpv => {
            command
                .arg("--no-terminal")
                .arg("--really-quiet")
                .arg("--force-window=yes")
                .arg("--ontop")
                .arg("--no-border")
                .arg(format!(
                    "--geometry={}x{}+{}+{}",
                    rect.width().round().max(1.0) as i32,
                    rect.height().round().max(1.0) as i32,
                    rect.left.round() as i32,
                    rect.top.round() as i32
                ));
            if !settings.audio_enabled {
                command.arg("--no-audio");
            }
            command.arg(path);
        }
        MediaBackend::Vlc => {
            command
                .arg("--quiet")
                .arg("--play-and-exit")
                .arg("--no-video-deco")
                .arg("--video-on-top")
                .arg(format!("--video-x={}", rect.left.round() as i32))
                .arg(format!("--video-y={}", rect.top.round() as i32))
                .arg(format!("--width={}", rect.width().round().max(1.0) as i32))
                .arg(format!(
                    "--height={}",
                    rect.height().round().max(1.0) as i32
                ));
            if !settings.audio_enabled {
                command.arg("--no-audio");
            }
            command.arg(path);
        }
    }
}

fn configure_audio_command(command: &mut Command, backend: MediaBackend, path: &Path) {
    match backend {
        MediaBackend::Ffplay => {
            command
                .arg("-nodisp")
                .arg("-autoexit")
                .arg("-loglevel")
                .arg("quiet")
                .arg(path);
        }
        MediaBackend::Mpv => {
            command
                .arg("--no-terminal")
                .arg("--really-quiet")
                .arg("--no-video")
                .arg(path);
        }
        MediaBackend::Vlc => {
            command
                .arg("--intf")
                .arg("dummy")
                .arg("--quiet")
                .arg("--play-and-exit")
                .arg(path);
        }
    }
}

fn locate_video_backends() -> Vec<(MediaBackend, PathBuf)> {
    locate_media_backends()
}

fn locate_audio_backends() -> Vec<(MediaBackend, PathBuf)> {
    locate_media_backends()
}

fn locate_media_backends() -> Vec<(MediaBackend, PathBuf)> {
    let mut backends = Vec::new();
    if let Some(path) = locate_ffplay_executable() {
        backends.push((MediaBackend::Ffplay, path));
    }
    if let Some(path) = locate_mpv_executable() {
        backends.push((MediaBackend::Mpv, path));
    }
    if let Some(path) = locate_vlc_executable() {
        backends.push((MediaBackend::Vlc, path));
    }
    backends
}

fn locate_ffplay_executable() -> Option<PathBuf> {
    if let Some(path) = env::var_os("HOVER_FFPLAY_PATH").map(PathBuf::from)
        && path.is_file()
    {
        return Some(path);
    }
    if let Some(path) = find_executable_in_path("ffplay.exe") {
        return Some(path);
    }
    if let Some(exe_dir) = env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
    {
        let relatives = [
            PathBuf::from("ffplay.exe"),
            PathBuf::from("ffmpeg").join("bin").join("ffplay.exe"),
            PathBuf::from("tools")
                .join("ffmpeg")
                .join("bin")
                .join("ffplay.exe"),
        ];
        for relative in relatives {
            let candidate = exe_dir.join(relative);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    common_ffplay_locations()
        .into_iter()
        .find(|path| path.is_file())
}

fn locate_mpv_executable() -> Option<PathBuf> {
    if let Some(path) = env::var_os("HOVER_MPV_PATH").map(PathBuf::from)
        && path.is_file()
    {
        return Some(path);
    }
    find_executable_in_path("mpv.exe").or_else(|| {
        common_mpv_locations()
            .into_iter()
            .find(|path| path.is_file())
    })
}

fn locate_vlc_executable() -> Option<PathBuf> {
    if let Some(path) = env::var_os("HOVER_VLC_PATH").map(PathBuf::from)
        && path.is_file()
    {
        return Some(path);
    }
    find_executable_in_path("vlc.exe").or_else(|| {
        common_vlc_locations()
            .into_iter()
            .find(|path| path.is_file())
    })
}

fn find_executable_in_path(file_name: &str) -> Option<PathBuf> {
    let path_value = env::var_os("PATH")?;
    env::split_paths(&path_value)
        .map(|directory| directory.join(file_name))
        .find(|candidate| candidate.is_file())
}

fn common_ffplay_locations() -> Vec<PathBuf> {
    let mut paths = vec![PathBuf::from(r"C:\ffmpeg\bin\ffplay.exe")];
    for variable in [
        "ProgramFiles",
        "ProgramFiles(x86)",
        "LOCALAPPDATA",
        "ProgramData",
    ] {
        if let Some(root) = env::var_os(variable).map(PathBuf::from) {
            paths.push(root.join(r"Gyan\FFmpeg\bin\ffplay.exe"));
            paths.push(root.join(r"FFmpeg\bin\ffplay.exe"));
            paths.push(root.join(r"chocolatey\bin\ffplay.exe"));
            paths.push(root.join(r"Microsoft\WinGet\Packages\Gyan.FFmpeg_Microsoft.Winget.Source_8wekyb3d8bbwe\ffmpeg\bin\ffplay.exe"));
        }
    }
    paths
}

fn common_mpv_locations() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for variable in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
        if let Some(root) = env::var_os(variable).map(PathBuf::from) {
            paths.push(root.join(r"mpv\mpv.exe"));
            paths.push(root.join(r"mpv.net\mpvnet.exe"));
            paths.push(root.join(r"Microsoft\WinGet\Packages\mpv-player.mpv_Microsoft.Winget.Source_8wekyb3d8bbwe\mpv.exe"));
        }
    }
    paths
}

fn common_vlc_locations() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for variable in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(root) = env::var_os(variable).map(PathBuf::from) {
            paths.push(root.join(r"VideoLAN\VLC\vlc.exe"));
        }
    }
    paths
}

struct EnumWindowsData {
    target_pid: u32,
    found_hwnd: HWND,
    best_area: i64,
}

unsafe extern "system" fn enum_video_windows(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let data = &mut *(lparam as *mut EnumWindowsData);
    let mut window_pid = 0u32;
    GetWindowThreadProcessId(hwnd, &mut window_pid);
    if window_pid != data.target_pid || IsWindowVisible(hwnd) == 0 {
        return 1;
    }
    if !GetWindow(hwnd, GW_OWNER).is_null() {
        return 1;
    }
    let mut rect = RECT::default();
    if GetWindowRect(hwnd, &mut rect) == 0 {
        return 1;
    }
    let width = (rect.right - rect.left).max(0) as i64;
    let height = (rect.bottom - rect.top).max(0) as i64;
    let area = width * height;
    if area > data.best_area {
        data.best_area = area;
        data.found_hwnd = hwnd;
    }
    1
}

fn current_video_window() -> HWND {
    let stored = VIDEO_HWND.load(Ordering::Acquire) as HWND;
    if !stored.is_null() {
        return stored;
    }
    let pid = VIDEO_PID.load(Ordering::Acquire);
    if pid == 0 {
        return null_mut();
    }
    unsafe { find_video_window_by_pid(pid) }
}

unsafe fn find_video_window_by_pid(pid: u32) -> HWND {
    let mut data = EnumWindowsData {
        target_pid: pid,
        found_hwnd: null_mut(),
        best_area: 0,
    };
    EnumWindows(
        Some(enum_video_windows),
        &mut data as *mut EnumWindowsData as LPARAM,
    );
    if !data.found_hwnd.is_null() {
        VIDEO_HWND.store(data.found_hwnd as isize, Ordering::Release);
    }
    data.found_hwnd
}

fn try_apply_video_window_style(pid: u32) -> bool {
    unsafe {
        let hwnd = find_video_window_by_pid(pid);
        if hwnd.is_null() {
            VIDEO_HWND.store(0, Ordering::Release);
            return false;
        }
        let current_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let new_style = current_style
            | WS_EX_NOACTIVATE as isize
            | WS_EX_TOOLWINDOW as isize
            | WS_EX_TOPMOST as isize;
        if new_style != current_style {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_style);
        }
        SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
        );
        ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        true
    }
}

fn sync_video_window(video: &mut VideoPlayback) {
    let now = Instant::now();
    let interval = if video.window_attached {
        Duration::from_millis(VIDEO_STEADY_SYNC_MS)
    } else {
        Duration::from_millis(VIDEO_ATTACH_RETRY_MS)
    };
    if now.duration_since(video.last_window_sync) < interval {
        return;
    }
    video.last_window_sync = now;
    video.window_attached = ensure_video_window_topmost(video.rect);
}

fn ensure_video_window_topmost(rect: HoverRect) -> bool {
    let pid = VIDEO_PID.load(Ordering::Acquire);
    if pid != 0 {
        try_apply_video_window_style(pid);
    }
    let hwnd = VIDEO_HWND.load(Ordering::Acquire) as HWND;
    if hwnd.is_null() {
        return false;
    }
    unsafe {
        SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            rect.left.round() as i32,
            rect.top.round() as i32,
            rect.width().round().max(1.0) as i32,
            rect.height().round().max(1.0) as i32,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
        ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
    true
}

fn store_video_control_state(rect: HoverRect, seek_seconds: u32) {
    VIDEO_RECT_LEFT.store(rect.left.round() as i32, Ordering::Release);
    VIDEO_RECT_TOP.store(rect.top.round() as i32, Ordering::Release);
    VIDEO_RECT_RIGHT.store(rect.right.round() as i32, Ordering::Release);
    VIDEO_RECT_BOTTOM.store(rect.bottom.round() as i32, Ordering::Release);
    VIDEO_SEEK_SECONDS.store(seek_seconds.clamp(1, 60), Ordering::Release);
}

fn point_inside_video_rect(point: POINT) -> bool {
    // A retained geometry is not proof of a visible player. In particular,
    // pinned minimization must leave wheel input to the app now underneath it.
    unsafe {
        let overlay = OVERLAY_HWND.load(Ordering::Acquire) as HWND;
        if PREVIEW_PINNED.load(Ordering::Acquire) && IsIconic(overlay) != 0 {
            return false;
        }
        let player = current_video_window();
        if player.is_null() || IsWindowVisible(player) == 0 {
            return false;
        }
    }
    let left = VIDEO_RECT_LEFT.load(Ordering::Acquire);
    let top = VIDEO_RECT_TOP.load(Ordering::Acquire);
    let right = VIDEO_RECT_RIGHT.load(Ordering::Acquire);
    let bottom = VIDEO_RECT_BOTTOM.load(Ordering::Acquire);
    left < right
        && top < bottom
        && point.x >= left
        && point.x < right
        && point.y >= top
        && point.y < bottom
}

fn clear_video_window_state() {
    VIDEO_HWND.store(0, Ordering::Release);
    VIDEO_PID.store(0, Ordering::Release);
    VIDEO_RECT_LEFT.store(0, Ordering::Release);
    VIDEO_RECT_TOP.store(0, Ordering::Release);
    VIDEO_RECT_RIGHT.store(0, Ordering::Release);
    VIDEO_RECT_BOTTOM.store(0, Ordering::Release);
}

fn load_settings() -> Settings {
    let Some(path) = settings_path() else {
        return Settings::default();
    };
    fs::read_to_string(path)
        .map(|text| Settings::parse_lossy(&text))
        .unwrap_or_default()
}

fn save_settings(settings: &Settings) {
    let Some(path) = settings_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = File::create(path)
        .and_then(|mut file| file.write_all(settings_to_ini(settings).as_bytes()));
}

fn settings_path() -> Option<PathBuf> {
    if crate::agent::is_session() {
        return None;
    }
    env::var_os("APPDATA")
        .map(PathBuf::from)
        .map(|base| base.join("Hover").join("settings.ini"))
}

fn settings_to_ini(settings: &Settings) -> String {
    format!(
        concat!(
            "[general]\n",
            "enabled = {}\n",
            "show_tray_icon = {}\n\n",
            "[preview_types]\n",
            "images = {}\n",
            "videos = {}\n",
            "audio = {}\n",
            "folders = {}\n",
            "zips = {}\n",
            "text = {}\n",
            "sims_packages = {}\n\n",
            "[video]\n",
            "audio_enabled = {}\n",
            "seek_seconds = {}\n"
        ),
        settings.general.enabled,
        settings.general.show_tray_icon,
        settings.preview_types.images,
        settings.preview_types.videos,
        settings.preview_types.audio,
        settings.preview_types.folders,
        settings.preview_types.zips,
        settings.preview_types.text,
        settings.preview_types.sims_packages,
        settings.video.audio_enabled,
        settings.video.seek_seconds
    )
}

fn is_video_control_key(key: u32) -> bool {
    matches!(key as u16, VK_SPACE | VK_LEFT | VK_RIGHT | VK_UP | VK_DOWN)
}

unsafe fn seek_active_video_by_wheel(wheel_delta: i32) -> bool {
    if wheel_delta == 0 || VIDEO_PID.load(Ordering::Acquire) == 0 {
        return false;
    }
    let notches = (wheel_delta.unsigned_abs() / WHEEL_DELTA).clamp(1, 6);
    let key = (if wheel_delta > 0 { VK_RIGHT } else { VK_LEFT }) as u32;
    let mut sent = false;
    for _ in 0..notches {
        sent |= post_video_key_tap(key);
    }
    sent
}

unsafe fn post_video_key_tap(key: u32) -> bool {
    let hwnd = current_video_window();
    if hwnd.is_null() {
        return false;
    }
    let down = PostMessageW(hwnd, WM_KEYDOWN, key as WPARAM, 1) != 0;
    let up = PostMessageW(
        hwnd,
        WM_KEYUP,
        key as WPARAM,
        (1 | (1 << 30) | (1 << 31)) as LPARAM,
    ) != 0;
    down || up
}

unsafe fn forward_video_key(message: u32, keyboard: &KBDLLHOOKSTRUCT) -> bool {
    let pid = VIDEO_PID.load(Ordering::Acquire);
    if pid == 0 || foreground_process_id() != Some(pid) {
        return false;
    }
    let hwnd = current_video_window();
    if hwnd.is_null() {
        return false;
    }
    let repeat = 1u32;
    let scan_code = (keyboard.scanCode & 0xff) << 16;
    let key_was_down = matches!(message, WM_KEYUP | WM_SYSKEYUP) as u32;
    let transition = key_was_down << 31;
    let lparam = (repeat | scan_code | transition) as LPARAM;
    PostMessageW(hwnd, message, keyboard.vkCode as WPARAM, lparam) != 0
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

unsafe fn set_startup_enabled(enabled: bool) -> bool {
    if crate::agent::is_session() {
        return false;
    }
    if enabled {
        enable_startup()
    } else {
        disable_startup();
        false
    }
}

unsafe fn enable_startup() -> bool {
    let Some(exe_path) = std::env::current_exe()
        .ok()
        .map(|path| format!("\"{}\"", path.display()))
    else {
        return startup_enabled();
    };
    with_startup_key(KEY_SET_VALUE, |hkey| {
        let value_name = wide(STARTUP_VALUE_NAME);
        let exe_path = wide(&exe_path);
        RegSetValueExW(
            hkey,
            value_name.as_ptr(),
            0,
            REG_SZ,
            exe_path.as_ptr() as *const u8,
            (exe_path.len() * std::mem::size_of::<u16>()) as u32,
        ) == ERROR_SUCCESS
    })
    .unwrap_or_else(|| startup_enabled())
}

unsafe fn disable_startup() {
    let _ = with_startup_key(KEY_SET_VALUE, |hkey| {
        let value_name = wide(STARTUP_VALUE_NAME);
        let result = RegDeleteValueW(hkey, value_name.as_ptr());
        result == ERROR_SUCCESS
    });
}

unsafe fn startup_enabled() -> bool {
    if crate::agent::is_session() {
        return false;
    }
    with_startup_key(KEY_READ, |hkey| {
        let value_name = wide(STARTUP_VALUE_NAME);
        RegQueryValueExW(
            hkey,
            value_name.as_ptr(),
            null(),
            null_mut(),
            null_mut(),
            null_mut(),
        ) == ERROR_SUCCESS
    })
    .unwrap_or(false)
}

unsafe fn with_startup_key<T>(access: u32, callback: impl FnOnce(HKEY) -> T) -> Option<T> {
    let key_path = wide(STARTUP_KEY);
    let mut hkey: HKEY = null_mut();
    if RegOpenKeyExW(HKEY_CURRENT_USER, key_path.as_ptr(), 0, access, &mut hkey) != ERROR_SUCCESS {
        return None;
    }
    let result = callback(hkey);
    RegCloseKey(hkey);
    Some(result)
}

unsafe fn register_window_class(
    hinstance: *mut c_void,
    class_name: &str,
    proc: windows_sys::Win32::UI::WindowsAndMessaging::WNDPROC,
) -> Result<(), i32> {
    let class = wide(class_name);
    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: proc,
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinstance,
        // Win32 MAKEINTRESOURCE(1) is a resource identifier, not a pointer
        // that LoadIcon dereferences.
        hIcon: LoadIconW(hinstance, std::ptr::without_provenance(1)),
        hCursor: null_mut(),
        hbrBackground: null_mut(),
        lpszMenuName: null(),
        lpszClassName: class.as_ptr(),
        hIconSm: LoadIconW(hinstance, std::ptr::without_provenance(1)),
    };
    if RegisterClassExW(&wc) == 0 {
        return Err(1);
    }
    Ok(())
}

unsafe fn foreground_is_eligible() -> bool {
    let hwnd = GetForegroundWindow();
    if hwnd.is_null() {
        return false;
    }
    let class = window_class_name(hwnd);
    matches!(
        class.as_str(),
        "CabinetWClass" | "ExploreWClass" | "Progman" | "WorkerW" | "#32770"
    )
}

unsafe fn foreground_process_id() -> Option<u32> {
    let hwnd = GetForegroundWindow();
    if hwnd.is_null() {
        return None;
    }
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, &mut pid);
    (pid != 0).then_some(pid)
}

unsafe fn fullscreen_or_ineligible_foreground() -> bool {
    let hwnd = GetForegroundWindow();
    if hwnd.is_null() || foreground_is_eligible() {
        return false;
    }
    let mut rect = RECT::default();
    if GetWindowRect(hwnd, &mut rect) == 0 {
        return false;
    }
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    let screen_width = windows_sys::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
        windows_sys::Win32::UI::WindowsAndMessaging::SM_CXSCREEN,
    );
    let screen_height = windows_sys::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
        windows_sys::Win32::UI::WindowsAndMessaging::SM_CYSCREEN,
    );
    width >= screen_width && height >= screen_height
}

unsafe fn window_class_name(hwnd: HWND) -> String {
    let mut buf = [0u16; 256];
    let len = GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
    String::from_utf16_lossy(&buf[..len.max(0) as usize])
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn copy_wide_fixed(value: &str, dest: &mut [u16]) {
    let encoded: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
    let count = encoded.len().min(dest.len());
    dest[..count].copy_from_slice(&encoded[..count]);
}

const fn rgb(red: u8, green: u8, blue: u8) -> u32 {
    red as u32 | ((green as u32) << 8) | ((blue as u32) << 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image_model(width: u32, height: u32) -> OverlayModel {
        OverlayModel {
            content: OverlayContent::Frames {
                frames: vec![OverlayFrame {
                    width,
                    height,
                    stride: width * 4,
                    pixels_bgra: Vec::new(),
                    delay_ms: 0,
                }],
                zoom: ZoomState::new(
                    Size::new(width as f32, height as f32),
                    Size::new(400.0, 300.0),
                    ZoomConfig::default(),
                ),
            },
            current_frame: 0,
            scroll_offset: 0,
        }
    }

    #[test]
    fn reload_preserves_actual_custom_zoom_pan_and_clamps_animation_frame() {
        let mut old = image_model(1000, 800);
        if let OverlayContent::Frames { zoom, .. } = &mut old.content {
            zoom.set_mode(hover_core::ScaleMode::Native, ZoomConfig::default());
            zoom.pan_by(120.0, -80.0, ZoomConfig::default());
        }
        old.current_frame = 8;
        let saved = old.save_view();
        let mut new = image_model(1000, 800);
        assert!(new.restore_view(saved, Size::new(500.0, 400.0), 10));
        if let OverlayContent::Frames { zoom, .. } = new.content {
            assert_eq!(zoom.scale, 1.0);
            assert_eq!((zoom.pan_x, zoom.pan_y), (120.0, -80.0));
            assert_eq!(zoom.viewport_size, Size::new(500.0, 400.0));
        } else {
            panic!("image view lost");
        }
        assert_eq!(new.current_frame, 0);
    }

    #[test]
    fn changed_dimensions_or_content_kind_do_not_inherit_stale_view() {
        let old = image_model(1000, 800).save_view();
        let mut changed = image_model(200, 100);
        assert!(!changed.restore_view(old, Size::new(400.0, 300.0), 10));
        let mut text = OverlayModel {
            content: OverlayContent::Text {
                title: String::new(),
                subtitle: String::new(),
                lines: vec!["new".into()],
                truncated: false,
            },
            current_frame: 0,
            scroll_offset: 0,
        };
        assert!(!text.restore_view(old, Size::new(400.0, 300.0), 10));
        assert_eq!(text.scroll_offset, 0);
    }

    #[test]
    fn text_and_list_reload_keep_scroll_only_within_the_new_content() {
        let mut text = OverlayModel {
            content: OverlayContent::Text {
                title: String::new(),
                subtitle: String::new(),
                lines: vec!["line".into(); 50],
                truncated: false,
            },
            current_frame: 0,
            scroll_offset: 0,
        };
        assert!(text.restore_view(SavedView::Text { scroll: 99 }, Size::new(500.0, 300.0), 10));
        assert_eq!(text.scroll_offset, 40);
        let mut list = OverlayModel {
            content: OverlayContent::List {
                title: String::new(),
                subtitle: String::new(),
                entries: Vec::new(),
                omitted: 0,
            },
            current_frame: 0,
            scroll_offset: 99,
        };
        assert!(list.restore_view(SavedView::List { scroll: 99 }, Size::new(500.0, 300.0), 10));
        assert_eq!(list.scroll_offset, 0);
    }

    #[test]
    #[ignore = "Dedicated hosted Windows runner only: creates Hover's own native window; not an Explorer or physical-input test"]
    fn pinned_native_window_lifecycle() {
        assert_eq!(
            std::env::var("GITHUB_ACTIONS").as_deref(),
            Ok("true"),
            "Do not open test windows on the user's shared desktop"
        );
        std::thread::spawn(|| unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowTextW, PM_REMOVE, PeekMessageW, WM_QUIT};
            fn caption(hwnd: HWND) -> String {
                let mut text = [0u16; 1024];
                let count = unsafe { GetWindowTextW(hwnd, text.as_mut_ptr(), text.len() as i32) };
                String::from_utf16_lossy(&text[..count.max(0) as usize])
            }
            unsafe fn pump_until(mut predicate: impl FnMut() -> bool) {
                let start = Instant::now();
                loop {
                    let mut msg = MSG::default();
                    while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                        assert_ne!(msg.message, WM_QUIT, "Closing preview quit the resident app");
                        if !preprocess_preview_message(&msg) { TranslateMessage(&msg); DispatchMessageW(&msg); }
                    }
                    if predicate() { return; }
                    assert!(start.elapsed() < Duration::from_secs(10), "Native preview action did not settle");
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
            struct Cleanup(HWND, PathBuf);
            impl Drop for Cleanup {
                fn drop(&mut self) {
                    unsafe { if IsWindow(self.0) != 0 { DestroyWindow(self.0); } }
                    let _ = std::fs::remove_file(&self.1);
                }
            }
            let path = std::env::temp_dir().join(format!("hover-native-preview-{}.txt", std::process::id()));
            let initial = (0..100).map(|i| format!("initial {i}\n")).collect::<String>();
            let mut fixture = std::fs::OpenOptions::new().write(true).create_new(true).open(&path).unwrap();
            fixture.write_all(initial.as_bytes()).unwrap();
            drop(fixture);
            let control = create_app_window().unwrap();
            let _cleanup = Cleanup(control, path.clone());
            let ptr = GetWindowLongPtrW(control, windows_sys::Win32::UI::WindowsAndMessaging::GWLP_USERDATA) as *mut App;
            let overlay = (*ptr).overlay;
            (*ptr).preview_anchor = Some(HoverPoint::new(100.0, 100.0));
            (*ptr).show_payload(path.clone(), PreviewPayload::Text { title: "fixture".into(), subtitle: "native test".into(), lines: initial.lines().map(str::to_string).collect(), truncated: false });
            assert_ne!(GetWindowLongPtrW(overlay, GWL_EXSTYLE) as u32 & WS_EX_NOACTIVATE, 0);
            assert_eq!(caption(GetDlgItem(overlay, ID_PREVIEW_PIN as i32)), "Pin");
            SendMessageW(overlay, WM_COMMAND, ID_PREVIEW_PIN, 0);
            pump_until(|| (*ptr).pinned);
            assert_eq!(GetWindowLongPtrW(overlay, GWL_EXSTYLE) as u32 & WS_EX_NOACTIVATE, 0);
            assert_ne!(GetWindowLongPtrW(overlay, GWL_STYLE) as u32 & WS_CAPTION, 0);
            assert_eq!(caption(GetDlgItem(overlay, ID_PREVIEW_PIN as i32)), "Unpin");
            for target in [GetDlgItem(overlay, ID_PREVIEW_PIN as i32), GetDlgItem(overlay, ID_PREVIEW_RELOAD as i32), GetDlgItem(overlay, ID_PREVIEW_CLOSE as i32), overlay] {
                PostMessageW(GetFocus(), WM_KEYDOWN, 0x09, 0);
                pump_until(|| GetFocus() == target);
            }
            cycle_preview_focus(overlay, true);
            assert_eq!(GetFocus(), GetDlgItem(overlay, ID_PREVIEW_CLOSE as i32));
            cycle_preview_focus(overlay, false);
            assert_eq!(GetFocus(), overlay);
            (*ptr).on_foreground_changed();
            (*ptr).update_hover_target();
            assert_eq!((*ptr).current_path.as_ref(), Some(&path));
            OVERLAY_MODEL.get().unwrap().lock().unwrap().as_mut().unwrap().scroll_offset = 20;
            let changed = (0..100).map(|i| format!("updated {i}\n")).collect::<String>();
            std::fs::write(&path, &changed).unwrap();
            PostMessageW(overlay, WM_KEYDOWN, 0x74, 0); // F5 through the production message path.
            pump_until(|| (*ptr).pending_path.is_none() && caption(overlay).contains("View restored"));
            assert!((*ptr).pinned);
            assert_eq!((*ptr).state, HoverState::PreviewLocked);
            {
                let guard = OVERLAY_MODEL.get().unwrap().lock().unwrap();
                let model = guard.as_ref().unwrap();
                assert_eq!(model.scroll_offset, 20);
                assert!(matches!(&model.content, OverlayContent::Text { lines, .. } if lines[0] == "updated 0"));
            }
            assert_eq!(std::fs::read(&path).unwrap(), changed.as_bytes());
            // Exercise the wheel eligibility guard without launching a player
            // or injecting any physical input on the runner.
            let media_rect = overlay_media_rect(overlay);
            store_video_control_state(media_rect, 5);
            VIDEO_PID.store(std::process::id(), Ordering::Release);
            VIDEO_HWND.store(overlay as isize, Ordering::Release);
            let media_point = POINT { x: media_rect.left as i32 + 1, y: media_rect.top as i32 + 1 };
            assert!(point_inside_video_rect(media_point));
            ShowWindow(overlay, windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWMINNOACTIVE);
            assert!(IsIconic(overlay) != 0);
            assert!(!point_inside_video_rect(media_point), "Minimized preview captured wheel input");
            clear_video_window_state();
            PostMessageW(overlay, WM_KEYDOWN, 0x74, 0);
            pump_until(|| (*ptr).pending_path.is_none() && caption(overlay).contains("View restored"));
            assert!(IsIconic(overlay) != 0, "Reload completion restored a minimized preview");
            PostMessageW(overlay, WM_CLOSE, 0, 0);
            pump_until(|| (*ptr).current_path.is_none());
            assert!(IsWindow(control) != 0 && IsWindow(overlay) != 0);
            assert_eq!(IsWindowVisible(overlay), 0);
            DestroyWindow(control);
            assert_eq!(IsWindow(overlay), 0);
            assert_eq!(CONTROL_HWND.load(Ordering::Acquire), 0);
        }).join().unwrap();
    }
}
