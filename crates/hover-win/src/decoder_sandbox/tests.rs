use super::*;
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use windows_sys::Win32::System::Registry::*;

pub(super) fn verify_handle_exclusion(child: HANDLE, original: HANDLE) -> Result<(), String> {
    // Inspect our suspended child. A handle value absent there may collide
    // with another object; do not query it from inside the worker.
    let mut control = null_mut();
    if unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            original,
            GetCurrentProcess(),
            &mut control,
            0,
            0,
            DUPLICATE_SAME_ACCESS,
        )
    } == 0
    {
        return Err(error("Cannot duplicate synthetic handle positive control"));
    }
    let control = unsafe { OwnedHandle::from_raw_handle(control) };
    if unsafe { CompareObjectHandles(original, control.as_raw_handle()) } == 0 {
        return Err("Handle-comparison positive control failed".into());
    }
    let mut remote = null_mut();
    if unsafe {
        DuplicateHandle(
            child,
            original,
            GetCurrentProcess(),
            &mut remote,
            0,
            0,
            DUPLICATE_SAME_ACCESS,
        )
    } == 0
    {
        let error = std::io::Error::last_os_error();
        return if error.raw_os_error() == Some(6) {
            Ok(())
        } else {
            Err(format!("Cannot inspect owned child handle table: {error}"))
        };
    }
    let remote = unsafe { OwnedHandle::from_raw_handle(remote) };
    if unsafe { CompareObjectHandles(original, remote.as_raw_handle()) } != 0 {
        Err("Private sentinel handle was inherited outside the explicit pipe list".into())
    } else {
        Ok(())
    }
}

fn private_acl(file: &File, shared: Option<PSID>) {
    let mut token = null_mut();
    assert_ne!(
        unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) },
        0
    );
    let token = unsafe { OwnedHandle::from_raw_handle(token) };
    let user = token_info(token.as_raw_handle(), TokenUser).unwrap();
    let user = unsafe { &*user.as_ptr().cast::<TOKEN_USER>() };
    let mut trustee = TRUSTEE_W::default();
    unsafe {
        BuildTrusteeWithSidW(&mut trustee, user.User.Sid);
    }
    let mut entries = vec![EXPLICIT_ACCESS_W {
        grfAccessPermissions: FILE_ALL_ACCESS,
        grfAccessMode: GRANT_ACCESS,
        grfInheritance: NO_INHERITANCE,
        Trustee: trustee,
    }];
    if let Some(sid) = shared {
        let mut trustee = TRUSTEE_W::default();
        unsafe {
            BuildTrusteeWithSidW(&mut trustee, sid);
        }
        entries.push(EXPLICIT_ACCESS_W {
            grfAccessPermissions: FILE_GENERIC_READ,
            grfAccessMode: GRANT_ACCESS,
            grfInheritance: NO_INHERITANCE,
            Trustee: trustee,
        });
    }
    let mut acl = null_mut();
    win32(
        unsafe { SetEntriesInAclW(entries.len() as u32, entries.as_ptr(), null(), &mut acl) },
        "fixture ACL",
    )
    .unwrap();
    let _acl = LocalAllocation(acl.cast());
    win32(
        unsafe {
            SetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                null_mut(),
                null_mut(),
                acl,
                null(),
            )
        },
        "protect synthetic fixture",
    )
    .unwrap();
}

struct RegistryFixture {
    path: Vec<u16>,
}
impl Drop for RegistryFixture {
    fn drop(&mut self) {
        unsafe {
            RegDeleteKeyExW(HKEY_CURRENT_USER, self.path.as_ptr(), KEY_WOW64_64KEY, 0);
        }
    }
}
fn registry_fixture(id: &str) -> (RegistryFixture, String) {
    // RegCreateKeyEx applies REG_OPTION_VOLATILE to intermediate keys too.
    // Keep the volatile leaf directly under existing Software: creating it
    // below the real app namespace can make that namespace volatile on a
    // fresh profile and prevent a later installer from creating stable keys.
    let path = wide(std::ffi::OsStr::new(&format!(
        "Software\\HoverDecoderPermissionProbe-{id}"
    )));
    let (mut key, mut disposition) = (null_mut(), 0);
    win32(
        unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                path.as_ptr(),
                0,
                null(),
                REG_OPTION_VOLATILE,
                KEY_ALL_ACCESS | KEY_WOW64_64KEY,
                null(),
                &mut key,
                &mut disposition,
            )
        },
        "create synthetic registry sentinel",
    )
    .unwrap();
    assert_eq!(disposition, REG_CREATED_NEW_KEY);
    let fixture = RegistryFixture { path };
    let value = wide(std::ffi::OsStr::new("sentinel"));
    let bytes = 0x484f5652u32.to_le_bytes();
    let set = unsafe { RegSetValueExW(key, value.as_ptr(), 0, REG_DWORD, bytes.as_ptr(), 4) };
    unsafe {
        RegCloseKey(key);
    }
    win32(set, "write registry sentinel").unwrap();
    let mut token = null_mut();
    assert_ne!(
        unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) },
        0
    );
    let token = unsafe { OwnedHandle::from_raw_handle(token) };
    let user = token_info(token.as_raw_handle(), TokenUser).unwrap();
    let user = unsafe { &*user.as_ptr().cast::<TOKEN_USER>() };
    let mut text = null_mut();
    assert_ne!(
        unsafe { ConvertSidToStringSidW(user.User.Sid, &mut text) },
        0
    );
    let _text = LocalAllocation(text.cast());
    let mut length = 0;
    while unsafe { *text.add(length) } != 0 {
        length += 1;
    }
    let sid = String::from_utf16(unsafe { std::slice::from_raw_parts(text, length) }).unwrap();
    let absolute = format!("{sid}\\Software\\HoverDecoderPermissionProbe-{id}");
    let absolute_wide = wide(std::ffi::OsStr::new(&absolute));
    let mut check = null_mut();
    win32(
        unsafe {
            RegOpenKeyExW(
                HKEY_USERS,
                absolute_wide.as_ptr(),
                0,
                KEY_READ | KEY_WOW64_64KEY,
                &mut check,
            )
        },
        "parent registry positive control",
    )
    .unwrap();
    let mut actual = [0u8; 4];
    let mut size = 4;
    let read = unsafe {
        RegQueryValueExW(
            check,
            value.as_ptr(),
            null(),
            null_mut(),
            actual.as_mut_ptr(),
            &mut size,
        )
    };
    unsafe {
        RegCloseKey(check);
    }
    win32(read, "parent registry value control").unwrap();
    assert_eq!(actual, bytes);
    (fixture, absolute)
}

#[test]
fn appcontainer_denies_private_resources_and_unlisted_handles() {
    let id = format!(
        "{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let root = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join("sandbox-evidence")
        .join(&id);
    std::fs::create_dir_all(&root).unwrap();
    let private_path = root.join("private-sentinel.txt");
    let shared_path = root.join("read-only-sentinel.txt");
    let all_packages_path = root.join("all-packages-sentinel.txt");
    let marker = b"Hover generated permission sentinel\n";
    let mut private = OpenOptions::new()
        .read(true)
        .write(true)
        .access_mode(FILE_GENERIC_READ | FILE_GENERIC_WRITE | WRITE_DAC)
        .create_new(true)
        .open(&private_path)
        .unwrap();
    private.write_all(marker).unwrap();
    private.flush().unwrap();
    private_acl(&private, None);
    let mut shared = OpenOptions::new()
        .read(true)
        .write(true)
        .access_mode(FILE_GENERIC_READ | FILE_GENERIC_WRITE | WRITE_DAC)
        .create_new(true)
        .open(&shared_path)
        .unwrap();
    shared.write_all(marker).unwrap();
    shared.flush().unwrap();
    let decoder = Sid::decoder().unwrap();
    private_acl(&shared, Some(decoder.0));
    let mut all_packages = OpenOptions::new()
        .read(true)
        .write(true)
        .access_mode(FILE_GENERIC_READ | FILE_GENERIC_WRITE | WRITE_DAC)
        .create_new(true)
        .open(&all_packages_path)
        .unwrap();
    all_packages.write_all(marker).unwrap();
    all_packages.flush().unwrap();
    let mut all_sid = null_mut();
    let all_sid_string = wide(std::ffi::OsStr::new("S-1-15-2-1"));
    assert_ne!(
        unsafe { ConvertStringSidToSidW(all_sid_string.as_ptr(), &mut all_sid) },
        0
    );
    let _all_sid = LocalAllocation(all_sid);
    private_acl(&all_packages, Some(all_sid));
    assert_eq!(std::fs::read(&private_path).unwrap(), marker);
    assert!(OpenOptions::new().write(true).open(&private_path).is_ok());
    assert_ne!(
        unsafe {
            SetHandleInformation(
                private.as_raw_handle(),
                HANDLE_FLAG_INHERIT,
                HANDLE_FLAG_INHERIT,
            )
        },
        0
    );
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let positive = TcpStream::connect(("127.0.0.1", port)).unwrap();
    let accepted = listener.accept().unwrap();
    drop(positive);
    drop(accepted);
    let (_registry, registry) = registry_fixture(&id);
    let request = json!({"private_path":private_path,"shared_path":shared_path,"all_packages_path":all_packages_path,"parent_pid":std::process::id(),
        "port":port,"registry_path":registry,"unlisted_handle":private.as_raw_handle() as usize});
    let result = crate::decoder_process::permission_probe(&request).unwrap();
    println!("Permission results: {result}");
    assert!(result["token_verified"].as_bool().unwrap());
    assert_eq!(result["private_read_error"], 5);
    assert_eq!(result["private_write_error"], 5);
    assert_eq!(result["shared_read_matches"], true);
    assert_eq!(result["shared_write_error"], 5);
    assert_eq!(result["all_packages_read_error"], 5);
    assert_eq!(result["parent_write_handle_error"], 5);
    assert_eq!(result["parent_read_handle_error"], 5);
    assert!([2, 5].contains(&result["registry_read_error"].as_u64().unwrap()));
    assert_eq!(result["network_error"], 10013);
    assert_eq!(result["private_handle_inherited"], false);
    listener.set_nonblocking(true).unwrap();
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert_eq!(std::fs::read(&private_path).unwrap(), marker);
    assert_eq!(std::fs::read(&shared_path).unwrap(), marker);
    assert_eq!(std::fs::read(&all_packages_path).unwrap(), marker);
}

#[test]
#[ignore = "Native child probe only; compiled out of the shipping executable"]
fn permission_probe() {
    unsafe {
        windows_sys::Win32::System::Diagnostics::Debug::SetErrorMode(0x8003);
    }
    println!("HOVER_PROBE_STAGE=token");
    verify_token(unsafe { GetCurrentProcess() }).unwrap();
    println!("HOVER_PROBE_STAGE=input");
    let mut input = String::new();
    std::io::stdin()
        .take(8193)
        .read_to_string(&mut input)
        .unwrap();
    assert!(input.len() <= 8192);
    let request: Value = serde_json::from_str(&input).unwrap();
    println!("HOVER_PROBE_STAGE=resources");
    let private = Path::new(request["private_path"].as_str().unwrap());
    let shared = Path::new(request["shared_path"].as_str().unwrap());
    let all_packages = Path::new(request["all_packages_path"].as_str().unwrap());
    let all_packages_read_error = std::fs::read(all_packages)
        .err()
        .and_then(|e| e.raw_os_error())
        .unwrap_or(0);
    let private_read_error = std::fs::read(private)
        .err()
        .and_then(|e| e.raw_os_error())
        .unwrap_or(0);
    let private_write_error = OpenOptions::new()
        .write(true)
        .open(private)
        .err()
        .and_then(|e| e.raw_os_error())
        .unwrap_or(0);
    let shared_read_matches =
        std::fs::read(shared).is_ok_and(|data| data == b"Hover generated permission sentinel\n");
    let shared_write_error = OpenOptions::new()
        .write(true)
        .open(shared)
        .err()
        .and_then(|e| e.raw_os_error())
        .unwrap_or(0);
    let process = unsafe {
        OpenProcess(
            PROCESS_VM_WRITE | PROCESS_VM_OPERATION,
            0,
            request["parent_pid"].as_u64().unwrap() as u32,
        )
    };
    let parent_write_handle_error = if process.is_null() {
        std::io::Error::last_os_error().raw_os_error().unwrap_or(-1)
    } else {
        unsafe {
            CloseHandle(process);
        }
        0
    };
    let process = unsafe {
        OpenProcess(
            PROCESS_VM_READ,
            0,
            request["parent_pid"].as_u64().unwrap() as u32,
        )
    };
    let parent_read_handle_error = if process.is_null() {
        std::io::Error::last_os_error().raw_os_error().unwrap_or(-1)
    } else {
        unsafe {
            CloseHandle(process);
        }
        0
    };
    let registry = wide(std::ffi::OsStr::new(
        request["registry_path"].as_str().unwrap(),
    ));
    let mut key = null_mut();
    let registry_read_error = unsafe {
        RegOpenKeyExW(
            HKEY_USERS,
            registry.as_ptr(),
            0,
            KEY_READ | KEY_WOW64_64KEY,
            &mut key,
        )
    };
    if registry_read_error == 0 {
        unsafe {
            RegCloseKey(key);
        }
    }
    let address = SocketAddr::from(([127, 0, 0, 1], request["port"].as_u64().unwrap() as u16));
    let network = TcpStream::connect_timeout(&address, Duration::from_millis(500));
    let network_connected = network.is_ok();
    let network_error = network
        .err()
        .map(|e| e.raw_os_error().unwrap_or(-1))
        .unwrap_or(0);
    let result = json!({"token_verified":true,"private_read_error":private_read_error,"private_write_error":private_write_error,
        "shared_read_matches":shared_read_matches,"shared_write_error":shared_write_error,"all_packages_read_error":all_packages_read_error,"parent_write_handle_error":parent_write_handle_error,"parent_read_handle_error":parent_read_handle_error,
        "registry_read_error":registry_read_error,"network_error":network_error,"network_connected":network_connected});
    println!("HOVER_PERMISSION_RESULT={result}");
}
