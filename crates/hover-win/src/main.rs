#![windows_subsystem = "windows"]

#[cfg(windows)]
fn main() {
    if let Err(code) = hover_win::run() {
        std::process::exit(code);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("Hover is a Windows 11 native app.");
}
