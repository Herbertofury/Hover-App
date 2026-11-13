//! LPAC launch with only the system COM/registry-read capabilities needed by WIC.
use std::fs::{File, OpenOptions};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::os::windows::process::ExitStatusExt;
use std::path::Path;
use std::process::ExitStatus;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Security::Authorization::*;
use windows_sys::Win32::Security::Isolation::{
    CreateAppContainerProfile, DeriveAppContainerSidFromAppContainerName,
};
use windows_sys::Win32::Security::*;
use windows_sys::Win32::Storage::FileSystem::*;
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::System::Pipes::CreatePipe;
use windows_sys::Win32::System::SystemInformation::{
    GetSystemDirectoryW, GetSystemWindowsDirectoryW,
};
use windows_sys::Win32::System::SystemServices::SE_GROUP_ENABLED;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::Shell::{FOLDERID_LocalAppData, SHGetKnownFolderPath};

const MONIKER: &str = "Herbertofury.Hover.ImageDecoder.v1";

fn ensure_profile() -> Result<(), String> {
    static PROFILE: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();
    PROFILE
        .get_or_init(|| {
            let name = wide(std::ffi::OsStr::new(MONIKER));
            let display = wide(std::ffi::OsStr::new("Hover image decoder"));
            let description = wide(std::ffi::OsStr::new(
                "Isolated native image decoding without network or library capabilities",
            ));
            let mut sid = null_mut();
            let result = unsafe {
                CreateAppContainerProfile(
                    name.as_ptr(),
                    display.as_ptr(),
                    description.as_ptr(),
                    null(),
                    0,
                    &mut sid,
                )
            };
            if result == 0 {
                drop(Sid(sid));
                Ok(())
            } else if result as u32 == 0x800700b7 {
                Ok(())
            } else {
                Err(format!(
                    "Cannot initialize Hover decoder AppContainer profile: HRESULT {result:#x}"
                ))
            }
        })
        .clone()
}

fn error(action: &str) -> String {
    format!("{action}: {}", std::io::Error::last_os_error())
}
fn win32(code: u32, action: &str) -> Result<(), String> {
    if code == 0 {
        Ok(())
    } else {
        Err(format!(
            "{action}: {}",
            std::io::Error::from_raw_os_error(code as i32)
        ))
    }
}
fn wide(value: &std::ffi::OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}
struct Sid(PSID);
impl Sid {
    fn decoder() -> Result<Self, String> {
        let name = wide(std::ffi::OsStr::new(MONIKER));
        let mut sid = null_mut();
        let result = unsafe { DeriveAppContainerSidFromAppContainerName(name.as_ptr(), &mut sid) };
        if result < 0 {
            return Err(format!(
                "Cannot derive decoder AppContainer SID: HRESULT {result:#x}"
            ));
        }
        Ok(Self(sid))
    }
}
impl Drop for Sid {
    fn drop(&mut self) {
        unsafe {
            FreeSid(self.0);
        }
    }
}
struct LocalAllocation(*mut core::ffi::c_void);
impl Drop for LocalAllocation {
    fn drop(&mut self) {
        unsafe {
            LocalFree(self.0);
        }
    }
}

fn capability(name: &str) -> Result<LocalAllocation, String> {
    let name = wide(std::ffi::OsStr::new(name));
    let (mut groups, mut group_count, mut caps, mut count) = (null_mut(), 0, null_mut(), 0);
    let result = unsafe {
        DeriveCapabilitySidsFromName(
            name.as_ptr(),
            &mut groups,
            &mut group_count,
            &mut caps,
            &mut count,
        )
    };
    let failure = std::io::Error::last_os_error();
    let mut all = Vec::new();
    if !groups.is_null() {
        for i in 0..group_count {
            unsafe {
                LocalFree(*groups.add(i as usize));
            }
        }
        unsafe {
            LocalFree(groups.cast());
        }
    }
    if !caps.is_null() {
        for i in 0..count {
            all.push(LocalAllocation(unsafe { *caps.add(i as usize) }));
        }
        unsafe {
            LocalFree(caps.cast());
        }
    }
    if result == 0 {
        return Err(format!(
            "Cannot derive system decoder capability: {failure}"
        ));
    }
    if all.len() != 1 {
        return Err("Unexpected decoder capability count".into());
    }
    Ok(all.remove(0))
}

// Change only our trusted executable's DACL, never preview files/directories.
// The exact sandbox SID gets RX, preserving all existing grants/ownership.
// Keep this handle open through creation to prevent replacement during launch.
fn executable_access(path: &Path, sid: PSID) -> Result<File, String> {
    let executable = OpenOptions::new()
        .access_mode(READ_CONTROL)
        .share_mode(FILE_SHARE_READ)
        .open(path)
        .map_err(|e| format!("Cannot hold decoder executable: {e}"))?;
    let mut dacl = null_mut();
    let mut descriptor = null_mut();
    win32(
        unsafe {
            GetSecurityInfo(
                executable.as_raw_handle(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                null_mut(),
                null_mut(),
                &mut dacl,
                null_mut(),
                &mut descriptor,
            )
        },
        "Cannot inspect decoder executable ACL",
    )?;
    let _descriptor = LocalAllocation(descriptor);
    // A null DACL already grants access. Do not change or broaden it further.
    if dacl.is_null() {
        return Ok(executable);
    }
    let mut trustee = TRUSTEE_W::default();
    unsafe {
        BuildTrusteeWithSidW(&mut trustee, sid);
    }
    let required = FILE_GENERIC_READ | FILE_GENERIC_EXECUTE;
    let mut granted = 0;
    win32(
        unsafe { GetEffectiveRightsFromAclW(dacl, &trustee, &mut granted) },
        "Cannot inspect sandbox executable rights",
    )?;
    if granted & required == required {
        return Ok(executable);
    }
    // Opening with WRITE_DAC can legitimately fail for a system installation;
    // its installer must provision this same narrow grant. Never elevate.
    let writable = OpenOptions::new()
        .access_mode(READ_CONTROL | WRITE_DAC)
        .share_mode(FILE_SHARE_READ)
        .open(path)
        .map_err(|e| {
            format!(
                "Decoder executable needs its AppContainer RX grant; repair the installation: {e}"
            )
        })?;
    let entry = EXPLICIT_ACCESS_W {
        grfAccessPermissions: required,
        grfAccessMode: GRANT_ACCESS,
        grfInheritance: NO_INHERITANCE,
        Trustee: trustee,
    };
    let mut updated = null_mut();
    win32(
        unsafe { SetEntriesInAclW(1, &entry, dacl, &mut updated) },
        "Cannot prepare sandbox executable grant",
    )?;
    let _updated = LocalAllocation(updated.cast());
    win32(
        unsafe {
            SetSecurityInfo(
                writable.as_raw_handle(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                null_mut(),
                null_mut(),
                updated,
                null(),
            )
        },
        "Cannot grant sandbox read/execute on Hover executable",
    )?;
    Ok(executable)
}

#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct AttributeBlock {
    _bytes: [u8; 16],
}
struct Attributes<'a> {
    _storage: Vec<AttributeBlock>,
    pointer: LPPROC_THREAD_ATTRIBUTE_LIST,
    _inputs: std::marker::PhantomData<&'a ()>,
}
impl<'a> Attributes<'a> {
    fn new(count: u32) -> Result<Self, String> {
        let mut size = 0;
        unsafe {
            InitializeProcThreadAttributeList(null_mut(), count, 0, &mut size);
        }
        if size == 0 {
            return Err(error("Cannot size decoder process attributes"));
        }
        let mut storage =
            vec![AttributeBlock { _bytes: [0; 16] }; size.div_ceil(size_of::<AttributeBlock>())];
        let pointer = storage.as_mut_ptr().cast();
        if unsafe { InitializeProcThreadAttributeList(pointer, count, 0, &mut size) } == 0 {
            return Err(error("Cannot initialize decoder process attributes"));
        }
        Ok(Self {
            _storage: storage,
            pointer,
            _inputs: std::marker::PhantomData,
        })
    }
    fn set<T>(&mut self, key: u32, value: &'a T) -> Result<(), String> {
        if unsafe {
            UpdateProcThreadAttribute(
                self.pointer,
                0,
                key as usize,
                value as *const T as *const _,
                size_of::<T>(),
                null_mut(),
                null(),
            )
        } == 0
        {
            Err(error("Cannot enforce decoder process attribute"))
        } else {
            Ok(())
        }
    }
}
impl Drop for Attributes<'_> {
    fn drop(&mut self) {
        unsafe {
            DeleteProcThreadAttributeList(self.pointer);
        }
    }
}
fn pipe(parent_writes: bool) -> Result<(File, File), String> {
    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: null_mut(),
        bInheritHandle: 1,
    };
    let (mut read, mut write) = (null_mut(), null_mut());
    if unsafe { CreatePipe(&mut read, &mut write, &attributes, 0) } == 0 {
        return Err(error("Cannot create private decoder pipe"));
    }
    let (read, write) = unsafe { (File::from_raw_handle(read), File::from_raw_handle(write)) };
    let (parent, child) = if parent_writes {
        (write, read)
    } else {
        (read, write)
    };
    if unsafe { SetHandleInformation(parent.as_raw_handle(), HANDLE_FLAG_INHERIT, 0) } == 0 {
        return Err(error("Cannot prevent decoder parent-handle inheritance"));
    }
    Ok((parent, child))
}

pub(crate) struct Child {
    process: OwnedHandle,
    pub(crate) input: Option<File>,
    pub(crate) output: Option<File>,
    pub(crate) pid: u32,
}
impl Child {
    pub(crate) fn id(&self) -> u32 {
        self.pid
    }
    pub(crate) fn pipes(&mut self) -> (File, File) {
        (self.input.take().unwrap(), self.output.take().unwrap())
    }
    pub(crate) fn handle(&self) -> HANDLE {
        self.process.as_raw_handle()
    }
    pub(crate) fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>> {
        match unsafe { WaitForSingleObject(self.handle(), 0) } {
            WAIT_TIMEOUT => Ok(None),
            WAIT_OBJECT_0 => {
                let mut code = 0;
                if unsafe { GetExitCodeProcess(self.handle(), &mut code) } == 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(Some(ExitStatus::from_raw(code)))
            }
            _ => Err(std::io::Error::last_os_error()),
        }
    }
    pub(crate) fn kill(&mut self) -> std::io::Result<()> {
        if unsafe { TerminateProcess(self.handle(), 0x484f5602) } == 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}
impl Drop for Child {
    fn drop(&mut self) {
        if !matches!(self.try_wait(), Ok(Some(_))) {
            let _ = self.kill();
        }
    }
}

fn token_info(token: HANDLE, kind: TOKEN_INFORMATION_CLASS) -> Result<Vec<usize>, String> {
    let mut size = 0;
    unsafe {
        GetTokenInformation(token, kind, null_mut(), 0, &mut size);
    }
    if size == 0 || size > 65536 {
        return Err(error("Cannot size decoder token information"));
    }
    let mut storage = vec![0usize; (size as usize).div_ceil(size_of::<usize>())];
    if unsafe { GetTokenInformation(token, kind, storage.as_mut_ptr().cast(), size, &mut size) }
        == 0
    {
        return Err(error("Cannot inspect decoder token"));
    }
    Ok(storage)
}
fn sid_text(sid: PSID) -> Result<String, String> {
    let mut text = null_mut();
    if unsafe { ConvertSidToStringSidW(sid, &mut text) } == 0 {
        return Err(error("Cannot format decoder security identity"));
    }
    let _text = LocalAllocation(text.cast());
    let mut length = 0;
    while length < 256 && unsafe { *text.add(length) } != 0 {
        length += 1;
    }
    if length == 256 {
        return Err("Invalid decoder SID string length".into());
    }
    String::from_utf16(unsafe { std::slice::from_raw_parts(text, length) })
        .map_err(|_| "Invalid decoder SID encoding".into())
}
fn access_check(token: HANDLE, user: PSID, package: &str) -> Result<bool, String> {
    let user = sid_text(user)?;
    let descriptor = wide(std::ffi::OsStr::new(&format!(
        "O:{user}G:{user}D:(A;;FR;;;{user})(A;;FR;;;{package})"
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
        return Err(error("Cannot construct decoder authorization check"));
    }
    let _security = LocalAllocation(security);
    let mut duplicate = null_mut();
    if unsafe {
        DuplicateTokenEx(
            token,
            TOKEN_QUERY | TOKEN_IMPERSONATE,
            null(),
            SecurityImpersonation,
            TokenImpersonation,
            &mut duplicate,
        )
    } == 0
    {
        return Err(error("Cannot prepare decoder authorization token"));
    }
    let duplicate = unsafe { OwnedHandle::from_raw_handle(duplicate) };
    let mapping = GENERIC_MAPPING {
        GenericRead: FILE_GENERIC_READ,
        GenericWrite: FILE_GENERIC_WRITE,
        GenericExecute: FILE_GENERIC_EXECUTE,
        GenericAll: FILE_ALL_ACCESS,
    };
    let mut privilege = [0usize; 128];
    let mut length = size_of_val(&privilege) as u32;
    let (mut granted, mut allowed) = (0, 0);
    if unsafe {
        AccessCheck(
            security,
            duplicate.as_raw_handle(),
            FILE_GENERIC_READ,
            &mapping,
            privilege.as_mut_ptr().cast(),
            &mut length,
            &mut granted,
            &mut allowed,
        )
    } == 0
    {
        return Err(error("Cannot verify decoder authorization semantics"));
    }
    Ok(allowed != 0 && granted & FILE_GENERIC_READ == FILE_GENERIC_READ)
}
pub(crate) fn verify_token(process: HANDLE) -> Result<(), String> {
    let mut token = null_mut();
    if unsafe { OpenProcessToken(process, TOKEN_QUERY | TOKEN_DUPLICATE, &mut token) } == 0 {
        return Err(error("Cannot open decoder token"));
    }
    let token = unsafe { OwnedHandle::from_raw_handle(token) };
    let app = token_info(token.as_raw_handle(), TokenIsAppContainer)?;
    if app[0] as u32 != 1 {
        return Err("Decoder is not an AppContainer".into());
    }
    let capabilities = token_info(token.as_raw_handle(), TokenCapabilities)?;
    let groups = unsafe { &*capabilities.as_ptr().cast::<TOKEN_GROUPS>() };
    if groups.GroupCount != 2
        || capabilities.len() * size_of::<usize>()
            < std::mem::offset_of!(TOKEN_GROUPS, Groups) + 2 * size_of::<SID_AND_ATTRIBUTES>()
    {
        return Err("Decoder has unexpected capabilities".into());
    }
    for name in ["lpacCom", "registryRead"] {
        let expected = capability(name)?;
        if !(0..2).any(|i| {
            let group = unsafe { &*groups.Groups.as_ptr().add(i) };
            group.Attributes & SE_GROUP_ENABLED as u32 != 0
                && unsafe { EqualSid(group.Sid, expected.0) } != 0
        }) {
            return Err("Unexpected decoder system capability".into());
        }
    }
    // The LPAC information class lacks a portable Win32 query contract (and
    // returned ERROR_INVALID_PARAMETER here). Check its documented access
    // semantics with the actual token before allowing any child code to run.
    let user = token_info(token.as_raw_handle(), TokenUser)?;
    let user = unsafe { &*user.as_ptr().cast::<TOKEN_USER>() };
    let identity = token_info(token.as_raw_handle(), TokenAppContainerSid)?;
    let identity = unsafe { &*identity.as_ptr().cast::<TOKEN_APPCONTAINER_INFORMATION>() };
    let expected = Sid::decoder()?;
    if unsafe { EqualSid(identity.TokenAppContainer, expected.0) } == 0 {
        return Err("Unexpected decoder AppContainer identity".into());
    }
    if !access_check(token.as_raw_handle(), user.User.Sid, &sid_text(expected.0)?)?
        || access_check(token.as_raw_handle(), user.User.Sid, "S-1-15-2-1")?
    {
        return Err("Decoder authorization is not LPAC: expected package access and ALL APPLICATION PACKAGES exclusion".into());
    }
    let integrity = token_info(token.as_raw_handle(), TokenIntegrityLevel)?;
    let integrity = unsafe { &*integrity.as_ptr().cast::<TOKEN_MANDATORY_LABEL>() };
    let count = unsafe { *GetSidSubAuthorityCount(integrity.Label.Sid) };
    if count == 0 || unsafe { *GetSidSubAuthority(integrity.Label.Sid, (count - 1) as u32) } != 4096
    {
        return Err("Decoder is not low integrity".into());
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub(crate) enum Entry {
    Decoder,
    #[cfg(test)]
    PermissionProbe(usize),
    #[cfg(test)]
    FaultProbe(&'static str),
    #[cfg(test)]
    RegularAppContainerControl,
}
pub(crate) fn spawn(job: HANDLE, entry: Entry) -> Result<Child, String> {
    ensure_profile()?;
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let sid = Sid::decoder()?;
    let _executable = executable_access(&executable, sid.0)?;
    let (input, child_input) = pipe(true)?;
    let (output, child_output) = pipe(false)?;
    let capability_sids = [capability("lpacCom")?, capability("registryRead")?];
    let mut capability_entries = capability_sids
        .iter()
        .map(|cap| SID_AND_ATTRIBUTES {
            Sid: cap.0,
            Attributes: SE_GROUP_ENABLED as u32,
        })
        .collect::<Vec<_>>();
    let capabilities = SECURITY_CAPABILITIES {
        AppContainerSid: sid.0,
        Capabilities: capability_entries.as_mut_ptr(),
        CapabilityCount: capability_entries.len() as u32,
        Reserved: 0,
    };
    let handles = [child_input.as_raw_handle(), child_output.as_raw_handle()];
    let jobs = [job];
    let all_packages_policy = 1u32; // PROCESS_CREATION_ALL_APPLICATION_PACKAGES_OPT_OUT (LPAC)
    let lpac = match entry {
        #[cfg(test)]
        Entry::RegularAppContainerControl => false,
        _ => true,
    };
    let mut attributes = Attributes::new(if lpac { 4 } else { 3 })?;
    attributes.set(PROC_THREAD_ATTRIBUTE_HANDLE_LIST, &handles)?;
    attributes.set(PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES, &capabilities)?;
    attributes.set(PROC_THREAD_ATTRIBUTE_JOB_LIST, &jobs)?;
    if lpac {
        attributes.set(
            PROC_THREAD_ATTRIBUTE_ALL_APPLICATION_PACKAGES_POLICY,
            &all_packages_policy,
        )?;
    }
    let startup = STARTUPINFOEXW {
        StartupInfo: STARTUPINFOW {
            cb: size_of::<STARTUPINFOEXW>() as u32,
            dwFlags: STARTF_USESTDHANDLES,
            hStdInput: handles[0],
            hStdOutput: handles[1],
            hStdError: handles[1],
            ..Default::default()
        },
        lpAttributeList: attributes.pointer,
    };
    let application = wide(executable.as_os_str());
    if application[..application.len() - 1].contains(&34) {
        return Err("Unsupported quote in Hover executable path".into());
    }
    let mut command = vec![34u16];
    command.extend_from_slice(&application[..application.len() - 1]);
    let arguments = match entry {
        Entry::Decoder => "\" --decode-worker",
        #[cfg(test)]
        Entry::RegularAppContainerControl => "\" --decode-worker",
        #[cfg(test)]
        Entry::PermissionProbe(_) => {
            "\" --exact decoder_sandbox::tests::permission_probe --ignored --nocapture --test-threads=1"
        }
        #[cfg(test)]
        Entry::FaultProbe(_) => {
            "\" --exact decoder_process::tests::fault_probe --ignored --nocapture --test-threads=1"
        }
    };
    command.extend(arguments.encode_utf16().chain(Some(0)));
    let mut system = vec![0u16; 32768];
    let count = unsafe { GetSystemDirectoryW(system.as_mut_ptr(), system.len() as u32) } as usize;
    if count == 0 || count >= system.len() {
        return Err(error("Cannot obtain Windows system directory"));
    }
    system.truncate(count + 1);
    let mut windows = vec![0u16; 32768];
    let count =
        unsafe { GetSystemWindowsDirectoryW(windows.as_mut_ptr(), windows.len() as u32) } as usize;
    if count == 0 || count >= windows.len() {
        return Err(error("Cannot obtain Windows root directory"));
    }
    windows.truncate(count);
    let mut local = null_mut();
    let result = unsafe { SHGetKnownFolderPath(&FOLDERID_LocalAppData, 0, null_mut(), &mut local) };
    if result < 0 {
        return Err(format!(
            "Cannot locate AppContainer profile root: HRESULT {result:#x}"
        ));
    }
    let mut local_len = 0usize;
    while local_len < 32768 && unsafe { *local.add(local_len) } != 0 {
        local_len += 1;
    }
    if local_len == 32768 {
        unsafe {
            CoTaskMemFree(local.cast());
        }
        return Err("Invalid AppContainer profile-root length".into());
    }
    let local_app_data = unsafe { std::slice::from_raw_parts(local, local_len) }.to_vec();
    unsafe {
        CoTaskMemFree(local.cast());
    }
    // Only required OS/profile paths, never arbitrary inherited environment or
    // credentials. Windows remaps LOCALAPPDATA into this AppContainer's profile.
    let mut environment = Vec::new();
    #[cfg(test)]
    if let Entry::FaultProbe(case) = entry {
        if !["sleep", "exit", "memory", "spawn", "cpu"].contains(&case) {
            return Err("Unknown native test probe".into());
        }
        environment.extend(format!("HOVER_DECODER_PROBE_CASE={case}").encode_utf16());
        environment.push(0);
    }
    for (key, value) in [
        ("LOCALAPPDATA=", local_app_data.as_slice()),
        ("PATH=", &system[..system.len() - 1]),
        ("SystemRoot=", windows.as_slice()),
        ("WINDIR=", windows.as_slice()),
    ] {
        environment.extend(key.encode_utf16());
        environment.extend(value);
        environment.push(0);
    }
    environment.push(0);
    let mut info = PROCESS_INFORMATION::default();
    if unsafe {
        CreateProcessW(
            application.as_ptr(),
            command.as_mut_ptr(),
            null(),
            null(),
            1,
            EXTENDED_STARTUPINFO_PRESENT
                | CREATE_SUSPENDED
                | CREATE_NO_WINDOW
                | CREATE_UNICODE_ENVIRONMENT,
            environment.as_ptr().cast(),
            system.as_ptr(),
            &startup.StartupInfo,
            &mut info,
        )
    } == 0
    {
        return Err(error(
            "Cannot launch AppContainer decoder; no image was sent",
        ));
    }
    let mut child = Child {
        process: unsafe { OwnedHandle::from_raw_handle(info.hProcess) },
        input: Some(input),
        output: Some(output),
        pid: info.dwProcessId,
    };
    let thread = unsafe { OwnedHandle::from_raw_handle(info.hThread) };
    verify_token(child.handle())?;
    #[cfg(test)]
    if let Entry::PermissionProbe(unlisted) = entry {
        tests::verify_handle_exclusion(child.handle(), unlisted as HANDLE)?;
    }
    if unsafe { ResumeThread(thread.as_raw_handle()) } == u32::MAX {
        let _ = child.kill();
        return Err(error("Cannot resume contained decoder"));
    }
    Ok(child)
}

#[cfg(test)]
#[path = "decoder_sandbox/tests.rs"]
mod tests;
