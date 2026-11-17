use std::{collections::HashMap, env, fs, io, net::SocketAddr, process::ExitCode};

use voidgate_backend::{app, serve, AppState};

const SETTINGS: [&str; 4] = [
    "VOIDGATE_API_KEY",
    "VOIDGATE_DECOY_PATHS",
    "VOIDGATE_BIND",
    "VOIDGATE_DEV_DOCS",
];

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Local convenience only: a deployed instance has no `.env` and is
    // configured through the process environment, which always wins.
    let file = match fs::read_to_string(".env") {
        // UTF-16 text without a byte-order mark is valid UTF-8 full of NULs.
        Ok(text) if text.contains('\0') => return Err("cannot read .env; save it as UTF-8".into()),
        Ok(text) => parse_dotenv(&text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => HashMap::new(),
        // An unreadable file must stop the start: silently skipping it would
        // run the service without the decoys it configures.
        Err(error) => return Err(format!("cannot read .env ({error}); save it as UTF-8").into()),
    };
    let from_environment = env::vars_os().map(|(name, _)| name.to_string_lossy().into_owned());
    if let Some(name) = unknown_setting(file.keys().cloned().chain(from_environment)) {
        return Err(format!("unknown setting {name}; known: {}", SETTINGS.join(", ")).into());
    }
    let setting = |name: &'static str| setting(&file, name);

    let api_key = setting("VOIDGATE_API_KEY")?.ok_or("VOIDGATE_API_KEY is not set")?;
    let paths = setting("VOIDGATE_DECOY_PATHS")?
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect();
    let state = AppState::new(api_key, paths)?;

    let address: SocketAddr = setting("VOIDGATE_BIND")?
        .unwrap_or_else(|| "127.0.0.1:8080".into())
        .parse()
        .map_err(|_| "VOIDGATE_BIND must be an address such as 127.0.0.1:8080")?;
    let dev_docs = match setting("VOIDGATE_DEV_DOCS")?.as_deref() {
        None | Some("" | "0") => false,
        Some("1") => true,
        Some(_) => return Err("VOIDGATE_DEV_DOCS must be 0 or 1".into()),
    };

    #[allow(unused_mut)]
    let mut router = app(state);
    if dev_docs {
        #[cfg(not(debug_assertions))]
        return Err(
            "VOIDGATE_DEV_DOCS=1 is only available in development builds, not in --release".into(),
        );
        #[cfg(debug_assertions)]
        {
            // The docs describe the whole API; never serve them beyond this machine.
            if !address.ip().is_loopback() {
                return Err("VOIDGATE_DEV_DOCS=1 requires a loopback VOIDGATE_BIND".into());
            }
            router = router.merge(voidgate_backend::dev_docs());
        }
    }

    let listener = tokio::net::TcpListener::bind(address)
        .await
        .map_err(|error| format!("cannot listen on {address}: {error}"))?;
    eprintln!("voidgate-backend listening on http://{address}");
    if dev_docs {
        eprintln!("dev docs: http://{address}/docs");
    }
    serve(listener, router).await?;
    Ok(())
}

/// Process environment first, then the `.env` file.
fn setting(file: &HashMap<String, String>, name: &'static str) -> Result<Option<String>, String> {
    match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(file.get(name).cloned()),
        // The error value carries the raw bytes; never print them (it may be the key).
        Err(env::VarError::NotUnicode(_)) => Err(format!("{name} is not valid Unicode")),
    }
}

/// A misspelt name (`VOIDGATE_DECOY_PATH`) would otherwise be ignored and the
/// service would start without what it was meant to configure.
fn unknown_setting(names: impl Iterator<Item = String>) -> Option<String> {
    names
        .filter(|name| name.starts_with("VOIDGATE_") && !SETTINGS.contains(&name.as_str()))
        .min()
}

// shortcut: plain KEY=VALUE lines only (no quotes, no `export`, no inline
// comments); switch to the `dotenvy` crate if the file ever needs more.
fn parse_dotenv(text: &str) -> HashMap<String, String> {
    // Windows editors and PowerShell 5.1 start UTF-8 files with a byte-order mark.
    text.trim_start_matches('\u{FEFF}')
        .lines()
        .filter_map(|line| line.trim().split_once('='))
        .filter(|(key, _)| !key.starts_with('#'))
        .map(|(key, value)| (key.trim().to_owned(), value.trim().to_owned()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{parse_dotenv, unknown_setting};

    #[test]
    fn parses_pairs_and_skips_comments_and_blank_lines() {
        let file = parse_dotenv("# note\n\nA=1\n B = /x,/y \n#C=3\nbroken\nD=a=b\n");
        assert_eq!(file.len(), 3);
        assert_eq!(file["A"], "1");
        assert_eq!(file["B"], "/x,/y");
        assert_eq!(file["D"], "a=b");
    }

    #[test]
    fn reads_windows_files_with_byte_order_mark_and_crlf() {
        let file = parse_dotenv("\u{FEFF}VOIDGATE_DECOY_PATHS=/internal/backup\r\nB=2\r\n");
        assert_eq!(file["VOIDGATE_DECOY_PATHS"], "/internal/backup");
        assert_eq!(file["B"], "2");
    }

    #[test]
    fn reports_misspelt_settings_and_ignores_unrelated_names() {
        let names = |list: &[&str]| unknown_setting(list.iter().map(|name| name.to_string()));
        assert_eq!(names(&["VOIDGATE_API_KEY", "PATH", "VOIDGATE_BIND"]), None);
        assert_eq!(
            names(&["VOIDGATE_API_KEY", "VOIDGATE_DECOY_PATH"]),
            Some("VOIDGATE_DECOY_PATH".into())
        );
        assert_eq!(
            names(&["export VOIDGATE_API_KEY", "HOME"]),
            None,
            "only names with the VOIDGATE_ prefix are checked"
        );
    }
}
