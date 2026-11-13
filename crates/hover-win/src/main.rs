#![windows_subsystem = "windows"]

#[cfg(windows)]
fn main() {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if let Some(code) = hover_win::cli::inspect_arguments(&args, false) {
        std::process::exit(code);
    }
    if args == ["--decode-worker"] {
        std::process::exit(hover_win::run_decoder_worker());
    }
    if args == ["--agent-stdio"] {
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
