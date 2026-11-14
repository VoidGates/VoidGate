use std::{env, net::SocketAddr};

use voidgate_backend::{app, AppState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("VOIDGATE_API_KEY")?;
    let paths = optional_env("VOIDGATE_DECOY_PATHS")?
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect();
    let state = AppState::new(api_key, paths)?;

    let address: SocketAddr = optional_env("VOIDGATE_BIND")?
        .unwrap_or_else(|| "127.0.0.1:8080".into())
        .parse()?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(listener, app(state)).await?;
    Ok(())
}

fn optional_env(name: &'static str) -> Result<Option<String>, env::VarError> {
    match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(error),
    }
}
