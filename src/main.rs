use std::{env, net::SocketAddr};

use voidgate_backend::{app, AppState, RateLimitConfig};

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
    let rate_limit = match optional_env("VOIDGATE_RATE_LIMIT_PER_MINUTE")? {
        Some(value) => RateLimitConfig::per_minute(value.parse::<u32>()?)?,
        None => RateLimitConfig::default(),
    };
    let state = AppState::with_rate_limit(api_key, paths, rate_limit)?;

    let address: SocketAddr = optional_env("VOIDGATE_BIND")?
        .unwrap_or_else(|| "127.0.0.1:8080".into())
        .parse()?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(listener, app(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

#[cfg(unix)]
async fn shutdown_signal() {
    let mut terminate = match tokio::signal::unix::signal(
        tokio::signal::unix::SignalKind::terminate(),
    ) {
        Ok(signal) => signal,
        Err(error) => {
            eprintln!("failed to register SIGTERM handler: {error}");
            std::process::exit(1);
        }
    };
    tokio::select! {
        result = tokio::signal::ctrl_c() => {
            if let Err(error) = result {
                eprintln!("failed to receive Ctrl+C: {error}");
                std::process::exit(1);
            }
        }
        _ = terminate.recv() => {}
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        eprintln!("failed to receive Ctrl+C: {error}");
        std::process::exit(1);
    }
}

fn optional_env(name: &'static str) -> Result<Option<String>, env::VarError> {
    match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(error),
    }
}
