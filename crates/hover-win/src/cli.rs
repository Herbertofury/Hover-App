use serde_json::{Value, json};

pub const APP_USER_MODEL_ID: &str = "Herbertofury.Hover";

/// Build identity is an inspection aid, not a signature or release certificate.
pub fn build_info() -> Value {
    json!({"schemaVersion":1,"app":"Hover","cargoVersion":env!("CARGO_PKG_VERSION"),
        "packageVersion":option_env!("HOVER_PACKAGE_VERSION").unwrap_or(env!("CARGO_PKG_VERSION")),
        "sourceRevision":option_env!("HOVER_SOURCE_REVISION"),
        "architecture":std::env::consts::ARCH,"platform":std::env::consts::OS,
        "profile":if cfg!(debug_assertions){"debug"}else{"release"},
        "appUserModelId":APP_USER_MODEL_ID,"agentProtocol":1,"features":{"nativeComparison":cfg!(windows)},"releaseVerified":false})
}

pub fn inspect_arguments(args: &[std::ffi::OsString], console: bool) -> Option<i32> {
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        println!(
            "Hover native Windows app\nUsage: hover [--agent-stdio | --version | --version-json | --help]\n       hover-agent --stdio\n       hover-agent [--resident-status | --show | --shutdown]\n\nNo arguments starts Hover in the notification area. Lifecycle commands target only this user's Hover in the same installation directory. Private stdio owns a real native session; quit/EOF closes it. See docs/agent-control.md."
        );
        return Some(0);
    }
    if args.len() == 1 && args[0] == "--version-json" {
        println!("{}", build_info());
        return Some(0);
    }
    if args.len() == 1 && args[0] == "--version" {
        println!(
            "Hover {}",
            option_env!("HOVER_PACKAGE_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
        );
        return Some(0);
    }
    #[cfg(windows)]
    if args.len() == 1
        && let Some(action @ ("--resident-status" | "--show" | "--shutdown")) = args[0].to_str()
    {
        return Some(match crate::resident::command(action) {
            Ok(value) => {
                println!("{value}");
                0
            }
            Err(error) => {
                eprintln!("{error}");
                2
            }
        });
    }
    let permitted = if console {
        args.len() == 1 && (args[0] == "--stdio" || args[0] == "--decode-worker")
    } else {
        args.is_empty()
            || args.len() == 1 && (args[0] == "--agent-stdio" || args[0] == "--decode-worker")
    };
    if !permitted {
        eprintln!("Invalid Hover arguments. Use --help for supported commands.");
        return Some(2);
    }
    None
}
