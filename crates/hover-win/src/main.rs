#![windows_subsystem = "windows"]

#[cfg(windows)]
fn main() {
    if std::env::args().skip(1).collect::<Vec<_>>() == ["--agent-stdio"] {
        std::process::exit(hover_win::run_agent_stdio());
    }
    if let Err(code) = hover_win::run() {
        std::process::exit(code);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("Hover is a Windows 11 native app.");
}
