use std::mem::ManuallyDrop;
use std::path::{Path, PathBuf};
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use windows::Win32::System::Variant::{
    VARIANT, VARIANT_0, VARIANT_0_0, VARIANT_0_0_0, VT_I4, VariantClear,
};
use windows::Win32::UI::Accessibility::{AccessibleObjectFromPoint, IAccessible};
use windows::Win32::UI::Shell::{IShellWindows, ShellWindows};
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetForegroundWindow};
use windows::core::Interface;

pub fn resolve_hover_path() -> Option<PathBuf> {
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
        .ok()
        .ok()?;
    struct ComGuard;
    impl Drop for ComGuard {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }
    let _com = ComGuard;
    let item = item_name_or_path_under_cursor()?;
    if let Some(path) = existing_absolute_path(&item) {
        return Some(path);
    }
    let folder = foreground_explorer_folder()?;
    let direct = folder.join(item.trim());
    if direct.exists() {
        return Some(direct);
    }
    None
}

fn item_name_or_path_under_cursor() -> Option<String> {
    unsafe {
        let mut cursor = POINT::default();
        GetCursorPos(&mut cursor).ok()?;

        let mut accessible: Option<IAccessible> = None;
        let mut child = VARIANT::default();
        let result = (|| {
            AccessibleObjectFromPoint(cursor, &mut accessible, &mut child).ok()?;
            let acc = accessible.as_ref()?;
            if let Ok(value) = acc.get_accValue(&child) {
                let text = value.to_string();
                if usable_item_text(&text) {
                    return Some(text);
                }
            }
            if let Ok(name) = acc.get_accName(&child) {
                let text = name.to_string();
                if usable_item_text(&text) {
                    return Some(text);
                }
            }
            let default_child = VARIANT::default();
            if let Ok(name) = acc.get_accName(&default_child) {
                let text = name.to_string();
                if usable_item_text(&text) {
                    return Some(text);
                }
            }
            None
        })();
        let _ = VariantClear(&mut child);
        result
    }
}

fn foreground_explorer_folder() -> Option<PathBuf> {
    unsafe {
        let foreground = GetForegroundWindow();
        if foreground.is_invalid() {
            return None;
        }
        let shell_windows: IShellWindows =
            CoCreateInstance(&ShellWindows, None, CLSCTX_ALL).ok()?;
        let count = shell_windows.Count().ok()?;
        for index in 0..count {
            let variant = variant_i4(index);
            let disp = shell_windows.Item(&variant).ok()?;
            let browser = disp
                .cast::<windows::Win32::UI::Shell::IWebBrowser2>()
                .ok()?;
            let browser_hwnd = browser.HWND().ok()?;
            if HWND(browser_hwnd.0 as *mut _) != foreground {
                continue;
            }
            let url = browser.LocationURL().ok()?.to_string();
            if let Some(path) = file_url_to_path(&url) {
                return Some(path);
            }
        }
        None
    }
}

unsafe fn variant_i4(value: i32) -> VARIANT {
    VARIANT {
        Anonymous: VARIANT_0 {
            Anonymous: ManuallyDrop::new(VARIANT_0_0 {
                vt: VT_I4,
                wReserved1: 0,
                wReserved2: 0,
                wReserved3: 0,
                Anonymous: VARIANT_0_0_0 { lVal: value },
            }),
        },
    }
}

fn existing_absolute_path(value: &str) -> Option<PathBuf> {
    let path = PathBuf::from(value.trim());
    if path.is_absolute() && path.exists() {
        Some(path)
    } else {
        None
    }
}

fn usable_item_text(value: &str) -> bool {
    let text = value.trim();
    if text.is_empty() {
        return false;
    }
    const CONTAINERS: &[&str] = &[
        "Items View",
        "Folder View",
        "Shell Folder View",
        "UIItemsView",
        "DirectUIHWND",
        "Name",
        "Date modified",
        "Type",
        "Size",
    ];
    !CONTAINERS
        .iter()
        .any(|container| text.eq_ignore_ascii_case(container))
}

fn file_url_to_path(url: &str) -> Option<PathBuf> {
    let without_prefix = url.strip_prefix("file:///")?;
    let decoded = percent_decode(without_prefix)?.replace('/', "\\");
    if decoded.contains('\0') || !Path::new(&decoded).is_absolute() {
        return None;
    }
    Some(Path::new(&decoded).to_path_buf())
}

fn percent_decode(value: &str) -> Option<String> {
    let mut output = Vec::with_capacity(value.len());
    let mut chars = value.as_bytes().iter().copied();
    while let Some(byte) = chars.next() {
        if byte == b'%' {
            let hex = [chars.next()?, chars.next()?];
            output.push(u8::from_str_radix(std::str::from_utf8(&hex).ok()?, 16).ok()?);
        } else {
            output.push(byte);
        }
    }
    String::from_utf8(output).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_file_url() {
        assert_eq!(
            file_url_to_path("file:///C:/Fixtures/My%20Images").unwrap(),
            PathBuf::from(r"C:\Fixtures\My Images")
        );
    }

    #[test]
    fn preserves_unicode_file_identity_and_rejects_malformed_urls() {
        assert_eq!(
            file_url_to_path("file:///C:/Fixtures/%E9%9B%AA%20caf%C3%A9.png"),
            Some(PathBuf::from(r"C:\Fixtures\雪 café.png"))
        );
        assert_eq!(
            file_url_to_path("file:///C:/Fixtures/雪.png"),
            Some(PathBuf::from(r"C:\Fixtures\雪.png"))
        );
        for url in [
            "file:///C:/bad%",
            "file:///C:/bad%XX",
            "file:///C:/bad%FF",
            "file:///C:/bad%00",
            "file://server/share/file",
        ] {
            assert!(file_url_to_path(url).is_none());
        }
    }
}
