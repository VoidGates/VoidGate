use std::{collections::HashMap, env, fs, net::SocketAddr};

use voidgate_backend::{app, dev_docs, AppState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Local convenience only: a deployed instance has no `.env` and is
    // configured through the process environment, which always wins.
    let file = parse_dotenv(&fs::read_to_string(".env").unwrap_or_default());
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
        .parse()?;
    let mut router = app(state);
    if setting("VOIDGATE_DEV_DOCS")?.as_deref() == Some("1") {
        // The docs describe the whole API; never serve them beyond this machine.
        if !address.ip().is_loopback() {
            return Err("VOIDGATE_DEV_DOCS=1 requires a loopback VOIDGATE_BIND".into());
        }
        router = router.merge(dev_docs());
        eprintln!("dev docs: http://{address}/docs");
    }
    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(listener, router).await?;
    Ok(())
}

/// Process environment first, then the `.env` file.
fn setting(
    file: &HashMap<String, String>,
    name: &'static str,
) -> Result<Option<String>, env::VarError> {
    match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(file.get(name).cloned()),
        Err(error) => Err(error),
    }
}

// shortcut: plain KEY=VALUE lines only (no quotes, no `export`, no inline
// comments); switch to the `dotenvy` crate if the file ever needs more.
fn parse_dotenv(text: &str) -> HashMap<String, String> {
    text.lines()
        .filter_map(|line| line.trim().split_once('='))
        .filter(|(key, _)| !key.starts_with('#'))
        .map(|(key, value)| (key.trim().to_owned(), value.trim().to_owned()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse_dotenv;

    #[test]
    fn parses_pairs_and_skips_comments_and_blank_lines() {
        let file = parse_dotenv("# note\n\nA=1\n B = /x,/y \n#C=3\nbroken\nD=a=b\n");
        assert_eq!(file.len(), 3);
        assert_eq!(file["A"], "1");
        assert_eq!(file["B"], "/x,/y");
        assert_eq!(file["D"], "a=b");
    }
}
