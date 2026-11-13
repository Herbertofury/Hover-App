#[cfg(windows)]
fn main() {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if let Some(code) = hover_win::cli::inspect_arguments(&args, true) {
        std::process::exit(code);
    }
    if args == ["--decode-worker"] {
        std::process::exit(hover_win::run_decoder_worker());
    }
    std::process::exit(hover_win::run_agent_stdio());
}

#[cfg(not(windows))]
fn main() {
    eprintln!("Hover is a native Windows app.");
    std::process::exit(2);
}
