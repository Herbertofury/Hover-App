use hover_browser_host::{NativeMessage, read_native_message, write_native_message};
use std::io::{self, Write};

fn main() {
    while let Ok(Some(message)) = read_native_message(io::stdin()) {
        let response = match message {
            NativeMessage::Hover { url } => {
                if hover_browser_host::validate_remote_url(&url).is_ok() {
                    "{\"ok\":true,\"accepted\":true}"
                } else {
                    "{\"ok\":false,\"accepted\":false}"
                }
            }
            NativeMessage::Exit => break,
            NativeMessage::Unknown => "{\"ok\":false,\"accepted\":false}",
        };
        let _ = write_native_message(io::stdout(), response.as_bytes());
        let _ = io::stdout().flush();
    }
}
