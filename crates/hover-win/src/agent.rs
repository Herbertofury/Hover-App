//! Private, bounded stdio control of an owned real Hover process. No listener,
//! arbitrary executable, script evaluation or external-window input injection.
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};

pub(crate) const WM_AGENT_REQUEST: u32 = WM_APP + 20;
const MAX_REQUEST_BYTES: usize = 16 * 1024;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
static SESSION_ACTIVE: AtomicBool = AtomicBool::new(false);

pub(crate) fn is_session() -> bool {
    SESSION_ACTIVE.load(Ordering::Acquire)
}

pub(crate) struct Request {
    pub command: Value,
    pub reply: SyncSender<Result<Value, String>>,
}

pub(crate) struct Launch {
    pub receiver: Receiver<Request>,
    pub ready: SyncSender<Result<isize, i32>>,
}

struct Session {
    sender: SyncSender<Request>,
    hwnd: isize,
}

impl Session {
    fn call(&self, command: Value) -> Result<Value, String> {
        let (reply, receive) = mpsc::sync_channel(1);
        self.sender
            .try_send(Request { command, reply })
            .map_err(|_| "Hover control queue is full or closed".to_owned())?;
        if unsafe { PostMessageW(self.hwnd as HWND, WM_AGENT_REQUEST, 0, 0) } == 0 {
            return Err("Hover's UI thread is unavailable".into());
        }
        receive
            .recv_timeout(COMMAND_TIMEOUT)
            .map_err(|_| "Hover did not respond within 10 seconds".to_owned())?
    }

    fn execute(&self, command: Value) -> Result<Value, String> {
        if command.get("command").and_then(Value::as_str) != Some("wait") {
            return self.call(command);
        }
        validate_fields(&command, &["command", "timeout_ms"])?;
        let timeout = match command.get("timeout_ms") {
            None => 10_000,
            Some(value) => value
                .as_u64()
                .filter(|n| (1..=30_000).contains(n))
                .ok_or("timeout_ms must be an integer between 1 and 30000")?,
        };
        let started = Instant::now();
        loop {
            let state = self.call(json!({"command":"status"}))?;
            if state["loading"] == false {
                return Ok(state);
            }
            if started.elapsed() >= Duration::from_millis(timeout) {
                return Err(
                    "Preview is still loading; wait timed out without cancelling it".into(),
                );
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

pub(crate) fn validate_fields(command: &Value, allowed: &[&str]) -> Result<(), String> {
    let object = command.as_object().ok_or("Command must be a JSON object")?;
    if let Some(key) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(format!("Unknown field: {key}"));
    }
    Ok(())
}

pub(crate) fn serve() -> i32 {
    SESSION_ACTIVE.store(true, Ordering::Release);
    let (sender, receiver) = mpsc::sync_channel(8);
    let (ready, started) = mpsc::sync_channel(1);
    let ui = std::thread::spawn(move || crate::platform::run_agent(Launch { receiver, ready }));
    let hwnd = match started.recv_timeout(COMMAND_TIMEOUT) {
        Ok(Ok(hwnd)) => hwnd,
        result => {
            eprintln!("Hover agent UI failed to start: {result:?}");
            return 2;
        }
    };
    let session = Session { sender, hwnd };
    let stdin = std::io::stdin();
    let mut input = BufReader::new(stdin.lock());
    let mut output = std::io::stdout().lock();
    let mut exit_code = 0;
    let mut quit_sent = false;
    loop {
        let mut line = Vec::new();
        let count =
            Read::take(&mut input, (MAX_REQUEST_BYTES + 1) as u64).read_until(b'\n', &mut line);
        match count {
            Ok(0) => break,
            Err(error) => {
                eprintln!("Agent input failed: {error}");
                exit_code = 2;
                break;
            }
            _ => {}
        }
        if line.len() > MAX_REQUEST_BYTES {
            let _ = writeln!(
                output,
                "{}",
                json!({"protocol":1,"id":null,"ok":false,"error":"Command exceeds 16 KiB; session stopped"})
            );
            exit_code = 2;
            break;
        }
        let (id, command) = match serde_json::from_slice::<Value>(&line) {
            Ok(Value::Object(mut object)) => (
                object.remove("id").unwrap_or(Value::Null),
                Value::Object(object),
            ),
            Ok(_) => {
                if writeln!(output, "{}", json!({"protocol":1,"id":null,"ok":false,"error":"Command must be a JSON object"})).and_then(|_| output.flush()).is_err() { break; }
                continue;
            }
            Err(error) => {
                if writeln!(output, "{}", json!({"protocol":1,"id":null,"ok":false,"error":format!("Invalid JSON: {error}")})).and_then(|_| output.flush()).is_err() { break; }
                continue;
            }
        };
        let quit = command.get("command").and_then(Value::as_str) == Some("quit");
        let response = match session.execute(command) {
            Ok(result) => json!({"protocol":1,"id":id,"ok":true,"result":result}),
            Err(error) => json!({"protocol":1,"id":id,"ok":false,"error":error}),
        };
        quit_sent = quit && response["ok"] == true;
        if writeln!(output, "{response}")
            .and_then(|_| output.flush())
            .is_err()
            || (quit && response["ok"] == true)
        {
            break;
        }
    }
    if !quit_sent {
        let _ = session.call(json!({"command":"quit"}));
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    while !ui.is_finished() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    if !ui.is_finished() {
        eprintln!("Owned Hover UI did not stop within 3 seconds");
        return 2;
    }
    match ui.join() {
        Ok(Ok(())) => exit_code,
        result => {
            eprintln!("Hover UI failed: {result:?}");
            2
        }
    }
}
