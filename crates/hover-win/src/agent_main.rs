#[cfg(windows)]
fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args == ["--help"] || args == ["-h"] {
        println!(
            "Usage: hover-agent --stdio\nSend one JSON command per line. Use capabilities to discover commands; quit or EOF closes only this session. See docs/agent-control.md."
        );
        return;
    }
    if args != ["--stdio"] {
        eprintln!(
            "Usage: hover-agent --stdio\nSend one JSON command per line; send quit or close stdin to stop this owned Hover session."
        );
        std::process::exit(2);
    }
    std::process::exit(hover_win::run_agent_stdio());
}

#[cfg(not(windows))]
fn main() {
    eprintln!("Hover is a native Windows app.");
    std::process::exit(2);
}
