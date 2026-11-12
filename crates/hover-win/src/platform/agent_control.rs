//! The agent calls production App/model operations on the native UI thread.
use super::*;
use crate::agent::validate_fields;
use serde_json::{Value, json};

fn required_string<'a>(command: &'a Value, key: &str) -> Result<&'a str, String> {
    command
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{key} must be a string"))
}

fn boolean(command: &Value, key: &str, default: bool) -> Result<bool, String> {
    match command.get(key) {
        None => Ok(default),
        Some(value) => value
            .as_bool()
            .ok_or_else(|| format!("{key} must be a boolean")),
    }
}

fn integer(command: &Value, key: &str, min: u64, max: u64) -> Result<u64, String> {
    command
        .get(key)
        .and_then(Value::as_u64)
        .filter(|n| (min..=max).contains(n))
        .ok_or_else(|| format!("{key} must be an integer between {min} and {max}"))
}

fn number(command: &Value, key: &str) -> Result<f32, String> {
    command
        .get(key)
        .and_then(Value::as_f64)
        .filter(|n| n.is_finite() && n.abs() <= 1_000_000.0)
        .map(|n| n as f32)
        .ok_or_else(|| format!("{key} must be finite and within +/-1000000"))
}

unsafe fn window_text(hwnd: HWND) -> String {
    let mut text = [0u16; 1024];
    let len = windows_sys::Win32::UI::WindowsAndMessaging::GetWindowTextW(
        hwnd,
        text.as_mut_ptr(),
        text.len() as i32,
    );
    String::from_utf16_lossy(&text[..len.max(0) as usize])
}

impl App {
    pub(super) unsafe fn execute_agent_command(&mut self, command: Value) -> Result<Value, String> {
        let name = required_string(&command, "command")?;
        match name {
            "capabilities" => {
                validate_fields(&command, &["command"])?;
                return Ok(
                    json!({"protocol":1,"transport":"private_stdio","owns_session":true,
                    "configuration":"ephemeral_defaults_no_startup_mutation",
                    "commands":["capabilities","support","settings","status","inspect","open","pin","reload","close","wait","zoom","pan","scroll","resize","minimize","restore","focus","play","activation","capture","quit"],
                    "limitations":["No external-window input injection","Controlled activation by default; activation can enable production shell targeting","No claim of physical-input, accessibility or packaged-release certification"]}),
                );
            }
            "status" => {
                validate_fields(&command, &["command"])?;
            }
            "support" => {
                validate_fields(&command, &["command"])?;
                return serde_json::from_str(include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../docs/support-status.json"
                )))
                .map_err(|e| format!("Bundled support status is invalid: {e}"));
            }
            "settings" => {
                validate_fields(&command, &["command"])?;
                return Ok(
                    json!({"effective_ini":settings_to_ini(&self.settings),"persisted":false,"startup_mutation":false}),
                );
            }
            "inspect" => {
                validate_fields(&command, &["command", "offset", "limit"])?;
                let offset = if command.get("offset").is_some() {
                    integer(&command, "offset", 0, 20_000)? as usize
                } else {
                    0
                };
                let limit = if command.get("limit").is_some() {
                    integer(&command, "limit", 1, 128)? as usize
                } else {
                    128
                };
                let mut status = self.agent_status();
                status["content"] = inspect_content(offset, limit);
                return Ok(status);
            }
            "open" => {
                validate_fields(&command, &["command", "path", "pin", "activate"])?;
                let path = PathBuf::from(required_string(&command, "path")?);
                if !path.is_absolute() || path.as_os_str().is_empty() {
                    return Err("path must be an absolute local path".into());
                }
                let pin = boolean(&command, "pin", true)?;
                let activate = boolean(&command, "activate", false)?;
                self.cancel_decode();
                self.hide_preview();
                self.candidate_path = Some(path.clone());
                self.dismissed_path = None;
                self.start_decode(path);
                if pin && !self.pinned {
                    self.toggle_pin_with_activation(activate);
                }
            }
            "pin" => {
                validate_fields(&command, &["command", "value", "activate"])?;
                let value = boolean(&command, "value", true)?;
                let activate = boolean(&command, "activate", false)?;
                if self.current_path.is_none() {
                    return Err("No preview to pin".into());
                }
                if self.pinned != value {
                    self.toggle_pin_with_activation(activate);
                }
            }
            "reload" => {
                validate_fields(&command, &["command"])?;
                if self.current_path.is_none() {
                    return Err("No preview to reload".into());
                }
                if self.pending_path.is_some() {
                    return Err("A preview is already loading".into());
                }
                self.reload_current();
            }
            "close" => {
                validate_fields(&command, &["command"])?;
                self.dismiss_current();
            }
            "quit" => {
                validate_fields(&command, &["command"])?;
                PostQuitMessage(0);
            }
            "play" => {
                validate_fields(&command, &["command"])?;
                if self.media_ready.is_none() {
                    return Err("No media is awaiting explicit playback".into());
                }
                self.play_ready_media();
            }
            "activation" => {
                validate_fields(&command, &["command", "automatic"])?;
                let automatic = boolean(&command, "automatic", false)?;
                self.agent_controlled = !automatic;
                if automatic {
                    self.on_foreground_changed();
                } else {
                    KillTimer(self.hwnd, TIMER_CURSOR);
                }
            }
            "zoom" | "pan" => {
                if name == "zoom" {
                    validate_fields(&command, &["command", "mode"])?;
                } else {
                    validate_fields(&command, &["command", "dx", "dy"])?;
                }
                let mode = if name == "zoom" {
                    Some(match required_string(&command, "mode")? {
                        "fit" => hover_core::ScaleMode::FitAll,
                        "fit_width" => hover_core::ScaleMode::FitWidth,
                        "fit_height" => hover_core::ScaleMode::FitHeight,
                        "native" => hover_core::ScaleMode::Native,
                        _ => return Err("mode must be fit, fit_width, fit_height or native".into()),
                    })
                } else {
                    None
                };
                let delta = if name == "pan" {
                    Some((number(&command, "dx")?, number(&command, "dy")?))
                } else {
                    None
                };
                {
                    let mut guard = OVERLAY_MODEL
                        .get_or_init(|| Mutex::new(None))
                        .lock()
                        .unwrap();
                    let model = guard.as_mut().ok_or("No preview is open")?;
                    let OverlayContent::Frames { zoom, .. } = &mut model.content else {
                        return Err("This preview is not an image".into());
                    };
                    if let Some(mode) = mode {
                        zoom.set_mode(mode, ZoomConfig::default());
                    }
                    if let Some((dx, dy)) = delta {
                        zoom.pan_by(dx, dy, ZoomConfig::default());
                    }
                    zoom.settle_quality();
                }
                InvalidateRect(self.overlay, null(), 0);
            }
            "scroll" => {
                validate_fields(&command, &["command", "offset"])?;
                let offset = integer(&command, "offset", 0, 20_000)? as usize;
                {
                    let mut guard = OVERLAY_MODEL
                        .get_or_init(|| Mutex::new(None))
                        .lock()
                        .unwrap();
                    let model = guard.as_mut().ok_or("No preview is open")?;
                    if matches!(model.content, OverlayContent::Frames { .. }) {
                        return Err("Images pan instead of scrolling rows".into());
                    }
                    model.scroll_offset = offset;
                }
                resize_overlay_view(self.overlay);
            }
            "resize" => {
                validate_fields(&command, &["command", "width", "height"])?;
                let width = integer(&command, "width", 96, 4096)? as f32;
                let height = integer(&command, "height", 96, 2160)? as f32;
                if self.current_path.is_none() || IsIconic(self.overlay) != 0 {
                    return Err("Open and restore the preview before resizing".into());
                }
                let rect = overlay_screen_rect(self.overlay);
                self.place_overlay(HoverRect::new(
                    rect.left,
                    rect.top,
                    rect.left + width,
                    rect.top + height,
                ));
            }
            "minimize" | "restore" => {
                validate_fields(&command, &["command"])?;
                if !self.pinned {
                    return Err("Pin the preview first".into());
                }
                ShowWindow(
                    self.overlay,
                    if name == "minimize" {
                        windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWMINNOACTIVE
                    } else {
                        SW_SHOWNOACTIVATE
                    },
                );
                self.sync_pinned_media();
            }
            "focus" => {
                validate_fields(&command, &["command", "target"])?;
                if !self.pinned {
                    return Err("Pin the preview first".into());
                }
                match required_string(&command, "target")? {
                    "next" => cycle_preview_focus(self.overlay, false),
                    "previous" => cycle_preview_focus(self.overlay, true),
                    "content" => {
                        SetFocus(self.overlay);
                    }
                    _ => return Err("target must be content, next or previous".into()),
                }
            }
            "capture" => {
                validate_fields(&command, &["command", "path"])?;
                let path = PathBuf::from(required_string(&command, "path")?);
                return capture_preview(self.overlay, &path);
            }
            _ => return Err(format!("Unknown command: {name}")),
        }
        Ok(self.agent_status())
    }

    unsafe fn agent_status(&self) -> Value {
        let rect = overlay_screen_rect(self.overlay);
        let client = content_client_rect(self.overlay);
        let guard = OVERLAY_MODEL
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap();
        let model = guard.as_ref();
        let content = match model.map(|model| &model.content) {
            Some(OverlayContent::Frames { frames, zoom }) => {
                json!({"kind":"image","frames":frames.len(),"frame":model.unwrap().current_frame,
                "image_width":zoom.image_size.width,"image_height":zoom.image_size.height,"scale":zoom.scale,"pan_x":zoom.pan_x,"pan_y":zoom.pan_y,
                "viewport_width":zoom.viewport_size.width,"viewport_height":zoom.viewport_size.height})
            }
            Some(OverlayContent::Text {
                lines, truncated, ..
            }) => json!({"kind":"text","rows":lines.len(),"truncated":truncated}),
            Some(OverlayContent::List {
                entries, omitted, ..
            }) => json!({"kind":"list","rows":entries.len(),"omitted":omitted}),
            None => Value::Null,
        };
        let controls = [ID_PREVIEW_PIN,ID_PREVIEW_RELOAD,ID_PREVIEW_CLOSE].map(|id| {
            let hwnd = GetDlgItem(self.overlay,id as i32);
            let mut rect = RECT::default();
            GetWindowRect(hwnd, &mut rect);
            let mut origin = POINT { x:rect.left, y:rect.top };
            ScreenToClient(self.overlay, &mut origin);
            json!({"id":id,"label":window_text(hwnd),"visible":IsWindowVisible(hwnd)!=0,"enabled":IsWindowEnabled(hwnd)!=0,"focused":GetFocus()==hwnd,"rect":{"x":origin.x,"y":origin.y,"width":rect.right-rect.left,"height":rect.bottom-rect.top}})
        });
        json!({"pid":std::process::id(),"version":env!("CARGO_PKG_VERSION"),"profile":if cfg!(debug_assertions){"debug"}else{"release"},
            "state":format!("{:?}",self.state),"phase":if self.pending_path.is_some(){"loading"}else if self.preview_error.is_some(){"error"}else if self.current_path.is_some(){"ready"}else{"empty"},
            "path":self.current_path,"pending_path":self.pending_path,"loading":self.pending_path.is_some(),"error":self.preview_error,
            "pinned":self.pinned,"automatic_activation":!self.agent_controlled,"visible":IsWindowVisible(self.overlay)!=0,"minimized":IsIconic(self.overlay)!=0,
            "caption":window_text(self.overlay),"no_activate":GetWindowLongPtrW(self.overlay,GWL_EXSTYLE) as u32 & WS_EX_NOACTIVATE !=0,
            "rect":{"left":rect.left,"top":rect.top,"width":rect.width(),"height":rect.height()},
            "content_rect":{"width":client.right-client.left,"height":client.bottom-client.top},
            "scroll":model.map(|model|model.scroll_offset),"content":content,"controls":controls,
            "content_focused":GetFocus()==self.overlay,"media_waiting":self.media_ready.is_some(),"video_running":self.video.is_some(),"audio_running":self.audio.is_some(),
            "worker_busy":self.decode_worker.as_ref().is_some_and(PreviewWorker::is_busy)})
    }
}

fn inspect_content(offset: usize, limit: usize) -> Value {
    let guard = OVERLAY_MODEL
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap();
    match guard.as_ref().map(|model| &model.content) {
        Some(OverlayContent::Text {
            title,
            subtitle,
            lines,
            truncated,
        }) => {
            json!({"kind":"text","title":title,"subtitle":subtitle,"total_rows":lines.len(),"offset":offset,"lines":lines.iter().skip(offset).take(limit).collect::<Vec<_>>(),"source_truncated":truncated})
        }
        Some(OverlayContent::List {
            title,
            subtitle,
            entries,
            omitted,
        }) => {
            json!({"kind":"list","title":title,"subtitle":subtitle,"total_rows":entries.len(),"offset":offset,"entries":entries.iter().skip(offset).take(limit).map(|entry|json!({"name":entry.name.chars().take(4096).collect::<String>(),"name_truncated":entry.name.chars().count()>4096,"detail":entry.detail.chars().take(1024).collect::<String>()})).collect::<Vec<_>>(),"omitted":omitted})
        }
        Some(OverlayContent::Frames { frames, .. }) => {
            let model = guard.as_ref().unwrap();
            frames.get(model.current_frame).map(|frame|json!({"kind":"image","width":frame.width,"height":frame.height,"stride":frame.stride,"frame":model.current_frame,"frames":frames.len(),"pixel_checksum_fnv1a64":format!("{:016x}",frame.pixels_bgra.iter().fold(0xcbf29ce484222325u64,|hash,byte|(hash^*byte as u64).wrapping_mul(0x100000001b3)))})).unwrap_or(Value::Null)
        }
        None => Value::Null,
    }
}

unsafe fn capture_preview(hwnd: HWND, path: &Path) -> Result<Value, String> {
    use windows_sys::Win32::Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, HBITMAP, HDC, HGDIOBJ, SelectObject,
        SetViewportOrgEx,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        PRF_CHILDREN, PRF_CLIENT, PRF_ERASEBKGND, PRF_NONCLIENT, WM_PRINTCLIENT,
    };
    if !path.is_absolute()
        || path
            .extension()
            .is_none_or(|ext| !ext.eq_ignore_ascii_case("bmp"))
    {
        return Err("Capture requires an absolute .bmp output path".into());
    }
    let file_name = path
        .file_name()
        .ok_or("Missing output filename")?
        .to_string_lossy();
    let stem = file_name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end_matches(' ')
        .to_ascii_uppercase();
    // Windows also reserves COM/LPT names with superscript digits:
    // https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file
    let numbered_device = stem
        .strip_prefix("COM")
        .or_else(|| stem.strip_prefix("LPT"))
        .is_some_and(|suffix| {
            suffix.chars().count() == 1
                && suffix
                    .chars()
                    .next()
                    .is_some_and(|c| matches!(c, '0'..='9' | '¹' | '²' | '³'))
        });
    if file_name
        .chars()
        .any(|c| c < ' ' || r#"<>:"/\|?*"#.contains(c))
        || file_name.ends_with(['.', ' '])
        || matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        )
        || numbered_device
    {
        return Err("Device/stream output names are not allowed".into());
    }
    let parent = path.parent().ok_or("Missing output directory")?;
    if !local_metadata(parent).map_err(|e| e.to_string())?.is_dir() {
        return Err("Output directory is not a local directory".into());
    }
    if IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 {
        return Err("Open and restore the preview before capturing".into());
    }
    let mut rect = RECT::default();
    GetClientRect(hwnd, &mut rect);
    let (width, height) = (rect.right, rect.bottom);
    if width <= 0 || height <= 0 || width > 4096 || height > 2160 {
        return Err("Capture exceeds the 4096 x 2160 client-area budget".into());
    }
    struct Surface {
        dc: HDC,
        bitmap: HBITMAP,
        old: HGDIOBJ,
    }
    impl Drop for Surface {
        fn drop(&mut self) {
            unsafe {
                if !self.old.is_null() {
                    SelectObject(self.dc, self.old);
                }
                if !self.bitmap.is_null() {
                    DeleteObject(self.bitmap);
                }
                if !self.dc.is_null() {
                    DeleteDC(self.dc);
                }
            }
        }
    }
    let mut surface = Surface {
        dc: CreateCompatibleDC(null_mut()),
        bitmap: null_mut(),
        old: null_mut(),
    };
    if surface.dc.is_null() {
        return Err("Could not allocate capture DC".into());
    }
    let bytes = (width as usize) * (height as usize) * 4;
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: 40,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: bytes as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut pixels = null_mut();
    surface.bitmap = CreateDIBSection(
        surface.dc,
        &info,
        DIB_RGB_COLORS,
        &mut pixels,
        null_mut(),
        0,
    );
    if surface.bitmap.is_null() || pixels.is_null() {
        return Err("Could not allocate capture bitmap".into());
    }
    std::ptr::write_bytes(pixels as *mut u8, 0, bytes);
    surface.old = SelectObject(surface.dc, surface.bitmap);
    if surface.old.is_null() || surface.old as isize == -1 {
        surface.old = null_mut();
        return Err("Could not select capture bitmap".into());
    }
    // The actual preview painter, viewport and native controls render into this
    // bounded surface. This does not capture the desktop or another app.
    paint_overlay(hwnd, surface.dc, &rect);
    for id in [
        ID_PREVIEW_TITLE,
        ID_PREVIEW_PIN as i32,
        ID_PREVIEW_RELOAD as i32,
        ID_PREVIEW_CLOSE as i32,
    ] {
        let child = GetDlgItem(hwnd, id);
        if IsWindowVisible(child) == 0 {
            continue;
        }
        let mut child_rect = RECT::default();
        GetWindowRect(child, &mut child_rect);
        let mut point = POINT {
            x: child_rect.left,
            y: child_rect.top,
        };
        ScreenToClient(hwnd, &mut point);
        let saved = SaveDC(surface.dc);
        if saved == 0 {
            return Err("Could not preserve native control capture state".into());
        }
        let mut old = POINT::default();
        SetViewportOrgEx(surface.dc, point.x, point.y, &mut old);
        SendMessageW(
            child,
            WM_PRINTCLIENT,
            surface.dc as usize,
            (PRF_CLIENT | PRF_CHILDREN | PRF_NONCLIENT | PRF_ERASEBKGND) as isize,
        );
        RestoreDC(surface.dc, saved);
    }
    windows_sys::Win32::Graphics::Gdi::GdiFlush();
    let mut image = Vec::with_capacity(54 + bytes);
    image.extend_from_slice(b"BM");
    image.extend_from_slice(&((54 + bytes) as u32).to_le_bytes());
    image.extend_from_slice(&[0; 4]);
    image.extend_from_slice(&54u32.to_le_bytes());
    image.extend_from_slice(&40u32.to_le_bytes());
    image.extend_from_slice(&width.to_le_bytes());
    image.extend_from_slice(&(-height).to_le_bytes());
    image.extend_from_slice(&1u16.to_le_bytes());
    image.extend_from_slice(&32u16.to_le_bytes());
    image.extend_from_slice(&0u32.to_le_bytes());
    image.extend_from_slice(&(bytes as u32).to_le_bytes());
    image.extend_from_slice(&[0; 16]);
    image.extend_from_slice(std::slice::from_raw_parts(pixels as *const u8, bytes));
    drop(surface);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("Output was not overwritten: {e}"))?;
    file.write_all(&image)
        .map_err(|e| format!("Capture write failed; output may be incomplete: {e}"))?;
    Ok(
        json!({"path":path,"width":width,"height":height,"bytes":image.len(),"representation":"native_preview_client_render","desktop_screenshot":false}),
    )
}
