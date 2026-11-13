//! One authorized memory snapshot per short-lived, resource-limited decoder.
//! LPAC provides permissions; the Windows job provides crash/resource limits.
use hover_core::CancellationToken;
use hover_decode::{DecodeLimits, DecodedFrame};
use serde_json::{Value, json};
use std::fs::File;
use std::io::{Read, Write};
use std::os::windows::io::AsRawHandle;
use std::path::Path;
use std::ptr::{null, null_mut};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::System::IO::CancelSynchronousIo;
use windows_sys::Win32::System::JobObjects::*;

const REQUEST_MAGIC: &[u8; 8] = b"HOVREQ1\0";
const RESPONSE_MAGIC: &[u8; 8] = b"HOVRES1\0";
const MAX_INPUT: u64 = 512 * 1024 * 1024;
const MAX_OUTPUT: u64 = 256 * 1024 * 1024;
const MAX_FRAMES: u32 = 1_000;
const MEMORY_LIMIT: usize = 2 * 1024 * 1024 * 1024;
const WALL_LIMIT: Duration = Duration::from_secs(8);
static ACTIVE_PID: AtomicU32 = AtomicU32::new(0);
static POISONED: AtomicBool = AtomicBool::new(false);
static LAST_REPORT: Mutex<Option<Value>> = Mutex::new(None);

struct Job(HANDLE);
impl Drop for Job {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
impl Job {
    fn new(memory: usize) -> Result<Self, String> {
        let handle = unsafe { CreateJobObjectW(null(), null()) };
        if handle.is_null() {
            return Err(format!(
                "Cannot create decoder job: {}",
                std::io::Error::last_os_error()
            ));
        }
        let job = Self(handle);
        let limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
            BasicLimitInformation: JOBOBJECT_BASIC_LIMIT_INFORMATION {
                LimitFlags: JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
                    | JOB_OBJECT_LIMIT_ACTIVE_PROCESS
                    | JOB_OBJECT_LIMIT_PROCESS_MEMORY
                    | JOB_OBJECT_LIMIT_PROCESS_TIME,
                ActiveProcessLimit: 1,
                PerProcessUserTimeLimit: 5 * 10_000_000,
                ..Default::default()
            },
            ProcessMemoryLimit: memory,
            ..Default::default()
        };
        job.set(JobObjectExtendedLimitInformation, &limits)?;
        let cpu = JOBOBJECT_CPU_RATE_CONTROL_INFORMATION {
            ControlFlags: JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP,
            Anonymous: JOBOBJECT_CPU_RATE_CONTROL_INFORMATION_0 { CpuRate: 5_000 },
        };
        job.set(JobObjectCpuRateControlInformation, &cpu)?;
        // UI-limit jobs cannot be nested. Preserve the host's existing job;
        // never request breakaway. Permission sandboxing is a separate gate.
        // https://learn.microsoft.com/en-us/windows/win32/procthread/nested-jobs
        Ok(job)
    }
    fn set<T>(&self, kind: JOBOBJECTINFOCLASS, value: &T) -> Result<(), String> {
        if unsafe {
            SetInformationJobObject(
                self.0,
                kind,
                value as *const T as *const _,
                size_of::<T>() as u32,
            )
        } == 0
        {
            Err(format!(
                "Cannot enforce decoder resource limit: {}",
                std::io::Error::last_os_error()
            ))
        } else {
            Ok(())
        }
    }
    fn terminate(&self) {
        unsafe {
            TerminateJobObject(self.0, 0x484f5601);
        }
    }
    fn accounting(&self) -> Option<(usize, i64, i64, u32)> {
        let mut memory = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        let mut cpu = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
        let memory_ok = unsafe {
            QueryInformationJobObject(
                self.0,
                JobObjectExtendedLimitInformation,
                &mut memory as *mut _ as *mut _,
                size_of_val(&memory) as u32,
                null_mut(),
            )
        } != 0;
        let cpu_ok = unsafe {
            QueryInformationJobObject(
                self.0,
                JobObjectBasicAccountingInformation,
                &mut cpu as *mut _ as *mut _,
                size_of_val(&cpu) as u32,
                null_mut(),
            )
        } != 0;
        (memory_ok && cpu_ok).then_some((
            memory.PeakProcessMemoryUsed,
            cpu.TotalUserTime,
            cpu.TotalKernelTime,
            cpu.ActiveProcesses,
        ))
    }
}

struct Exchange<T> {
    result: Result<T, String>,
    pid: u32,
    peak_bytes: usize,
    cpu_user_100ns: i64,
    cpu_kernel_100ns: i64,
    elapsed: Duration,
    reason: &'static str,
    active_after_stop: Option<u32>,
    permission_verified: bool,
}

use crate::decoder_sandbox::{Child as ManagedChild, Entry as Launch};

// The closure owns only this child's pipes. The supervising worker can terminate
// the job even when native code or a pipe operation does not cooperate.
fn exchange<T: Send + 'static>(
    launch: Launch,
    token: &CancellationToken,
    wall: Duration,
    memory: usize,
    transfer: impl FnOnce(File, File) -> Result<T, String> + Send + 'static,
) -> Exchange<T> {
    let start = Instant::now();
    let mut report = Exchange {
        result: Err("Decoder could not start".into()),
        pid: 0,
        peak_bytes: 0,
        cpu_user_100ns: 0,
        cpu_kernel_100ns: 0,
        elapsed: Duration::ZERO,
        reason: "startup_error",
        active_after_stop: None,
        permission_verified: false,
    };
    if token.is_cancelled() {
        report.result = Err("Preview was cancelled".into());
        report.reason = "cancelled";
        report.elapsed = start.elapsed();
        return report;
    }
    if POISONED.load(Ordering::Acquire) {
        report.result = Err(
            "Decoder cleanup did not finish. Restart Hover before decoding another image.".into(),
        );
        report.elapsed = start.elapsed();
        return report;
    }
    let job = match Job::new(memory) {
        Ok(job) => job,
        Err(error) => {
            report.result = Err(error);
            report.elapsed = start.elapsed();
            return report;
        }
    };
    let spawned = crate::decoder_sandbox::spawn(job.0, launch);
    let mut child = match spawned {
        Ok(child) => child,
        Err(error) => {
            report.result = Err(format!("Cannot launch decoder: {error}"));
            report.elapsed = start.elapsed();
            return report;
        }
    };
    report.pid = child.id();
    report.permission_verified = true;
    // Native production creation atomically applies the job and AppContainer;
    // no input is sent before its actual Windows token has been verified.
    ACTIVE_PID.store(child.id(), Ordering::Release);
    let (input, output) = child.pipes();
    let (send, receive) = mpsc::sync_channel(1);
    let io = match std::thread::Builder::new()
        .name("hover-decoder-pipe".into())
        .spawn(move || {
            let result = transfer(input, output);
            let _ = send.try_send(result);
        }) {
        Ok(thread) => thread,
        Err(error) => {
            job.terminate();
            let _ = child.kill();
            if wait_for_exit(&mut child, Duration::from_secs(1)) {
                ACTIVE_PID.store(0, Ordering::Release);
            } else {
                POISONED.store(true, Ordering::Release);
            }
            report.result = Err(error.to_string());
            report.elapsed = start.elapsed();
            return report;
        }
    };
    let mut response = None;
    loop {
        if token.is_cancelled() {
            report.result = Err("Preview was cancelled; decoder stopped".into());
            report.reason = "cancelled";
            break;
        }
        if start.elapsed() >= wall {
            report.result =
                Err("Preview decoder exceeded its wall-time budget and was stopped".into());
            report.reason = "timeout";
            break;
        }
        let incoming = if response.is_some() {
            // A closed result channel returns immediately. Do not spin if the
            // child closed stdout but remains alive after a valid response.
            std::thread::sleep(Duration::from_millis(10));
            Err(mpsc::RecvTimeoutError::Timeout)
        } else {
            receive.recv_timeout(Duration::from_millis(10))
        };
        match incoming {
            Ok(Err(error)) => {
                report.result = Err(error);
                report.reason = "decode_or_protocol_error";
                break;
            }
            Ok(value) => response = Some(value),
            Err(mpsc::RecvTimeoutError::Disconnected) if response.is_none() => {
                report.result = Err("Decoder pipe worker stopped unexpectedly".into());
                report.reason = "pipe_error";
                break;
            }
            _ => {}
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                if let Some(value) = response.take() {
                    report.reason = if status.success() {
                        if value.is_ok() {
                            "success"
                        } else {
                            "decode_error"
                        }
                    } else {
                        "child_exit"
                    };
                    report.result = if status.success() {
                        value
                    } else {
                        Err(format!(
                            "Decoder stopped unexpectedly ({status}); the image may be corrupt or beyond its CPU/memory budget."
                        ))
                    };
                    break;
                }
            }
            Ok(None) => {}
            Err(error) => {
                report.result = Err(error.to_string());
                report.reason = "child_exit";
                break;
            }
        }
    }
    job.terminate();
    let _ = child.kill();
    let stopped = wait_for_exit(&mut child, Duration::from_secs(1));
    let drain_deadline = Instant::now() + Duration::from_secs(1);
    while !io.is_finished() && Instant::now() < drain_deadline {
        // Cancels only our own pipe thread, including a blocked synchronous
        // read/write. It does not touch another application or desktop input.
        unsafe {
            CancelSynchronousIo(io.as_raw_handle());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    if io.is_finished() {
        let _ = io.join();
    } else {
        POISONED.store(true, Ordering::Release);
        report.result =
            Err("Decoder pipe cleanup failed; restart Hover before further image reads.".into());
        report.reason = "cleanup_failed";
    }
    if !stopped {
        POISONED.store(true, Ordering::Release);
        report.result=Err("Windows did not finish stopping the decoder; restart Hover before further image reads.".into());
        report.reason = "cleanup_failed";
    }
    if let Some((peak, user, kernel, active)) = job.accounting() {
        report.peak_bytes = peak;
        report.cpu_user_100ns = user;
        report.cpu_kernel_100ns = kernel;
        report.active_after_stop = Some(active);
    }
    report.elapsed = start.elapsed();
    if stopped {
        ACTIVE_PID.store(0, Ordering::Release);
    }
    report
}

fn wait_for_exit(child: &mut ManagedChild, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return true,
            Err(_) => return false,
            _ => {}
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

pub(crate) fn diagnostics() -> Value {
    json!({"active_pid":ACTIVE_PID.load(Ordering::Acquire),"last_task":LAST_REPORT.lock().unwrap().clone(),
        "memory_limit_bytes":MEMORY_LIMIT,"output_limit_bytes":MAX_OUTPUT,"wall_limit_ms":WALL_LIMIT.as_millis(),
        "cpu_user_limit_ms":5000,"cpu_rate_percent":50,"permission_sandbox":true,"sandbox_kind":"Windows LPAC","capabilities":["lpacCom","registryRead"],"poisoned":POISONED.load(Ordering::Acquire)})
}

pub(crate) fn decode_file(
    path: &Path,
    animated: bool,
    width: u32,
    height: u32,
    token: &CancellationToken,
) -> Result<Vec<DecodedFrame>, String> {
    let bytes = hover_decode::local_io::read_local_snapshot(path, MAX_INPUT, token)
        .map_err(|e| e.to_string())?;
    let report = exchange(
        Launch::Decoder,
        token,
        WALL_LIMIT,
        MEMORY_LIMIT,
        move |mut input, mut output| {
            input.write_all(REQUEST_MAGIC).map_err(|e| e.to_string())?;
            input
                .write_all(&[u8::from(animated)])
                .map_err(|e| e.to_string())?;
            input
                .write_all(&width.to_le_bytes())
                .map_err(|e| e.to_string())?;
            input
                .write_all(&height.to_le_bytes())
                .map_err(|e| e.to_string())?;
            input
                .write_all(&(bytes.len() as u64).to_le_bytes())
                .map_err(|e| e.to_string())?;
            input.write_all(&bytes).map_err(|e| e.to_string())?;
            drop(input);
            drop(bytes);
            read_response_limited(
                &mut output,
                width,
                height,
                if animated { MAX_FRAMES } else { 1 },
            )
        },
    );
    *LAST_REPORT.lock().unwrap() = Some(
        json!({"pid":report.pid,"outcome":report.reason,"peak_commit_bytes":report.peak_bytes,
        "user_cpu_ms":report.cpu_user_100ns/10000,"kernel_cpu_ms":report.cpu_kernel_100ns/10000,"wall_ms":report.elapsed.as_secs_f64()*1000.0,"active_after_stop":report.active_after_stop,"accounting_available":report.active_after_stop.is_some(),"permission_token_verified":report.permission_verified}),
    );
    report.result
}

fn read_u32(reader: &mut impl Read) -> Result<u32, String> {
    let mut v = [0; 4];
    reader.read_exact(&mut v).map_err(|e| e.to_string())?;
    Ok(u32::from_le_bytes(v))
}
fn read_u64(reader: &mut impl Read) -> Result<u64, String> {
    let mut v = [0; 8];
    reader.read_exact(&mut v).map_err(|e| e.to_string())?;
    Ok(u64::from_le_bytes(v))
}
fn read_response_limited(
    reader: &mut impl Read,
    max_width: u32,
    max_height: u32,
    max_frames: u32,
) -> Result<Vec<DecodedFrame>, String> {
    let mut magic = [0; 8];
    reader
        .read_exact(&mut magic)
        .map_err(|e| format!("Decoder response ended: {e}"))?;
    if &magic != RESPONSE_MAGIC {
        return Err("Invalid decoder response identity".into());
    }
    let mut kind = [0];
    reader.read_exact(&mut kind).map_err(|e| e.to_string())?;
    if kind[0] == 1 {
        let count = read_u32(reader)?;
        if count > 4096 {
            return Err("Decoder error message exceeds budget".into());
        }
        let mut message = vec![0; count as usize];
        reader.read_exact(&mut message).map_err(|e| e.to_string())?;
        return Err(String::from_utf8_lossy(&message).into_owned());
    }
    if kind[0] != 0 {
        return Err("Invalid decoder response kind".into());
    }
    let count = read_u32(reader)?;
    if !(1..=max_frames.min(MAX_FRAMES)).contains(&count) {
        return Err("Invalid decoder frame count".into());
    }
    let mut total = 0u64;
    let mut frames = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let width = read_u32(reader)?;
        let height = read_u32(reader)?;
        let delay = read_u32(reader)?;
        if width == 0
            || height == 0
            || width > max_width.min(8192)
            || height > max_height.min(8192)
            || delay > 655350
        {
            return Err("Invalid decoder frame geometry/timing".into());
        }
        let bytes = u64::from(width) * u64::from(height) * 4;
        total = total
            .checked_add(bytes)
            .ok_or("Decoder byte count overflow")?;
        if total > MAX_OUTPUT {
            return Err("Decoder output exceeds 256 MiB".into());
        }
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(bytes as usize)
            .map_err(|_| "Could not allocate bounded decoder output")?;
        pixels.resize(bytes as usize, 0);
        reader.read_exact(&mut pixels).map_err(|e| e.to_string())?;
        frames.push(DecodedFrame {
            width,
            height,
            stride: width * 4,
            pixels_bgra: pixels,
            delay_ms: delay,
        });
    }
    let mut extra = [0];
    if reader.read(&mut extra).map_err(|e| e.to_string())? != 0 {
        return Err("Unexpected trailing decoder output".into());
    }
    Ok(frames)
}

pub(crate) fn worker_entry() -> i32 {
    unsafe {
        windows_sys::Win32::System::Diagnostics::Debug::SetErrorMode(0x8003);
    }
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    let decoded = (|| -> Result<Vec<DecodedFrame>, String> {
        crate::decoder_sandbox::verify_token(unsafe {
            windows_sys::Win32::System::Threading::GetCurrentProcess()
        })?;
        let mut magic = [0; 8];
        input.read_exact(&mut magic).map_err(|e| e.to_string())?;
        if &magic != REQUEST_MAGIC {
            return Err("Invalid decoder request".into());
        }
        let mut kind = [0];
        input.read_exact(&mut kind).map_err(|e| e.to_string())?;
        let width = read_u32(&mut input)?;
        let height = read_u32(&mut input)?;
        let size = read_u64(&mut input)?;
        if kind[0] > 1
            || !(1..=8192).contains(&width)
            || !(1..=8192).contains(&height)
            || size > MAX_INPUT
        {
            return Err("Decoder request exceeds budget".into());
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size as usize)
            .map_err(|_| "Cannot allocate source snapshot")?;
        bytes.resize(size as usize, 0);
        input.read_exact(&mut bytes).map_err(|e| e.to_string())?;
        let token = CancellationToken::new();
        if kind[0] == 1 {
            hover_decode::decode_gif_snapshot(
                &bytes,
                width,
                height,
                DecodeLimits::default(),
                &token,
            )
            .map(|a| a.frames)
            .map_err(|e| e.to_string())
        } else {
            hover_decode::wic::decode_snapshot(
                &bytes,
                width,
                height,
                DecodeLimits::default(),
                &token,
            )
            .map(|image| {
                vec![DecodedFrame {
                    width: image.width,
                    height: image.height,
                    stride: image.stride,
                    pixels_bgra: image.pixels_bgra,
                    delay_ms: 0,
                }]
            })
            .map_err(|e| e.to_string())
        }
    })();
    let written = (|| -> std::io::Result<()> {
        output.write_all(RESPONSE_MAGIC)?;
        match decoded {
            Ok(frames) => {
                output.write_all(&[0])?;
                output.write_all(&(frames.len() as u32).to_le_bytes())?;
                for frame in frames {
                    output.write_all(&frame.width.to_le_bytes())?;
                    output.write_all(&frame.height.to_le_bytes())?;
                    output.write_all(&frame.delay_ms.to_le_bytes())?;
                    output.write_all(&frame.pixels_bgra)?;
                }
            }
            Err(error) => {
                let message = error.as_bytes();
                let length = message.len().min(4096);
                output.write_all(&[1])?;
                output.write_all(&(length as u32).to_le_bytes())?;
                output.write_all(&message[..length])?;
            }
        }
        output.flush()
    })();
    if written.is_ok() { 0 } else { 2 }
}

#[cfg(test)]
#[path = "decoder_process/tests.rs"]
mod tests;

#[cfg(test)]
pub(crate) fn permission_probe(request: &Value) -> Result<Value, String> {
    use std::io::Read;
    let unlisted = request["unlisted_handle"]
        .as_u64()
        .and_then(|h| usize::try_from(h).ok())
        .ok_or("Invalid test handle")?;
    let request = serde_json::to_vec(request).map_err(|e| e.to_string())?;
    let report = exchange(
        Launch::PermissionProbe(unlisted),
        &CancellationToken::new(),
        Duration::from_secs(8),
        64 * 1024 * 1024,
        move |mut input, mut output| {
            input.write_all(&request).map_err(|e| e.to_string())?;
            drop(input);
            use std::io::BufRead;
            let mut bytes = Vec::new();
            let mut reader = std::io::BufReader::new(&mut output);
            while bytes.len() < 65537 {
                let mut line = Vec::new();
                let count = Read::take(&mut reader, (65537 - bytes.len()) as u64)
                    .read_until(b'\n', &mut line)
                    .map_err(|e| e.to_string())?;
                if count == 0 {
                    break;
                }
                if let Some(stage) = String::from_utf8_lossy(&line)
                    .split_once("HOVER_PROBE_STAGE=")
                    .map(|(_, s)| s.trim().to_owned())
                {
                    eprintln!("Native permission probe stage: {stage}");
                }
                bytes.extend(line);
            }
            if bytes.len() > 65536 {
                return Err("Permission probe exceeded output budget".into());
            }
            let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
            let result = text
                .split_once("HOVER_PERMISSION_RESULT=")
                .ok_or_else(|| {
                    format!(
                        "Permission probe did not produce its marker: {}",
                        text.chars().take(4096).collect::<String>()
                    )
                })?
                .1
                .lines()
                .next()
                .ok_or("Missing permission results")?;
            serde_json::from_str::<Value>(result).map_err(|e| e.to_string())
        },
    );
    assert!(
        report.permission_verified,
        "Parent never verified the native token: {:?}",
        report.result
    );
    assert_eq!(report.reason, "success", "{:?}", report.result);
    assert_eq!(report.active_after_stop, Some(0));
    assert!(!POISONED.load(Ordering::Acquire));
    report.result.map(|mut value| {
        value["private_handle_inherited"] = json!(false);
        value
    })
}
