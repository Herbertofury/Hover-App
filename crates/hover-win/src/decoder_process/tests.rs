use super::*;
use std::io::{BufRead, BufReader, Cursor};
use std::sync::Arc;

const READY: &[u8] = b"HOVER_DECODER_PROBE_READY";

fn read_response(reader: &mut impl Read) -> Result<Vec<DecodedFrame>, String> {
    read_response_limited(reader, 8192, 8192, MAX_FRAMES)
}

fn probe_command(case: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "decoder_process::tests::fault_probe",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("HOVER_DECODER_PROBE_CASE", case);
    command
}

fn probe_transfer(
    ready: Arc<AtomicBool>,
) -> impl FnOnce(std::process::ChildStdin, std::process::ChildStdout) -> Result<Vec<u8>, String> {
    move |mut input, output| {
        input.write_all(&[1]).map_err(|e| e.to_string())?;
        drop(input);
        let mut reader = BufReader::new(output);
        let mut all = Vec::new();
        loop {
            let mut line = Vec::new();
            let count = Read::take(&mut reader, 65536)
                .read_until(b'\n', &mut line)
                .map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            if line.windows(READY.len()).any(|window| window == READY) {
                ready.store(true, Ordering::Release);
            }
            if all.len() + line.len() > 65536 {
                return Err("Probe exceeded test-output budget".into());
            }
            all.extend(line);
        }
        Ok(all)
    }
}

#[test]
fn wall_timeout_kills_an_actually_started_uncooperative_process_and_pipe() {
    let ready = Arc::new(AtomicBool::new(false));
    let result = exchange(
        probe_command("sleep"),
        &CancellationToken::new(),
        Duration::from_secs(2),
        64 * 1024 * 1024,
        probe_transfer(ready.clone()),
    );
    assert!(
        ready.load(Ordering::Acquire),
        "Probe never reached its blocking code: {:?}",
        result.result
    );
    assert_eq!(result.reason, "timeout");
    assert!(result.elapsed < Duration::from_secs(4));
    assert_eq!(result.active_after_stop, Some(0));
    assert!(result.peak_bytes > 0);
}

#[test]
fn cancellation_stops_a_blocked_child_and_joins_its_pipe_reader() {
    let ready = Arc::new(AtomicBool::new(false));
    let token = CancellationToken::new();
    let other = token.clone();
    let signal = ready.clone();
    let cancel = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(3);
        while !signal.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        other.cancel();
    });
    let result = exchange(
        probe_command("sleep"),
        &token,
        Duration::from_secs(5),
        64 * 1024 * 1024,
        probe_transfer(ready.clone()),
    );
    cancel.join().unwrap();
    assert!(
        ready.load(Ordering::Acquire),
        "Probe did not enter: {:?}",
        result.result
    );
    assert_eq!(result.reason, "cancelled");
    assert!(result.elapsed < Duration::from_secs(3));
    assert_eq!(result.active_after_stop, Some(0));
}

#[test]
fn an_early_response_from_a_still_running_child_does_not_spin_the_supervisor() {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::{GetCurrentThread, GetThreadTimes};
    fn cpu_time() -> u64 {
        let (mut created, mut exited, mut kernel, mut user) = (
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
        );
        assert_ne!(
            unsafe {
                GetThreadTimes(
                    GetCurrentThread(),
                    &mut created,
                    &mut exited,
                    &mut kernel,
                    &mut user,
                )
            },
            0
        );
        ((kernel.dwHighDateTime as u64) << 32 | kernel.dwLowDateTime as u64)
            + ((user.dwHighDateTime as u64) << 32 | user.dwLowDateTime as u64)
    }
    let before = cpu_time();
    let result = exchange(
        probe_command("sleep"),
        &CancellationToken::new(),
        Duration::from_secs(2),
        64 * 1024 * 1024,
        |mut input, output| {
            input.write_all(&[1]).map_err(|e| e.to_string())?;
            drop(input);
            let mut reader = BufReader::new(output);
            loop {
                let mut line = Vec::new();
                let count = reader
                    .read_until(b'\n', &mut line)
                    .map_err(|e| e.to_string())?;
                if count == 0 {
                    return Err("Probe ended before its ready marker".into());
                }
                if line.windows(READY.len()).any(|window| window == READY) {
                    return Ok(());
                }
            }
        },
    );
    let used = cpu_time() - before;
    assert_eq!(result.reason, "timeout");
    assert_eq!(result.active_after_stop, Some(0));
    assert!(
        used < 2_500_000,
        "Supervisor used {} ms CPU while waiting",
        used / 10_000
    );
}

#[test]
fn native_memory_and_child_process_limits_are_enforced() {
    for (case, marker) in [("memory", "MEMORY_DENIED"), ("spawn", "SPAWN_DENIED")] {
        let ready = Arc::new(AtomicBool::new(false));
        let result = exchange(
            probe_command(case),
            &CancellationToken::new(),
            Duration::from_secs(5),
            64 * 1024 * 1024,
            probe_transfer(ready.clone()),
        );
        assert!(ready.load(Ordering::Acquire));
        let bytes = result
            .result
            .unwrap_or_else(|error| panic!("{case}: {error}"));
        assert!(
            String::from_utf8_lossy(&bytes).contains(marker),
            "{case} limit did not reject its operation"
        );
        assert_eq!(result.active_after_stop, Some(0));
        assert!(result.peak_bytes <= 64 * 1024 * 1024);
    }
}

#[test]
fn abrupt_decoder_exit_is_contained_and_the_next_process_can_run() {
    let result = exchange(
        probe_command("exit"),
        &CancellationToken::new(),
        Duration::from_secs(5),
        64 * 1024 * 1024,
        probe_transfer(Arc::new(AtomicBool::new(false))),
    );
    assert_eq!(result.reason, "child_exit");
    assert!(result.result.is_err());
    assert_eq!(result.active_after_stop, Some(0));
    let next = exchange(
        probe_command("memory"),
        &CancellationToken::new(),
        Duration::from_secs(5),
        64 * 1024 * 1024,
        probe_transfer(Arc::new(AtomicBool::new(false))),
    );
    assert!(next.result.is_ok());
    assert_eq!(next.active_after_stop, Some(0));
}

#[test]
fn strict_frame_protocol_rejects_truncation_and_hostile_allocation_headers() {
    let mut valid = RESPONSE_MAGIC.to_vec();
    valid.push(0);
    valid.extend(1u32.to_le_bytes());
    valid.extend(2u32.to_le_bytes());
    valid.extend(1u32.to_le_bytes());
    valid.extend(33u32.to_le_bytes());
    valid.extend([0, 0, 255, 255, 0, 255, 0, 255]);
    for prefix in 0..valid.len() {
        assert!(
            read_response(&mut Cursor::new(&valid[..prefix])).is_err(),
            "prefix {prefix}"
        );
    }
    let frame = read_response(&mut Cursor::new(&valid)).unwrap().remove(0);
    assert!(read_response_limited(&mut Cursor::new(&valid), 1, 1, 1).is_err());
    assert_eq!(
        (frame.width, frame.height, frame.stride, frame.delay_ms),
        (2, 1, 8, 33)
    );
    assert_eq!(frame.pixels_bgra, [0, 0, 255, 255, 0, 255, 0, 255]);
    let mut trailing = valid.clone();
    trailing.push(1);
    assert!(read_response(&mut Cursor::new(trailing)).is_err());
    for count in [0, 1001, u32::MAX] {
        let mut packet = RESPONSE_MAGIC.to_vec();
        packet.push(0);
        packet.extend(count.to_le_bytes());
        assert!(read_response(&mut Cursor::new(packet)).is_err());
    }
    for width in [0, 8193, u32::MAX] {
        let mut packet = valid.clone();
        packet[13..17].copy_from_slice(&width.to_le_bytes());
        assert!(read_response(&mut Cursor::new(packet)).is_err());
    }
    let mut error = RESPONSE_MAGIC.to_vec();
    error.push(1);
    error.extend(u32::MAX.to_le_bytes());
    assert!(read_response(&mut Cursor::new(error)).is_err());
}

#[test]
#[ignore = "Dedicated Windows CI only: intentionally consumes decoder CPU until the kernel limit terminates it"]
fn native_cpu_limit() {
    assert_eq!(std::env::var("GITHUB_ACTIONS").as_deref(), Ok("true"));
    let result = exchange(
        probe_command("cpu"),
        &CancellationToken::new(),
        Duration::from_secs(15),
        64 * 1024 * 1024,
        probe_transfer(Arc::new(AtomicBool::new(false))),
    );
    assert_eq!(result.reason, "child_exit");
    assert!(result.cpu_user_100ns >= 45_000_000);
    assert_eq!(result.active_after_stop, Some(0));
}

#[test]
#[ignore = "Subprocess probe only, never shipped in Hover and never a file-triggered behavior"]
fn fault_probe() {
    use windows_sys::Win32::System::Memory::{
        MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE, VirtualAlloc, VirtualFree,
    };
    unsafe {
        windows_sys::Win32::System::Diagnostics::Debug::SetErrorMode(0x8003);
    }
    let case = std::env::var("HOVER_DECODER_PROBE_CASE").expect("Subprocess-only fixture");
    std::io::stdin().read_exact(&mut [0]).unwrap();
    println!("{}", String::from_utf8_lossy(READY));
    std::io::stdout().flush().unwrap();
    match case.as_str() {
        "sleep" => std::thread::sleep(Duration::from_secs(30)),
        "exit" => std::process::exit(37),
        "cpu" => loop {
            std::hint::spin_loop();
        },
        "memory" => unsafe {
            let memory = VirtualAlloc(
                null(),
                256 * 1024 * 1024,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            );
            if memory.is_null() {
                println!("MEMORY_DENIED");
            } else {
                VirtualFree(memory, 0, MEM_RELEASE);
                println!("MEMORY_ALLOWED");
            }
        },
        "spawn" => {
            let program = std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap())
                .join("System32/cmd.exe");
            match Command::new(program)
                .args(["/d", "/c", "exit 0"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
            {
                Err(_) => println!("SPAWN_DENIED"),
                Ok(mut child) => {
                    let status = child.wait().unwrap();
                    if status.success() {
                        println!("SPAWN_ALLOWED");
                    } else {
                        println!("SPAWN_DENIED");
                    }
                }
            }
        }
        _ => panic!("Unknown controlled test probe"),
    }
}
