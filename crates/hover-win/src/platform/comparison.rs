//! A real native comparison window. Shared immutable decoded data stays on one
//! bounded replacement worker; all interaction/model mutation stays on the UI
//! thread. Rc + interior state avoids unique references across Win32 callbacks.
use super::comparison_data::{self as data, Content, Event, Job, MAX_PANES, PairData, PaneData};
use super::*;
use crossbeam_channel::{Receiver, bounded};
use hover_core::comparison::{frame_at, link_view};
use serde_json::{Value, json};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use windows_sys::Win32::Graphics::Gdi::{
    CreateFontW, DT_VCENTER, DrawFocusRect, FW_NORMAL, SelectObject,
};
use windows_sys::Win32::UI::Controls::{
    DRAWITEMSTRUCT, ICC_WIN95_CLASSES, INITCOMMONCONTROLSEX, InitCommonControlsEx, NMHDR,
    NMTTDISPINFOW, ODS_FOCUS, TOOLTIPS_CLASSW, TTF_IDISHWND, TTF_SUBCLASS, TTM_ADDTOOLW,
    TTM_GETTEXTW, TTM_POP, TTM_SETMAXTIPWIDTH, TTN_GETDISPINFOW, TTS_ALWAYSTIP, TTS_NOPREFIX,
    TTTOOLINFOW,
};
use windows_sys::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForWindow, SetThreadDpiAwarenessContext,
};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const CLASS: &str = "HoverComparisonWindow";
const READY: u32 = WM_APP + 31;
pub(super) const OPEN_SINGLE: u32 = WM_APP + 32;
pub(super) const CLOSED: u32 = WM_APP + 33;
const LINK: i32 = 3101;
const DIFFERENCE: i32 = 3102;
const PREVIOUS: i32 = 3103;
const PLAY: i32 = 3104;
const NEXT: i32 = 3105;
const RELOAD: i32 = 3106;
const SINGLE: i32 = 3107;
const CLOSE: i32 = 3108;
const PANE: i32 = 3200;
const TIMER: usize = 91;
const BUTTONS: [i32; 8] = [
    LINK, DIFFERENCE, PREVIOUS, PLAY, NEXT, RELOAD, SINGLE, CLOSE,
];
const BG: u32 = 0x001b1713;
const SURFACE: u32 = 0x0029221c;
const INK: u32 = 0x00f4eee9;
const MUTED: u32 = 0x00b9a99c;
const ACCENT: u32 = 0x00e6c56d;

#[derive(Clone, Copy)]
struct View {
    zoom: Option<ZoomState>,
    scroll: usize,
    frame: usize,
}
struct Pane {
    path: PathBuf,
    data: Option<Arc<PaneData>>,
    error: Option<String>,
    view: View,
    saved: Option<View>,
    rect: RECT,
    body: RECT,
    view_note: String,
    delays: Vec<u32>,
    time_offset: i64,
}
impl Pane {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            data: None,
            error: None,
            view: View {
                zoom: None,
                scroll: 0,
                frame: 0,
            },
            saved: None,
            rect: RECT::default(),
            body: RECT::default(),
            view_note: String::new(),
            delays: Vec::new(),
            time_offset: 0,
        }
    }
}
struct Model {
    panes: Vec<Pane>,
    pairs: Vec<Option<Arc<PairData>>>,
    active: usize,
    linked: bool,
    difference: bool,
    playing: bool,
    accumulated_ms: u64,
    running_since: Option<Instant>,
    loading: bool,
    generation: u64,
    worker_generation: u64,
    selected_total: usize,
    settings: PreviewTypeSettings,
    pending_single: Option<PathBuf>,
}
struct Font(isize);
impl Drop for Font {
    fn drop(&mut self) {
        unsafe {
            DeleteObject(self.0 as _);
        }
    }
}
struct State {
    hwnd: Cell<HWND>,
    parent: HWND,
    model: RefCell<Model>,
    worker: PreviewWorker<Job, ()>,
    events: Receiver<Event>,
    notify: Arc<AtomicIsize>,
    font: RefCell<Font>,
    tooltip: Cell<HWND>,
    tooltip_text: RefCell<Vec<Vec<u16>>>,
    dpi: Cell<u32>,
    drag: Cell<Option<(i32, i32)>>,
}
pub(super) struct Session {
    state: Rc<State>,
}

impl Session {
    pub(super) unsafe fn new(parent: HWND) -> Result<Self, String> {
        let (send, events) = bounded(16);
        let notify = Arc::new(AtomicIsize::new(0));
        let notify_worker = Arc::clone(&notify);
        let notify_done = Arc::clone(&notify);
        let worker = PreviewWorker::new(
            move |job, token| {
                data::run(job, token, &send, &|| {
                    let hwnd = notify_worker.load(Ordering::Acquire) as HWND;
                    if !hwnd.is_null() {
                        PostMessageW(hwnd, READY, 0, 0);
                    }
                })
            },
            move || {
                let hwnd = notify_done.load(Ordering::Acquire) as HWND;
                if !hwnd.is_null() {
                    PostMessageW(hwnd, READY, 0, 0);
                }
            },
        )
        .map_err(|e| e.to_string())?;
        let state = Rc::new(State {
            hwnd: Cell::new(null_mut()),
            parent,
            model: RefCell::new(Model {
                panes: Vec::new(),
                pairs: Vec::new(),
                active: 1,
                linked: true,
                difference: false,
                playing: false,
                accumulated_ms: 0,
                running_since: None,
                loading: false,
                generation: 0,
                worker_generation: 0,
                selected_total: 0,
                settings: PreviewTypeSettings::default(),
                pending_single: None,
            }),
            worker,
            events,
            notify,
            font: RefCell::new(Font(0)),
            tooltip: Cell::new(null_mut()),
            tooltip_text: RefCell::new((0..MAX_PANES).map(|_| Vec::new()).collect()),
            dpi: Cell::new(96),
            drag: Cell::new(None),
        });
        let instance = GetModuleHandleW(null());
        let class = wide(CLASS);
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hIcon: LoadIconW(instance, std::ptr::without_provenance(1)),
            hIconSm: LoadIconW(instance, std::ptr::without_provenance(1)),
            lpszClassName: class.as_ptr(),
            ..Default::default()
        };
        if RegisterClassExW(&wc) == 0 && windows_sys::Win32::Foundation::GetLastError() != 1410 {
            return Err("Cannot register native comparison window".into());
        }
        let old_dpi = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let work = screen_work_area(current_cursor_point());
        let initial_width = 1100.min(work.width() as i32).max(1);
        let initial_height = 760.min(work.height() as i32).max(1);
        // Session owns this Rc until its Drop destroys the window. Window
        // callbacks take only temporary strong references; no window/Rc cycle.
        let raw = Rc::as_ptr(&state);
        let hwnd = CreateWindowExW(
            WS_EX_APPWINDOW,
            class.as_ptr(),
            wide("Compare files · Hover").as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
            work.left as i32 + (work.width() as i32 - initial_width) / 2,
            work.top as i32 + (work.height() as i32 - initial_height) / 2,
            initial_width,
            initial_height,
            null_mut(),
            null_mut(),
            instance,
            raw.cast(),
        );
        if hwnd.is_null() {
            if !old_dpi.is_null() {
                SetThreadDpiAwarenessContext(old_dpi);
            }
            return Err("Cannot create native comparison window".into());
        }
        state.hwnd.set(hwnd);
        state.notify.store(hwnd as isize, Ordering::Release);
        state.dpi.set(GetDpiForWindow(hwnd).max(96));
        let width = state.unit(1100).min(work.width() as i32).max(1);
        let height = state.unit(760).min(work.height() as i32).max(1);
        SetWindowPos(
            hwnd,
            null_mut(),
            work.left as i32 + (work.width() as i32 - width) / 2,
            work.top as i32 + (work.height() as i32 - height) / 2,
            width,
            height,
            SWP_NOACTIVATE | SWP_NOZORDER,
        );
        state.set_font();
        let dark = 1i32;
        windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute(
            hwnd,
            20,
            (&dark as *const i32).cast(),
            4,
        );
        let common = INITCOMMONCONTROLSEX {
            dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_WIN95_CLASSES,
        };
        InitCommonControlsEx(&common);
        let tooltip = CreateWindowExW(
            WS_EX_TOPMOST,
            TOOLTIPS_CLASSW,
            null(),
            WS_POPUP | TTS_ALWAYSTIP | TTS_NOPREFIX,
            0,
            0,
            0,
            0,
            hwnd,
            null_mut(),
            instance,
            null(),
        );
        state.tooltip.set(tooltip);
        if !tooltip.is_null() {
            SendMessageW(tooltip, TTM_SETMAXTIPWIDTH, 0, state.unit(640) as isize);
        }
        for (id, label) in [
            (LINK, "Linked views"),
            (DIFFERENCE, "Differences"),
            (PREVIOUS, "Previous"),
            (PLAY, "Play"),
            (NEXT, "Next"),
            (RELOAD, "Reload"),
            (SINGLE, "Open alone"),
            (CLOSE, "Close"),
        ] {
            state.create_button(id, label);
        }
        for i in 0..MAX_PANES {
            state.create_button(PANE + i as i32, "");
        }
        state.layout();
        if !old_dpi.is_null() {
            SetThreadDpiAwarenessContext(old_dpi);
        }
        Ok(Self { state })
    }
    pub(super) fn visible(&self) -> bool {
        unsafe { IsWindowVisible(self.state.hwnd.get()) != 0 }
    }
    pub(super) fn loading(&self) -> bool {
        self.visible() && self.state.model.borrow().loading
    }
    pub(super) fn open(
        &self,
        paths: Vec<PathBuf>,
        selected_total: usize,
        settings: PreviewTypeSettings,
        activate: bool,
    ) {
        unsafe {
            self.state.replace(paths, selected_total, settings, false);
            ShowWindow(self.state.hwnd.get(), SW_SHOWNOACTIVATE);
            if activate {
                SetForegroundWindow(self.state.hwnd.get());
                SetFocus(GetDlgItem(self.state.hwnd.get(), PANE + 1));
            }
            self.state.layout();
        }
    }
    pub(super) fn close(&self) {
        unsafe {
            self.state.close();
        }
    }
    pub(super) fn take_single(&self) -> Option<PathBuf> {
        self.state.model.borrow_mut().pending_single.take()
    }
    pub(super) unsafe fn preprocess(&self, message: &MSG) -> bool {
        let hwnd = self.state.hwnd.get();
        if !self.visible() || !(message.hwnd == hwnd || IsChild(hwnd, message.hwnd) != 0) {
            return false;
        }
        if message.message == WM_KEYDOWN {
            let control = GetKeyState(VK_CONTROL as i32) < 0;
            match message.wParam as u32 {
                0x09 => {
                    self.state.cycle_focus(GetKeyState(VK_SHIFT as i32) < 0);
                    return true;
                }
                0x1b => {
                    self.state.close();
                    return true;
                }
                0x74 => {
                    self.state.reload();
                    return true;
                }
                0x4c if control => {
                    self.state.toggle_link();
                    return true;
                }
                key @ 0x31..=0x34 if control => {
                    let index = (key - 0x31) as usize;
                    if index < self.state.model.borrow().panes.len() {
                        self.state.select(index);
                        SetFocus(GetDlgItem(hwnd, PANE + index as i32));
                    }
                    return true;
                }
                key if GetFocus() == hwnd
                    && matches!(key, 0x21..=0x28 | 0x6b | 0x6d | 0xbb | 0xbd) =>
                {
                    self.state.navigate(key);
                    return true;
                }
                _ => {}
            }
        }
        IsDialogMessageW(hwnd, message) != 0
    }
    pub(super) fn status(&self, details: bool, offset: usize, limit: usize) -> Value {
        let mut value = self.state.status(details, offset, limit);
        value["animation_running"] = json!(self.state.model.borrow().running_since.is_some());
        value["content_focused"] = json!(unsafe { GetFocus() == self.state.hwnd.get() });
        value["no_activate"] = json!(unsafe {
            GetWindowLongPtrW(self.state.hwnd.get(), GWL_EXSTYLE) as u32 & WS_EX_NOACTIVATE != 0
        });
        value["caption"] = json!(unsafe { agent_control::window_text(self.state.hwnd.get()) });
        let mut client = RECT::default();
        unsafe {
            GetClientRect(self.state.hwnd.get(), &mut client);
        }
        value["client_width"] = json!(client.right);
        value["client_height"] = json!(client.bottom);
        value
    }
    pub(super) unsafe fn command(&self, command: &Value) -> Result<Value, String> {
        use crate::agent::validate_fields;
        let action = command
            .get("action")
            .map(|value| value.as_str().ok_or("action must be a string"))
            .transpose()?
            .unwrap_or("status");
        let fields: &[&str] = match action {
            "status" | "close" | "reload" | "minimize" | "restore" => &["command", "action"],
            "select" => &["command", "action", "pane"],
            "link" | "difference" | "play" => &["command", "action", "value"],
            "zoom" => &["command", "action", "mode"],
            "pan" => &["command", "action", "dx", "dy"],
            "scroll" => &["command", "action", "offset"],
            "frame" => &["command", "action", "index"],
            "resize" => &["command", "action", "width", "height"],
            "inspect" => &["command", "action", "offset", "limit"],
            "focus" => &["command", "action", "target"],
            "capture" => &["command", "action", "path"],
            "invoke" => &["command", "action", "control"],
            "key" => &["command", "action", "key"],
            "tooltip" => &["command", "action", "pane"],
            _ => return Err("Unknown comparison action".into()),
        };
        validate_fields(command, fields)?;
        if action != "status" && !self.visible() {
            return Err("No comparison is open".into());
        }
        let integer = |name: &str, min: u64, max: u64| {
            command
                .get(name)
                .and_then(Value::as_u64)
                .filter(|v| (min..=max).contains(v))
                .ok_or_else(|| format!("{name} must be an integer from {min} to {max}"))
        };
        let boolean = || {
            command
                .get("value")
                .and_then(Value::as_bool)
                .ok_or("value must be a boolean")
        };
        let number = |name: &str| {
            command
                .get(name)
                .and_then(Value::as_f64)
                .filter(|v| v.is_finite() && v.abs() <= 1_000_000.0)
                .map(|v| v as f32)
                .ok_or_else(|| format!("{name} must be finite and bounded"))
        };
        match action {
            "tooltip" => {
                let index = integer("pane", 0, 3)? as usize;
                if index >= self.state.model.borrow().panes.len() {
                    return Err("Pane does not exist".into());
                }
                if self.state.tooltip.get().is_null() {
                    return Err("Native tooltip control is unavailable".into());
                }
                let mut text = vec![0u16; 12_000];
                let info = TTTOOLINFOW {
                    cbSize: std::mem::offset_of!(TTTOOLINFOW, lpReserved) as u32,
                    hwnd: self.state.hwnd.get(),
                    uFlags: TTF_IDISHWND,
                    uId: GetDlgItem(self.state.hwnd.get(), PANE + index as i32) as usize,
                    lpszText: text.as_mut_ptr(),
                    ..Default::default()
                };
                SendMessageW(
                    self.state.tooltip.get(),
                    TTM_GETTEXTW,
                    text.len(),
                    (&info as *const TTTOOLINFOW) as isize,
                );
                let len = text.iter().position(|ch| *ch == 0).unwrap_or(text.len());
                return Ok(
                    json!({"pane":index,"text":String::from_utf16_lossy(&text[..len]),"source":"native_tooltip_control"}),
                );
            }
            "key" => {
                let key = match command.get("key").and_then(Value::as_str) {
                    Some("left") => 0x25,
                    Some("right") => 0x27,
                    Some("up") => 0x26,
                    Some("down") => 0x28,
                    Some("page_up") => 0x21,
                    Some("page_down") => 0x22,
                    Some("home") => 0x24,
                    Some("end") => 0x23,
                    Some("reload") => 0x74,
                    Some("escape") => 0x1b,
                    Some("tab") => 0x09,
                    Some("plus") => 0xbb,
                    Some("minus") => 0xbd,
                    _ => return Err("Unknown comparison key".into()),
                };
                PostMessageW(self.state.hwnd.get(), WM_KEYDOWN, key, 1);
            }
            "invoke" => {
                let id = match command.get("control").and_then(Value::as_str) {
                    Some("link") => LINK,
                    Some("differences") => DIFFERENCE,
                    Some("previous") => PREVIOUS,
                    Some("play") => PLAY,
                    Some("next") => NEXT,
                    Some("reload") => RELOAD,
                    Some("single") => SINGLE,
                    Some("close") => CLOSE,
                    _ => return Err("Unknown native comparison control".into()),
                };
                let control = GetDlgItem(self.state.hwnd.get(), id);
                if IsWindowEnabled(control) == 0 {
                    return Err("This native control is disabled for the current content".into());
                }
                SendMessageW(control, BM_CLICK, 0, 0);
            }
            "close" => self.state.close(),
            "reload" => self.state.reload(),
            "select" => {
                let index = integer("pane", 0, 3)? as usize;
                if index >= self.state.model.borrow().panes.len() {
                    return Err("Pane does not exist".into());
                }
                self.state.select(index);
            }
            "link" => {
                let value = boolean()?;
                self.state.model.borrow_mut().linked = value;
                self.state.synchronize();
                self.state.refresh();
            }
            "difference" => {
                self.state.model.borrow_mut().difference = boolean()?;
                self.state.refresh();
            }
            "play" => {
                let value = boolean()?;
                if value
                    && !self
                        .state
                        .model
                        .borrow()
                        .panes
                        .iter()
                        .any(|pane| pane.delays.len() > 1)
                {
                    return Err("This comparison has no decoded animation to play".into());
                }
                self.state.model.borrow_mut().playing = value;
                self.state.sync_timer();
                self.state.refresh();
            }
            "frame" => self.state.set_frame(integer("index", 0, 999)? as usize)?,
            "zoom" => {
                let mode = match command.get("mode").and_then(Value::as_str) {
                    Some("fit") => hover_core::ScaleMode::FitAll,
                    Some("native") => hover_core::ScaleMode::Native,
                    Some("fit_width") => hover_core::ScaleMode::FitWidth,
                    Some("fit_height") => hover_core::ScaleMode::FitHeight,
                    _ => return Err("Unknown image scale mode".into()),
                };
                self.state.zoom_mode(mode)?;
            }
            "pan" => self.state.pan(number("dx")?, number("dy")?)?,
            "scroll" => {
                let offset = integer("offset", 0, 20_000)? as usize;
                let mut model = self.state.model.borrow_mut();
                let active = model.active;
                if model.linked {
                    for pane in &mut model.panes {
                        pane.view.scroll = offset;
                    }
                } else {
                    model.panes[active].view.scroll = offset;
                }
                drop(model);
                self.state.clamp_scroll();
                self.state.refresh();
            }
            "resize" => {
                let width = integer("width", 400, 4096)? as i32;
                let height = integer("height", 300, 2160)? as i32;
                let mut rect = RECT::default();
                GetWindowRect(self.state.hwnd.get(), &mut rect);
                SetWindowPos(
                    self.state.hwnd.get(),
                    null_mut(),
                    rect.left,
                    rect.top,
                    width,
                    height,
                    SWP_NOACTIVATE | SWP_NOZORDER,
                );
            }
            "minimize" => {
                ShowWindow(self.state.hwnd.get(), SW_SHOWMINNOACTIVE);
            }
            "restore" => {
                ShowWindow(self.state.hwnd.get(), SW_RESTORE);
            }
            "focus" => {
                let target = command
                    .get("target")
                    .and_then(Value::as_str)
                    .ok_or("target must be pane, next or previous")?;
                let hwnd = self.state.hwnd.get();
                if target == "content" {
                    SetFocus(hwnd);
                } else if target == "pane" {
                    SetFocus(GetDlgItem(
                        hwnd,
                        PANE + self.state.model.borrow().active as i32,
                    ));
                } else if matches!(target, "next" | "previous") {
                    self.state.cycle_focus(target == "previous");
                } else {
                    return Err("target must be content, pane, next or previous".into());
                }
            }
            "capture" => {
                let path = PathBuf::from(
                    command
                        .get("path")
                        .and_then(Value::as_str)
                        .ok_or("path must be a string")?,
                );
                let mut controls = BUTTONS.to_vec();
                controls
                    .extend((0..self.state.model.borrow().panes.len()).map(|i| PANE + i as i32));
                return agent_control::capture_window(
                    self.state.hwnd.get(),
                    &path,
                    paint,
                    &controls,
                );
            }
            "inspect" => {
                return Ok(self.status(
                    true,
                    command
                        .get("offset")
                        .map(|_| integer("offset", 0, 20_000))
                        .transpose()?
                        .unwrap_or(0) as usize,
                    command
                        .get("limit")
                        .map(|_| integer("limit", 1, 128))
                        .transpose()?
                        .unwrap_or(64) as usize,
                ));
            }
            _ => {}
        }
        Ok(self.status(false, 0, 0))
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.state.worker.cancel();
        self.state.notify.store(0, Ordering::Release);
        unsafe {
            if !self.state.tooltip.get().is_null() {
                DestroyWindow(self.state.tooltip.replace(null_mut()));
            }
            if IsWindow(self.state.hwnd.get()) != 0 {
                DestroyWindow(self.state.hwnd.get());
            }
        }
    }
}

impl State {
    fn unit(&self, value: i32) -> i32 {
        value * self.dpi.get() as i32 / 96
    }
    unsafe fn create_button(&self, id: i32, label: &str) {
        let hwnd = CreateWindowExW(
            0,
            wide("BUTTON").as_ptr(),
            wide(label).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_OWNERDRAW as u32,
            0,
            0,
            0,
            0,
            self.hwnd.get(),
            id as isize as _,
            GetModuleHandleW(null()),
            null(),
        );
        SendMessageW(hwnd, WM_SETFONT, self.font.borrow().0 as usize, 1);
        if (PANE..PANE + MAX_PANES as i32).contains(&id) && !self.tooltip.get().is_null() {
            let info = TTTOOLINFOW {
                // The application does not request a v6 common-controls
                // activation context. Use the SDK's TOOLINFO_V2_SIZE layout;
                // the final reserved pointer is not part of the v5 ABI.
                cbSize: std::mem::offset_of!(TTTOOLINFOW, lpReserved) as u32,
                uFlags: TTF_SUBCLASS | TTF_IDISHWND,
                hwnd: self.hwnd.get(),
                uId: hwnd as usize,
                lpszText: std::ptr::without_provenance_mut(usize::MAX),
                ..Default::default()
            };
            SendMessageW(
                self.tooltip.get(),
                TTM_ADDTOOLW,
                0,
                (&info as *const TTTOOLINFOW) as isize,
            );
        }
    }
    unsafe fn set_font(&self) {
        let face = wide("Segoe UI");
        let font = CreateFontW(
            -self.unit(14),
            0,
            0,
            0,
            FW_NORMAL as i32,
            0,
            0,
            0,
            1,
            0,
            0,
            5,
            0,
            face.as_ptr(),
        );
        let old = self.font.replace(Font(font as isize));
        for id in BUTTONS
            .into_iter()
            .chain((0..MAX_PANES).map(|i| PANE + i as i32))
        {
            let child = GetDlgItem(self.hwnd.get(), id);
            if !child.is_null() {
                SendMessageW(child, WM_SETFONT, font as usize, 1);
            }
        }
        drop(old);
    }
    unsafe fn replace(
        &self,
        paths: Vec<PathBuf>,
        selected_total: usize,
        settings: PreviewTypeSettings,
        reload: bool,
    ) {
        if !self.tooltip.get().is_null() {
            SendMessageW(self.tooltip.get(), TTM_POP, 0, 0);
        }
        self.worker.cancel();
        while self.events.try_recv().is_ok() {}
        let mut model = self.model.borrow_mut();
        let saved: Vec<_> = if reload {
            model
                .panes
                .iter()
                .map(|p| {
                    if p.data.is_some() {
                        p.view
                    } else {
                        p.saved.unwrap_or(p.view)
                    }
                })
                .collect()
        } else {
            Vec::new()
        };
        let rectangles: Vec<_> = if reload {
            model
                .panes
                .iter()
                .map(|pane| (pane.rect, pane.body))
                .collect()
        } else {
            Vec::new()
        };
        model.generation = model.generation.wrapping_add(1);
        let generation = model.generation;
        model.panes = paths
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, path)| {
                let mut pane = Pane::new(path);
                pane.saved = saved.get(i).copied();
                if let Some((rect, body)) = rectangles.get(i) {
                    pane.rect = *rect;
                    pane.body = *body;
                }
                pane
            })
            .collect();
        model.pairs = (0..paths.len()).map(|_| None).collect();
        model.active = model.active.min(paths.len() - 1);
        model.selected_total = selected_total;
        model.loading = true;
        model.playing = false;
        model.accumulated_ms = 0;
        model.running_since = None;
        model.settings = settings.clone();
        model.pending_single = None;
        model.worker_generation = self
            .worker
            .submit(Job {
                generation,
                paths,
                settings,
            })
            .0;
        drop(model);
        self.layout();
        self.sync_timer();
        self.refresh();
    }
    unsafe fn reload(&self) {
        let model = self.model.borrow();
        let paths = model.panes.iter().map(|p| p.path.clone()).collect();
        let total = model.selected_total;
        let settings = model.settings.clone();
        drop(model);
        self.replace(paths, total, settings, true);
    }
    unsafe fn close(&self) {
        if !self.tooltip.get().is_null() {
            SendMessageW(self.tooltip.get(), TTM_POP, 0, 0);
        }
        self.worker.cancel();
        while self.events.try_recv().is_ok() {}
        let mut model = self.model.borrow_mut();
        model.generation = model.generation.wrapping_add(1);
        model.loading = false;
        model.playing = false;
        model.panes.clear();
        model.pairs.clear();
        drop(model);
        if self.drag.take().is_some() {
            windows_sys::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
        }
        KillTimer(self.hwnd.get(), TIMER);
        ShowWindow(self.hwnd.get(), SW_HIDE);
        PostMessageW(self.parent, CLOSED, 0, 0);
    }
    unsafe fn poll(&self) {
        let mut model = self.model.borrow_mut();
        while let Ok(event) = self.events.try_recv() {
            match event {
                Event::Pane {
                    generation,
                    index,
                    result,
                } if generation == model.generation && index < model.panes.len() => {
                    let pane = &mut model.panes[index];
                    match result {
                        Ok(data) => {
                            if let Content::Images { frames } = &data.content
                                && let Some(first) = frames.first()
                            {
                                pane.delays = frames.iter().map(|f| f.delay_ms).collect();
                                let mut zoom = ZoomState::new(
                                    Size::new(first.width as f32, first.height as f32),
                                    Size::new(
                                        (pane.body.right - pane.body.left).max(1) as f32,
                                        (pane.body.bottom - pane.body.top).max(1) as f32,
                                    ),
                                    ZoomConfig::default(),
                                );
                                if let Some(View {
                                    zoom: Some(old),
                                    frame,
                                    ..
                                }) = pane.saved.take()
                                {
                                    if old.image_size == zoom.image_size {
                                        zoom = old;
                                        zoom.set_viewport(
                                            Size::new(
                                                (pane.body.right - pane.body.left).max(1) as f32,
                                                (pane.body.bottom - pane.body.top).max(1) as f32,
                                            ),
                                            ZoomConfig::default(),
                                        );
                                        pane.view.frame = frame.min(frames.len() - 1);
                                        pane.view_note = "View restored".into();
                                    } else {
                                        pane.view_note = "Dimensions changed; view reset".into();
                                    }
                                }
                                pane.view.zoom = Some(zoom);
                            } else if let Some(old) = pane.saved.take() {
                                pane.view.scroll = old.scroll;
                                pane.view_note = if old.zoom.is_none() {
                                    "Scroll restored"
                                } else {
                                    "Preview kind changed; view reset"
                                }
                                .into();
                            }
                            pane.data = Some(data);
                        }
                        Err(error) => {
                            pane.error = Some(error);
                            pane.view_note = "Reload to retry this file".into();
                        }
                    }
                }
                Event::Pair {
                    generation,
                    index,
                    result,
                } if generation == model.generation && index < model.pairs.len() => {
                    model.pairs[index] = Some(result)
                }
                _ => {}
            }
        }
        if let Some(result) = self.worker.take_result()
            && result.generation.0 == model.worker_generation
        {
            model.loading = false;
            if result.value.is_err() {
                for pane in &mut model.panes {
                    if pane.data.is_none() && pane.error.is_none() {
                        pane.error = Some("Comparison worker failed; reload to retry".into());
                    }
                }
            }
        }
        drop(model);
        self.clamp_scroll();
        self.sync_timer();
        self.refresh();
    }
    unsafe fn layout(&self) {
        let hwnd = self.hwnd.get();
        // Minimized windows can report an empty client rectangle. Keep the
        // last real viewport so a background reload cannot clamp away pan.
        if IsIconic(hwnd) != 0 {
            return;
        }
        let mut client = RECT::default();
        GetClientRect(hwnd, &mut client);
        let mut x = self.unit(16);
        let mut toolbar_y = self.unit(54);
        for (id, width) in [
            (LINK, 120),
            (DIFFERENCE, 108),
            (PREVIOUS, 80),
            (PLAY, 68),
            (NEXT, 60),
            (RELOAD, 80),
            (SINGLE, 100),
            (CLOSE, 68),
        ] {
            if x + self.unit(width) > client.right - self.unit(16) && x > self.unit(16) {
                x = self.unit(16);
                toolbar_y += self.unit(40);
            }
            SetWindowPos(
                GetDlgItem(hwnd, id),
                null_mut(),
                x,
                toolbar_y,
                self.unit(width),
                self.unit(32),
                SWP_NOACTIVATE | SWP_NOZORDER,
            );
            x += self.unit(width + 8);
        }
        let mut model = self.model.borrow_mut();
        let rows = if model.panes.len() > 2 { 2 } else { 1 };
        let margin = self.unit(16);
        let gap = self.unit(12);
        let top = toolbar_y + self.unit(52);
        let bottom = client.bottom - self.unit(116);
        let width = ((client.right - margin * 2 - gap) / 2).max(1);
        let height = ((bottom - top - gap * (rows - 1)) / rows).max(1);
        for (i, pane) in model.panes.iter_mut().enumerate() {
            let col = i as i32 % 2;
            let row = i as i32 / 2;
            let left = margin + col * (width + gap);
            let y = top + row * (height + gap);
            pane.rect = RECT {
                left,
                top: y,
                right: left + width,
                bottom: y + height,
            };
            let compact = height < self.unit(140);
            let content_top = if compact {
                self.unit(42)
            } else {
                self.unit(74)
            };
            let content_bottom = if compact { self.unit(8) } else { self.unit(26) };
            pane.body = RECT {
                left: left + self.unit(8),
                top: y + content_top.min((height - 1).max(0)),
                right: left + width - self.unit(8),
                bottom: y + (height - content_bottom).max(content_top.min((height - 1).max(0)) + 1),
            };
            if let Some(zoom) = pane.view.zoom.as_mut() {
                zoom.set_viewport(
                    Size::new(
                        (pane.body.right - pane.body.left).max(1) as f32,
                        (pane.body.bottom - pane.body.top).max(1) as f32,
                    ),
                    ZoomConfig::default(),
                );
            }
        }
        let labels: Vec<_> = model
            .panes
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let name = p.path.file_name().unwrap_or_default().to_string_lossy();
                (i, format!("{}  {name}", i + 1), p.rect)
            })
            .collect();
        let count = model.panes.len();
        drop(model);
        for i in 0..MAX_PANES {
            ShowWindow(
                GetDlgItem(hwnd, PANE + i as i32),
                if i < count { SW_SHOWNA } else { SW_HIDE },
            );
        }
        for (i, label, rect) in labels {
            SetWindowTextW(GetDlgItem(hwnd, PANE + i as i32), wide(&label).as_ptr());
            SetWindowPos(
                GetDlgItem(hwnd, PANE + i as i32),
                null_mut(),
                rect.left + self.unit(6),
                rect.top + self.unit(6),
                (rect.right - rect.left - self.unit(12)).max(1),
                self.unit(32),
                SWP_NOACTIVATE | SWP_NOZORDER,
            );
        }
        self.clamp_scroll();
        self.refresh();
    }
    unsafe fn refresh(&self) {
        let model = self.model.borrow();
        let labels = [
            (
                LINK,
                if model.linked {
                    "Linked views"
                } else {
                    "Independent"
                },
            ),
            (
                DIFFERENCE,
                if model.difference {
                    "Show originals"
                } else {
                    "Differences"
                },
            ),
            (PLAY, if model.playing { "Pause" } else { "Play" }),
        ];
        let animated = model.panes.iter().any(|pane| pane.delays.len() > 1);
        drop(model);
        for id in [PREVIOUS, PLAY, NEXT] {
            EnableWindow(GetDlgItem(self.hwnd.get(), id), i32::from(animated));
        }
        for (id, label) in labels {
            SetWindowTextW(GetDlgItem(self.hwnd.get(), id), wide(label).as_ptr());
        }
        InvalidateRect(self.hwnd.get(), null(), 0);
    }
    fn elapsed(model: &Model) -> u64 {
        model.accumulated_ms
            + model
                .running_since
                .map(|time| time.elapsed().as_millis().min(u64::MAX as u128) as u64)
                .unwrap_or(0)
    }
    unsafe fn sync_timer(&self) {
        let mut model = self.model.borrow_mut();
        let run = model.playing
            && model.panes.iter().any(|p| p.delays.len() > 1)
            && IsWindowVisible(self.hwnd.get()) != 0
            && IsIconic(self.hwnd.get()) == 0;
        if run {
            if model.running_since.is_none() {
                model.running_since = Some(Instant::now());
            }
            SetTimer(self.hwnd.get(), TIMER, 33, None);
        } else {
            if let Some(start) = model.running_since.take() {
                model.accumulated_ms = model
                    .accumulated_ms
                    .saturating_add(start.elapsed().as_millis().min(u64::MAX as u128) as u64);
            }
            KillTimer(self.hwnd.get(), TIMER);
        }
    }
    unsafe fn tick(&self) {
        let mut model = self.model.borrow_mut();
        let elapsed = Self::elapsed(&model);
        let mut changed = false;
        for pane in &mut model.panes {
            if pane.delays.len() > 1 {
                let time = (elapsed as i128 + i128::from(pane.time_offset))
                    .max(0)
                    .min(u64::MAX as i128) as u64;
                let frame = frame_at(time, &pane.delays);
                changed |= pane.view.frame != frame;
                pane.view.frame = frame;
            }
        }
        drop(model);
        if changed {
            InvalidateRect(self.hwnd.get(), null(), 0);
        }
    }
    unsafe fn select(&self, index: usize) {
        self.model.borrow_mut().active = index;
        self.refresh();
    }
    unsafe fn cycle_focus(&self, reverse: bool) {
        let hwnd = self.hwnd.get();
        let count = self.model.borrow().panes.len();
        let mut controls = BUTTONS
            .into_iter()
            .chain((0..count).map(|i| PANE + i as i32))
            .map(|id| GetDlgItem(hwnd, id))
            .filter(|child| IsWindowVisible(*child) != 0 && IsWindowEnabled(*child) != 0)
            .collect::<Vec<_>>();
        controls.push(hwnd);
        let current = GetFocus();
        let index = controls
            .iter()
            .position(|child| *child == current)
            .unwrap_or(controls.len() - 1);
        let next = if reverse {
            (index + controls.len() - 1) % controls.len()
        } else {
            (index + 1) % controls.len()
        };
        SetFocus(controls[next]);
    }
    unsafe fn toggle_link(&self) {
        let value = !self.model.borrow().linked;
        self.model.borrow_mut().linked = value;
        self.synchronize();
        self.refresh();
    }
    fn synchronize(&self) {
        let mut model = self.model.borrow_mut();
        if !model.linked {
            return;
        }
        let active = model.active;
        if let Some(source) = model.panes.get(active).and_then(|p| p.view.zoom) {
            for (index, pane) in model.panes.iter_mut().enumerate() {
                if index != active
                    && let Some(target) = pane.view.zoom.as_mut()
                {
                    link_view(source, target);
                }
            }
        } else if let Some(pane) = model.panes.get(active) {
            let scroll = pane.view.scroll;
            for pane in &mut model.panes {
                pane.view.scroll = scroll;
            }
        }
    }
    unsafe fn zoom_mode(&self, mode: hover_core::ScaleMode) -> Result<(), String> {
        let mut model = self.model.borrow_mut();
        let active = model.active;
        model
            .panes
            .get_mut(active)
            .and_then(|p| p.view.zoom.as_mut())
            .ok_or("Active pane is not a decoded image")?
            .set_mode(mode, ZoomConfig::default());
        drop(model);
        self.synchronize();
        self.refresh();
        Ok(())
    }
    unsafe fn pan(&self, dx: f32, dy: f32) -> Result<(), String> {
        let mut model = self.model.borrow_mut();
        let active = model.active;
        model
            .panes
            .get_mut(active)
            .and_then(|p| p.view.zoom.as_mut())
            .ok_or("Active pane is not a decoded image")?
            .pan_by(dx, dy, ZoomConfig::default());
        drop(model);
        self.synchronize();
        self.refresh();
        Ok(())
    }
    unsafe fn set_frame(&self, index: usize) -> Result<(), String> {
        let mut model = self.model.borrow_mut();
        let active = model.active;
        let count = model.panes.get(active).map(|p| p.delays.len()).unwrap_or(0);
        if index >= count {
            return Err("Frame does not exist in the active pane".into());
        }
        model.playing = false;
        let elapsed = Self::elapsed(&model);
        let linked = model.linked;
        for (i, pane) in model.panes.iter_mut().enumerate() {
            if i == active || linked {
                pane.view.frame = index.min(pane.delays.len().saturating_sub(1));
                let time = pane
                    .delays
                    .iter()
                    .take(pane.view.frame)
                    .map(|d| u64::from((*d).max(20)))
                    .sum::<u64>();
                pane.time_offset = time as i64 - elapsed.min(i64::MAX as u64) as i64;
            }
        }
        drop(model);
        self.sync_timer();
        self.refresh();
        Ok(())
    }
    fn clamp_scroll(&self) {
        let mut model = self.model.borrow_mut();
        let selected = model.active.max(1);
        let aligned = model
            .pairs
            .get(selected)
            .and_then(Option::as_ref)
            .and_then(|p| p.lines.as_ref())
            .map(Vec::len);
        for (index, pane) in model.panes.iter_mut().enumerate() {
            if let Some(data) = &pane.data
                && let Content::Rows { lines, .. } = &data.content
            {
                let rows = if index == 0 || index == selected {
                    aligned.unwrap_or(lines.len())
                } else {
                    lines.len()
                };
                pane.view.scroll = pane.view.scroll.min(rows.saturating_sub(1));
            }
        }
    }
    unsafe fn navigate(&self, key: u32) {
        let mut model = self.model.borrow_mut();
        let active = model.active;
        let Some(pane) = model.panes.get_mut(active) else {
            return;
        };
        if let Some(zoom) = pane.view.zoom.as_mut() {
            match key {
                0x25 => zoom.pan_by(40.0, 0.0, ZoomConfig::default()),
                0x27 => zoom.pan_by(-40.0, 0.0, ZoomConfig::default()),
                0x26 => zoom.pan_by(0.0, 40.0, ZoomConfig::default()),
                0x28 => zoom.pan_by(0.0, -40.0, ZoomConfig::default()),
                0x24 => zoom.set_mode(hover_core::ScaleMode::FitAll, ZoomConfig::default()),
                0x6b | 0xbb | 0x6d | 0xbd => {
                    let point = HoverPoint::new(
                        zoom.viewport_size.width / 2.0,
                        zoom.viewport_size.height / 2.0,
                    );
                    zoom.apply_wheel(
                        point,
                        if matches!(key, 0x6b | 0xbb) {
                            1.0
                        } else {
                            -1.0
                        },
                        WheelModifier::Plain,
                        ZoomConfig::default(),
                    );
                    zoom.settle_quality();
                }
                _ => {}
            }
        } else {
            let page = ((pane.body.bottom - pane.body.top) / self.unit(21)).max(1) as usize;
            pane.view.scroll = match key {
                0x26 => pane.view.scroll.saturating_sub(1),
                0x28 => pane.view.scroll.saturating_add(1),
                0x21 => pane.view.scroll.saturating_sub(page),
                0x22 => pane.view.scroll.saturating_add(page),
                0x24 => 0,
                0x23 => 20_000,
                _ => pane.view.scroll,
            };
        }
        drop(model);
        self.synchronize();
        self.clamp_scroll();
        self.refresh();
    }
    unsafe fn wheel(&self, wparam: WPARAM, lparam: LPARAM) {
        let mut point = POINT {
            x: signed_loword_isize(lparam),
            y: signed_hiword_isize(lparam),
        };
        ScreenToClient(self.hwnd.get(), &mut point);
        let mut model = self.model.borrow_mut();
        let index = model.panes.iter().position(|p| {
            point.x >= p.body.left
                && point.x < p.body.right
                && point.y >= p.body.top
                && point.y < p.body.bottom
        });
        let Some(index) = index else {
            return;
        };
        model.active = index;
        let notches = ((wparam >> 16) as u16 as i16) as f32 / 120.0;
        let pane = &mut model.panes[index];
        if let Some(zoom) = pane.view.zoom.as_mut() {
            if GetKeyState(VK_SHIFT as i32) < 0 {
                zoom.pan_by(notches * 40.0, 0.0, ZoomConfig::default());
            } else {
                zoom.apply_wheel(
                    HoverPoint::new(
                        (point.x - pane.body.left) as f32,
                        (point.y - pane.body.top) as f32,
                    ),
                    notches,
                    WheelModifier::Plain,
                    ZoomConfig::default(),
                );
                zoom.settle_quality();
            }
        } else {
            pane.view.scroll = pane
                .view
                .scroll
                .saturating_add_signed((-notches * 3.0) as isize);
        }
        drop(model);
        self.synchronize();
        self.clamp_scroll();
        self.refresh();
    }
    fn status(&self, details: bool, offset: usize, limit: usize) -> Value {
        let model = self.model.borrow();
        let active_pair = model.active.max(1);
        let pair = model.pairs.get(active_pair).and_then(Option::as_ref);
        let controls=BUTTONS.into_iter().chain((0..model.panes.len()).map(|i|PANE+i as i32)).map(|id|unsafe{let control=GetDlgItem(self.hwnd.get(),id);json!({"id":id,"label":agent_control::window_text(control),"focused":GetFocus()==control,"visible":IsWindowVisible(control)!=0})}).collect::<Vec<_>>();
        json!({"visible":unsafe{IsWindowVisible(self.hwnd.get())!=0},"minimized":unsafe{IsIconic(self.hwnd.get())!=0},"dpi":self.dpi.get(),"loading":model.loading,"generation":model.generation,"active_pane":model.active,"linked":model.linked,"difference":model.difference,"playing":model.playing,"selected_total":model.selected_total,"resident_payload_bytes":model.panes.iter().filter_map(|p|p.data.as_ref()).map(|d|d.resident_bytes).sum::<usize>(),"controls":controls,
        "panes":model.panes.iter().enumerate().map(|(index,pane)|{let mut value=json!({"index":index,"path":pane.path,"phase":if pane.error.is_some(){"error"}else if pane.data.is_some(){"ready"}else{"loading"},"error":pane.error,"frame":pane.view.frame,"scroll":pane.view.scroll,"view_note":pane.view_note,"rect":{"left":pane.rect.left,"top":pane.rect.top,"right":pane.rect.right,"bottom":pane.rect.bottom},"body":{"left":pane.body.left,"top":pane.body.top,"right":pane.body.right,"bottom":pane.body.bottom},"zoom":pane.view.zoom.map(|z|json!({"scale":z.scale,"pan_x":z.pan_x,"pan_y":z.pan_y,"width":z.image_size.width,"height":z.image_size.height,"viewport_width":z.viewport_size.width,"viewport_height":z.viewport_size.height}))});if let Some(data)=&pane.data{value["bytes"]=json!(data.bytes);value["stage"]=json!(data.stage);value["has_alpha"]=json!(data.has_alpha);value["modified_ns"]=json!(data.modified.map(|n|n.to_string()));value["note"]=json!(data.note);value["content"]=match &data.content{Content::Images{frames}=>json!({"kind":"image","frames":frames.len(),"delays":pane.delays}),Content::Rows{subtitle,lines,truncated}=>json!({"kind":"text","subtitle":subtitle,"rows":lines.len(),"truncated":truncated,"offset":offset,"lines":if details{lines.iter().skip(offset).take(limit).collect::<Vec<_>>()}else{Vec::new()}})};}value}).collect::<Vec<_>>(),
        "pair":pair.map(|pair|json!({"reference":0,"other":active_pair,"byte_scan":{"state":pair.byte_scan.state,"examined":pair.byte_scan.examined,"changed_positions":pair.byte_scan.changed,"note":pair.byte_scan.note},"note":pair.note,"timing_equal":pair.timing_equal,"aligned_rows":pair.lines.as_ref().map(Vec::len),"changed_rows":pair.lines.as_ref().map(|lines|lines.iter().filter(|r|r.changed).count()),"decoded_frames_compared":pair.pixel_stats.len(),"changed_pixels":pair.pixel_stats.iter().map(|s|s.changed_pixels).sum::<usize>(),"rgb_changed_pixels":pair.pixel_stats.iter().map(|s|s.rgb_changed_pixels).sum::<usize>(),"alpha_changed_pixels":pair.pixel_stats.iter().map(|s|s.alpha_changed_pixels).sum::<usize>()})),"limits":{"panes":MAX_PANES,"resident_payload_bytes":data::MAX_RESIDENT_BYTES,"per_pane_payload_bytes":data::MAX_PANE_BYTES,"byte_scan_per_file":data::MAX_BYTE_SCAN,"shared_difference_pixels":hover_core::comparison::MAX_DIFF_PIXELS},"scope":"Decoded previews and displayed metadata; no dependency, 3D, colorimetric or atomic-source equality claim"})
    }
}

pub(super) unsafe fn paint(hwnd: HWND, hdc: windows_sys::Win32::Graphics::Gdi::HDC, _rect: &RECT) {
    let raw = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State;
    if raw.is_null() {
        return;
    }
    let state = &*raw;
    let model = state.model.borrow();
    let mut client = RECT::default();
    GetClientRect(hwnd, &mut client);
    fill_rect(hdc, 0, 0, client.right, client.bottom, BG);
    let old = SelectObject(hdc, state.font.borrow().0 as _);
    SetBkMode(hdc, TRANSPARENT as i32);
    text(
        hdc,
        RECT {
            left: state.unit(18),
            top: state.unit(15),
            right: client.right - state.unit(16),
            bottom: state.unit(44),
        },
        &format!(
            "Compare  ·  {} files{}",
            model.panes.len(),
            if model.selected_total > model.panes.len() {
                format!(
                    "  ·  first {} of {} selected",
                    model.panes.len(),
                    model.selected_total
                )
            } else {
                String::new()
            }
        ),
        INK,
    );
    let selected = model.active.max(1);
    let pair = model.pairs.get(selected).and_then(Option::as_ref);
    for (index, pane) in model.panes.iter().enumerate() {
        fill_rect(
            hdc,
            pane.rect.left,
            pane.rect.top,
            pane.rect.right,
            pane.rect.bottom,
            SURFACE,
        );
        let edge = CreateSolidBrush(if index == model.active {
            ACCENT
        } else {
            rgb(54, 60, 68)
        });
        FrameRect(hdc, &pane.rect, edge);
        DeleteObject(edge);
        let metadata = if let Some(data) = &pane.data {
            match &data.content {
                Content::Images { frames } => frames
                    .first()
                    .map(|f| {
                        format!(
                            "Decoded {} × {}  ·  {}  ·  frame {}/{}",
                            f.width,
                            f.height,
                            format_bytes(data.bytes),
                            pane.view.frame + 1,
                            frames.len()
                        )
                    })
                    .unwrap_or_else(|| "No decoded frames".into()),
                Content::Rows { subtitle, .. } => format!(
                    "{}  ·  {subtitle}",
                    if data.is_dir {
                        "Folder metadata".into()
                    } else {
                        format_bytes(data.bytes)
                    }
                ),
            }
        } else if pane.error.is_some() {
            "Unavailable".into()
        } else {
            "Loading this file…".into()
        };
        if pane.rect.bottom - pane.rect.top >= state.unit(140) {
            text(
                hdc,
                RECT {
                    left: pane.rect.left + state.unit(12),
                    top: pane.rect.top + state.unit(44),
                    right: pane.rect.right - state.unit(10),
                    bottom: pane.rect.top + state.unit(68),
                },
                &metadata,
                MUTED,
            );
        }
        let saved = SaveDC(hdc);
        IntersectClipRect(
            hdc,
            pane.body.left,
            pane.body.top,
            pane.body.right,
            pane.body.bottom,
        );
        if let Some(error) = &pane.error {
            text_wrapped(
                hdc,
                pane.body,
                &format!("Preview unavailable\n{error}\nReload to retry."),
                rgb(255, 169, 134),
            );
        } else if let Some(data) = &pane.data {
            match &data.content {
                Content::Images { frames } => {
                    let mask = if model.difference && index == selected {
                        pair.and_then(|p| {
                            let reference = model.panes.first()?;
                            if reference.view.frame == pane.view.frame {
                                p.masks.get(pane.view.frame)
                            } else {
                                None
                            }
                        })
                    } else {
                        None
                    };
                    let frame = if model.difference && index == selected {
                        mask
                    } else {
                        frames.get(pane.view.frame)
                    };
                    if let (Some(frame), Some(zoom)) = (frame, pane.view.zoom) {
                        let origin = SaveDC(hdc);
                        windows_sys::Win32::Graphics::Gdi::SetViewportOrgEx(
                            hdc,
                            pane.body.left,
                            pane.body.top,
                            null_mut(),
                        );
                        if data
                            .has_alpha
                            .get(pane.view.frame)
                            .copied()
                            .unwrap_or(false)
                            && mask.is_none()
                        {
                            if !paint_alpha_frame(hdc, frame, zoom, state.unit(12)) {
                                text_wrapped(
                                    hdc,
                                    RECT {
                                        left: 0,
                                        top: 0,
                                        right: zoom.viewport_size.width as i32,
                                        bottom: zoom.viewport_size.height as i32,
                                    },
                                    "The transparent frame could not be rendered; reload to retry.",
                                    MUTED,
                                );
                            }
                        } else {
                            paint_frame(hdc, frame, zoom, 0);
                        }
                        RestoreDC(hdc, origin);
                    } else {
                        text_wrapped(
                            hdc,
                            pane.body,
                            "No aligned decoded-pixel mask for this frame. Pause, select a common frame, or show originals.",
                            MUTED,
                        );
                    }
                }
                Content::Rows { lines, .. } => {
                    let alignment = if index == 0 || index == selected {
                        pair.and_then(|p| p.lines.as_ref())
                    } else {
                        None
                    };
                    let count = alignment.map(Vec::len).unwrap_or(lines.len());
                    let line_height = state.unit(21);
                    let visible =
                        ((pane.body.bottom - pane.body.top) / line_height).max(0) as usize;
                    for row in pane.view.scroll..count.min(pane.view.scroll + visible) {
                        let (source, changed) = if let Some(map) = alignment {
                            let entry = &map[row];
                            (
                                if index == 0 { entry.left } else { entry.right },
                                entry.changed,
                            )
                        } else {
                            (Some(row), false)
                        };
                        let y = pane.body.top + (row - pane.view.scroll) as i32 * line_height;
                        let r = RECT {
                            left: pane.body.left,
                            top: y,
                            right: pane.body.right,
                            bottom: y + line_height,
                        };
                        if changed {
                            fill_rect(
                                hdc,
                                r.left,
                                r.top,
                                r.right,
                                r.bottom,
                                if index == 0 {
                                    rgb(69, 37, 39)
                                } else {
                                    rgb(28, 65, 53)
                                },
                            );
                        }
                        if let Some(source) = source
                            && let Some(line) = lines.get(source)
                        {
                            text(hdc, r, &format!("{:>4}  {line}", source + 1), INK);
                        }
                    }
                }
            }
        } else {
            text_wrapped(
                hdc,
                pane.body,
                "Loading an independent read-only preview…",
                MUTED,
            );
        }
        RestoreDC(hdc, saved);
        if pane.rect.bottom - pane.rect.top >= state.unit(140) {
            text(
                hdc,
                RECT {
                    left: pane.rect.left + state.unit(10),
                    top: pane.rect.bottom - state.unit(23),
                    right: pane.rect.right - state.unit(8),
                    bottom: pane.rect.bottom - state.unit(3),
                },
                if pane.view_note.is_empty() {
                    if index == 0 {
                        "Reference · file 1"
                    } else {
                        "Compared with file 1"
                    }
                } else {
                    &pane.view_note
                },
                MUTED,
            );
        }
    }
    let y = client.bottom - state.unit(100);
    let summary = if let Some(pair) = pair {
        let bytes = pair
            .byte_scan
            .changed
            .map(|changed| {
                format!(
                    "{changed} differing byte positions of {} examined",
                    pair.byte_scan.examined
                )
            })
            .unwrap_or_else(|| pair.byte_scan.note.clone());
        let detail = if !pair.pixel_stats.is_empty() {
            format!(
                "{} decoded pixels changed across {} frame(s)",
                pair.pixel_stats
                    .iter()
                    .map(|s| s.changed_pixels)
                    .sum::<usize>(),
                pair.pixel_stats.len()
            )
        } else if let Some(lines) = &pair.lines {
            format!(
                "{} changed displayed-line rows",
                lines.iter().filter(|r| r.changed).count()
            )
        } else {
            pair.note.clone()
        };
        format!("Files 1 ↔ {}   ·   {bytes}\n{detail}", selected + 1)
    } else if model.loading {
        "Previews arrive first; bounded file and content differences follow.".into()
    } else {
        "A comparison is unavailable. Each pane keeps its own identity and error.".into()
    };
    text_wrapped(
        hdc,
        RECT {
            left: state.unit(18),
            top: y,
            right: client.right - state.unit(16),
            bottom: y + state.unit(52),
        },
        &summary,
        INK,
    );
    text(
        hdc,
        RECT {
            left: state.unit(18),
            top: client.bottom - state.unit(42),
            right: client.right - state.unit(16),
            bottom: client.bottom - state.unit(14),
        },
        "Read-only previews · F5 reload · Ctrl+1–4 select · wheel zoom · drag pan · Ctrl+L link · dependencies/3D not compared",
        MUTED,
    );
    if !old.is_null() {
        SelectObject(hdc, old);
    }
}
/// WIC outputs premultiplied BGRA. Composite it over an explicit checkerboard
/// instead of treating the premultiplied channels as opaque. The temporary
/// surface is one validated frame and is released before the next pane paints.
unsafe fn paint_alpha_frame(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    frame: &OverlayFrame,
    zoom: ZoomState,
    tile: i32,
) -> bool {
    use windows_sys::Win32::Graphics::Gdi::{
        AlphaBlend, BLENDFUNCTION, CreateCompatibleDC, CreateDIBSection, DeleteDC, SelectObject,
    };
    let tile = tile.max(1);
    let w = zoom.viewport_size.width as i32;
    let h = zoom.viewport_size.height as i32;
    for y in (0..h).step_by(tile as usize) {
        for x in (0..w).step_by(tile as usize) {
            fill_rect(
                hdc,
                x,
                y,
                (x + tile).min(w),
                (y + tile).min(h),
                if (x / tile + y / tile) % 2 == 0 {
                    rgb(32, 36, 42)
                } else {
                    rgb(45, 50, 57)
                },
            );
        }
    }
    if frame.width == 0
        || frame.height == 0
        || frame.stride != frame.width * 4
        || frame.pixels_bgra.len() != frame.stride as usize * frame.height as usize
    {
        return false;
    }
    let origin = zoom.image_top_left();
    let scale = zoom.scale;
    let sx = ((0.0 - origin.x) / scale)
        .floor()
        .max(0.0)
        .min(frame.width as f32) as i32;
    let sy = ((0.0 - origin.y) / scale)
        .floor()
        .max(0.0)
        .min(frame.height as f32) as i32;
    let end_x = ((w as f32 - origin.x) / scale)
        .ceil()
        .max(0.0)
        .min(frame.width as f32) as i32;
    let end_y = ((h as f32 - origin.y) / scale)
        .ceil()
        .max(0.0)
        .min(frame.height as f32) as i32;
    if end_x <= sx || end_y <= sy {
        return true;
    }
    let source = CreateCompatibleDC(hdc);
    if source.is_null() {
        return false;
    }
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: 40,
            biWidth: frame.width as i32,
            biHeight: -(frame.height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut pixels = null_mut();
    let bitmap = CreateDIBSection(source, &info, DIB_RGB_COLORS, &mut pixels, null_mut(), 0);
    if bitmap.is_null() || pixels.is_null() {
        if !bitmap.is_null() {
            DeleteObject(bitmap);
        }
        DeleteDC(source);
        return false;
    }
    std::ptr::copy_nonoverlapping(
        frame.pixels_bgra.as_ptr(),
        pixels.cast::<u8>(),
        frame.pixels_bgra.len(),
    );
    let old = SelectObject(source, bitmap);
    if old.is_null() || old as isize == -1 {
        DeleteObject(bitmap);
        DeleteDC(source);
        return false;
    }
    let ok = AlphaBlend(
        hdc,
        (origin.x + sx as f32 * scale).round() as i32,
        (origin.y + sy as f32 * scale).round() as i32,
        ((end_x - sx) as f32 * scale).round().max(1.0) as i32,
        ((end_y - sy) as f32 * scale).round().max(1.0) as i32,
        source,
        sx,
        sy,
        end_x - sx,
        end_y - sy,
        BLENDFUNCTION {
            BlendOp: 0,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: 1,
        },
    ) != 0;
    if !old.is_null() {
        SelectObject(source, old);
    }
    DeleteObject(bitmap);
    DeleteDC(source);
    ok
}

unsafe fn text(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    mut rect: RECT,
    value: &str,
    color: u32,
) {
    SetTextColor(hdc, color);
    let s = wide(value);
    DrawTextW(
        hdc,
        s.as_ptr(),
        (s.len() - 1) as i32,
        &mut rect,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
    );
}
unsafe fn text_wrapped(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    mut rect: RECT,
    value: &str,
    color: u32,
) {
    SetTextColor(hdc, color);
    let s = wide(value);
    DrawTextW(
        hdc,
        s.as_ptr(),
        (s.len() - 1) as i32,
        &mut rect,
        DT_LEFT | DT_TOP | DT_WORDBREAK | DT_NOPREFIX,
    );
}
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        let create = &*(lparam as *const CREATESTRUCTW);
        let raw = create.lpCreateParams as *const State;
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw as isize);
        (*raw).hwnd.set(hwnd);
    }
    let raw = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State;
    if raw.is_null() {
        return DefWindowProcW(hwnd, message, wparam, lparam);
    }
    Rc::increment_strong_count(raw);
    let state = Rc::from_raw(raw);
    match message {
        WM_NOTIFYFORMAT => NFR_UNICODE as isize,
        WM_NOTIFY => {
            if lparam == 0 {
                return 0;
            }
            // Other notifications carry only NMHDR: check it before forming
            // a reference to the larger tooltip-specific structure.
            let header = *(lparam as *const NMHDR);
            if header.hwndFrom == state.tooltip.get() && header.code == TTN_GETDISPINFOW {
                let info = &mut *(lparam as *mut NMTTDISPINFOW);
                let id = GetDlgCtrlID(info.hdr.idFrom as HWND);
                let index = id - PANE;
                if (0..MAX_PANES as i32).contains(&index) {
                    let model = state.model.borrow();
                    if let Some(pane) = model.panes.get(index as usize) {
                        let details = if let Some(data) = &pane.data {
                            format!("{}\n{}", data.stage.replace('_', " "), data.note)
                        } else {
                            pane.error
                                .clone()
                                .unwrap_or_else(|| "Loading this file".into())
                        };
                        let text = format!("{}\n{details}", pane.path.display());
                        let mut text: Vec<u16> = text.encode_utf16().take(8191).collect();
                        if text.last().is_some_and(|ch| (0xd800..=0xdbff).contains(ch)) {
                            text.pop();
                        }
                        text.push(0);
                        let mut buffers = state.tooltip_text.borrow_mut();
                        buffers[index as usize] = text;
                        info.lpszText = buffers[index as usize].as_mut_ptr();
                        info.hinst = null_mut();
                    }
                }
            }
            0
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let dc = BeginPaint(hwnd, &mut ps);
            paint(hwnd, dc, &ps.rcPaint);
            EndPaint(hwnd, &ps);
            0
        }
        WM_ERASEBKGND => 1,
        READY => {
            state.poll();
            0
        }
        WM_SIZE => {
            state.layout();
            state.sync_timer();
            0
        }
        WM_TIMER if wparam == TIMER => {
            state.tick();
            0
        }
        WM_DPICHANGED => {
            state.dpi.set((wparam as u32 & 0xffff).max(96));
            state.set_font();
            let r = &*(lparam as *const RECT);
            SetWindowPos(
                hwnd,
                null_mut(),
                r.left,
                r.top,
                r.right - r.left,
                r.bottom - r.top,
                SWP_NOACTIVATE | SWP_NOZORDER,
            );
            0
        }
        WM_GETMINMAXINFO => {
            let info = &mut *(lparam as *mut MINMAXINFO);
            let work = screen_work_area(current_cursor_point());
            info.ptMinTrackSize = POINT {
                x: state.unit(500).min(work.width() as i32).max(1),
                y: state.unit(400).min(work.height() as i32).max(1),
            };
            0
        }
        WM_COMMAND => {
            let id = (wparam & 0xffff) as i32;
            match id {
                LINK => state.toggle_link(),
                DIFFERENCE => {
                    let value = !state.model.borrow().difference;
                    state.model.borrow_mut().difference = value;
                    state.refresh();
                }
                RELOAD => state.reload(),
                CLOSE => state.close(),
                PLAY => {
                    let value = !state.model.borrow().playing;
                    state.model.borrow_mut().playing = value;
                    state.sync_timer();
                    state.refresh();
                }
                PREVIOUS | NEXT => {
                    let model = state.model.borrow();
                    let active = model.active;
                    let frame = model.panes.get(active).map(|p| p.view.frame).unwrap_or(0);
                    let count = model.panes.get(active).map(|p| p.delays.len()).unwrap_or(0);
                    drop(model);
                    if count > 0 {
                        let frame = if id == NEXT {
                            (frame + 1) % count
                        } else {
                            (frame + count - 1) % count
                        };
                        let _ = state.set_frame(frame);
                    }
                }
                SINGLE => {
                    let path = {
                        let model = state.model.borrow();
                        model.panes.get(model.active).map(|p| p.path.clone())
                    };
                    state.model.borrow_mut().pending_single = path;
                    PostMessageW(state.parent, OPEN_SINGLE, 0, 0);
                }
                id if (PANE..PANE + MAX_PANES as i32).contains(&id) => {
                    state.select((id - PANE) as usize)
                }
                _ => {}
            }
            0
        }
        WM_DRAWITEM => {
            let draw = &*(lparam as *const DRAWITEMSTRUCT);
            let dc = draw.hDC;
            let model = state.model.borrow();
            let selected = (draw.CtlID as i32) == PANE + model.active as i32;
            fill_rect(
                dc,
                draw.rcItem.left,
                draw.rcItem.top,
                draw.rcItem.right,
                draw.rcItem.bottom,
                if selected {
                    rgb(45, 69, 81)
                } else {
                    rgb(40, 47, 55)
                },
            );
            let old = SelectObject(dc, state.font.borrow().0 as _);
            SetBkMode(dc, TRANSPARENT as i32);
            let label = agent_control::window_text(draw.hwndItem);
            text(
                dc,
                RECT {
                    left: draw.rcItem.left + state.unit(10),
                    ..draw.rcItem
                },
                &label,
                if selected { ACCENT } else { INK },
            );
            if draw.itemState & ODS_FOCUS != 0 {
                let mut r = draw.rcItem;
                r.left += 3;
                r.top += 3;
                r.right -= 3;
                r.bottom -= 3;
                DrawFocusRect(dc, &r);
            }
            if !old.is_null() {
                SelectObject(dc, old);
            }
            1
        }
        WM_MOUSEWHEEL => {
            state.wheel(wparam, lparam);
            0
        }
        WM_LBUTTONDOWN => {
            let x = signed_loword_isize(lparam);
            let y = signed_hiword_isize(lparam);
            let index = state.model.borrow().panes.iter().position(|p| {
                x >= p.body.left && x < p.body.right && y >= p.body.top && y < p.body.bottom
            });
            if let Some(index) = index {
                state.select(index);
                SetFocus(hwnd);
                state.drag.set(Some((x, y)));
                windows_sys::Win32::UI::Input::KeyboardAndMouse::SetCapture(hwnd);
            }
            0
        }
        WM_MOUSEMOVE => {
            if let Some((old_x, old_y)) = state.drag.get() {
                let x = signed_loword_isize(lparam);
                let y = signed_hiword_isize(lparam);
                let _ = state.pan((x - old_x) as f32, (y - old_y) as f32);
                state.drag.set(Some((x, y)));
            }
            0
        }
        WM_LBUTTONUP => {
            state.drag.set(None);
            windows_sys::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
            0
        }
        WM_CAPTURECHANGED => {
            state.drag.set(None);
            0
        }
        WM_CLOSE => {
            state.close();
            0
        }
        WM_NCDESTROY => {
            state.notify.store(0, Ordering::Release);
            state.worker.cancel();
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            state.hwnd.set(null_mut());
            DefWindowProcW(hwnd, message, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}
