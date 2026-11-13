//! First-party lifecycle control, scoped to this user and installation folder.
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::PathBuf;
use std::ptr::null_mut;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Security::Authorization::*;
use windows_sys::Win32::Security::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub(crate) const MESSAGE: u32 = WM_APP + 21;
pub(crate) struct Scope {
    pub title: String,
    name: Vec<u16>,
    user: String,
    executable: PathBuf,
}
fn wide(s: &std::ffi::OsStr) -> Vec<u16> {
    s.encode_wide().chain(Some(0)).collect()
}
fn error(action: &str) -> String {
    format!("{action}: {}", std::io::Error::last_os_error())
}
fn user_sid(process: HANDLE) -> Result<String, String> {
    let mut token = null_mut();
    if unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) } == 0 {
        return Err(error("Cannot verify Hover owner"));
    }
    let token = unsafe { OwnedHandle::from_raw_handle(token) };
    let mut size = 0;
    unsafe {
        GetTokenInformation(token.as_raw_handle(), TokenUser, null_mut(), 0, &mut size);
    }
    if size == 0 || size > 16384 {
        return Err(error("Cannot size Hover owner information"));
    }
    let mut buffer = vec![0usize; (size as usize).div_ceil(size_of::<usize>())];
    if unsafe {
        GetTokenInformation(
            token.as_raw_handle(),
            TokenUser,
            buffer.as_mut_ptr().cast(),
            size,
            &mut size,
        )
    } == 0
    {
        return Err(error("Cannot query Hover owner"));
    }
    let user = unsafe { &*buffer.as_ptr().cast::<TOKEN_USER>() };
    let mut sid = null_mut();
    if unsafe { ConvertSidToStringSidW(user.User.Sid, &mut sid) } == 0 {
        return Err(error("Cannot encode Hover owner"));
    }
    let mut count = 0;
    while count < 256 && unsafe { *sid.add(count) } != 0 {
        count += 1;
    }
    let text = if count < 256 {
        String::from_utf16(unsafe { std::slice::from_raw_parts(sid, count) })
            .map_err(|_| "Invalid owner encoding".into())
    } else {
        Err("Invalid owner length".into())
    };
    unsafe {
        LocalFree(sid.cast());
    }
    text
}
impl Scope {
    pub(crate) fn current() -> Result<Self, String> {
        let current = std::env::current_exe().map_err(|e| e.to_string())?;
        let directory = current
            .parent()
            .ok_or("Missing Hover installation directory")?
            .canonicalize()
            .map_err(|e| e.to_string())?;
        let executable = if current
            .file_name()
            .is_some_and(|s| s.eq_ignore_ascii_case("hover-agent.exe"))
        {
            directory.join("hover.exe")
        } else {
            current.canonicalize().map_err(|e| e.to_string())?
        };
        let user = user_sid(unsafe { GetCurrentProcess() })?;
        let mut hash = 0xcbf29ce484222325u64;
        // This hash is a namespace, not authentication. Ownership/image checks
        // and the protected mutex DACL provide the boundary.
        for byte in directory
            .to_string_lossy()
            .to_lowercase()
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
        {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
        }
        let title = format!("Hover.Resident.{user}.{hash:016x}");
        let name = wide(std::ffi::OsStr::new(&format!("Local\\{title}")));
        Ok(Self {
            title,
            name,
            user,
            executable,
        })
    }
    pub(crate) fn window(&self) -> Result<Option<(HWND, u32)>, String> {
        let title = wide(std::ffi::OsStr::new(&self.title));
        let class = wide(std::ffi::OsStr::new(crate::platform::CONTROL_CLASS));
        let hwnd = unsafe { FindWindowW(class.as_ptr(), title.as_ptr()) };
        if hwnd.is_null() {
            return Ok(None);
        }
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut pid);
        }
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if handle.is_null() {
            return Err(error("Cannot verify resident Hover process"));
        }
        let handle = unsafe { OwnedHandle::from_raw_handle(handle) };
        if user_sid(handle.as_raw_handle())? != self.user {
            return Err("Resident window belongs to a different user".into());
        }
        let mut path = vec![0u16; 32768];
        let mut count = path.len() as u32;
        if unsafe {
            QueryFullProcessImageNameW(handle.as_raw_handle(), 0, path.as_mut_ptr(), &mut count)
        } == 0
        {
            return Err(error("Cannot verify resident Hover image"));
        }
        let path = PathBuf::from(std::ffi::OsString::from_wide(&path[..count as usize]))
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if !path
            .to_string_lossy()
            .eq_ignore_ascii_case(&self.executable.to_string_lossy())
        {
            return Err("Resident window has an unexpected executable".into());
        }
        Ok(Some((hwnd, pid)))
    }
    pub(crate) fn signal(&self, show: bool) -> Result<bool, String> {
        let Some((hwnd, pid)) = self.window()? else {
            return Ok(false);
        };
        if show {
            unsafe {
                AllowSetForegroundWindow(pid);
            }
        }
        if unsafe { PostMessageW(hwnd, MESSAGE, usize::from(show), 0) } == 0 {
            return Err(error("Cannot signal resident Hover"));
        }
        Ok(true)
    }
    pub(crate) fn claim(&self) -> Result<Option<OwnedHandle>, String> {
        let descriptor = wide(std::ffi::OsStr::new(&format!(
            "D:P(A;;GA;;;SY)(A;;GA;;;{})",
            self.user
        )));
        let mut security = null_mut();
        if unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                descriptor.as_ptr(),
                SDDL_REVISION_1,
                &mut security,
                null_mut(),
            )
        } == 0
        {
            return Err(error("Cannot prepare resident ownership"));
        }
        let attributes = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: security,
            bInheritHandle: 0,
        };
        let mutex = unsafe { CreateMutexW(&attributes, 0, self.name.as_ptr()) };
        let result = unsafe { GetLastError() };
        unsafe {
            LocalFree(security);
        }
        if mutex.is_null() {
            return Err(format!(
                "Cannot claim resident Hover: {}",
                std::io::Error::from_raw_os_error(result as i32)
            ));
        }
        let mutex = unsafe { OwnedHandle::from_raw_handle(mutex) };
        if result == ERROR_ALREADY_EXISTS {
            drop(mutex);
            Ok(None)
        } else {
            Ok(Some(mutex))
        }
    }
}

pub fn command(action: &str) -> Result<serde_json::Value, String> {
    let scope = Scope::current()?;
    if action == "--resident-status" {
        return Ok(match scope.window()? {
            Some((_, pid)) => {
                serde_json::json!({"schemaVersion":1,"running":true,"pid":pid,"app":"Hover"})
            }
            None => serde_json::json!({"schemaVersion":1,"running":false,"app":"Hover"}),
        });
    }
    let sent = scope.signal(action == "--show")?;
    Ok(serde_json::json!({"schemaVersion":1,"app":"Hover","request":action,"sent":sent}))
}
