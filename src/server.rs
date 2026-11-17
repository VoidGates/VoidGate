//! Connection handling with the limits `axum::serve` does not apply.
//!
//! `axum::serve` gives hyper no timer, so a client that opens a connection and
//! never finishes its request would hold it forever, without needing a key.

use std::{io, sync::Arc, time::Duration};

use axum::{extract::Request, middleware::Next, response::Response, Router};
use hyper::server::conn::http1;
use hyper_util::{
    rt::{TokioIo, TokioTimer},
    service::TowerToHyperService,
};
use tokio::{net::TcpListener, sync::Semaphore};

use crate::error::AppError;

/// Connections served at once; beyond this, new ones wait in the OS backlog.
const MAX_CONNECTIONS: usize = 1024;

/// Time a client has to send a complete request head. It also closes
/// keep-alive connections that stay idle this long.
const HEADER_READ_TIMEOUT: Duration = Duration::from_secs(30);

/// Time a request has to be read and answered once its head has arrived.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

pub async fn serve(listener: TcpListener, router: Router) -> io::Result<()> {
    serve_with(listener, router, HEADER_READ_TIMEOUT).await
}

async fn serve_with(
    listener: TcpListener,
    router: Router,
    header_read_timeout: Duration,
) -> io::Result<()> {
    let mut http = http1::Builder::new();
    http.timer(TokioTimer::new())
        .header_read_timeout(header_read_timeout);
    let slots = Arc::new(Semaphore::new(MAX_CONNECTIONS));
    loop {
        // The semaphore is never closed, so this only fails if that changes.
        let Ok(slot) = slots.clone().acquire_owned().await else {
            return Ok(());
        };
        let stream = match listener.accept().await {
            Ok((stream, _)) => stream,
            // Usually "too many open files": back off instead of spinning.
            Err(_) => {
                tokio::time::sleep(Duration::from_millis(50)).await;
                continue;
            }
        };
        let connection = http.serve_connection(
            TokioIo::new(stream),
            TowerToHyperService::new(router.clone()),
        );
        tokio::spawn(async move {
            // A failed connection only affects its own client.
            let _ = connection.await;
            drop(slot);
        });
    }
}

/// Middleware: bounds how long one request may take, body upload included.
pub(crate) async fn request_timeout(request: Request, next: Next) -> Result<Response, AppError> {
    tokio::time::timeout(REQUEST_TIMEOUT, next.run(request))
        .await
        .map_err(|_| AppError::RequestTimeout)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
    };

    use super::serve_with;
    use crate::{app, AppState};

    async fn start(header_read_timeout: Duration) -> std::net::SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        let router = app(AppState::new("a".repeat(32), vec![]).expect("valid test config"));
        tokio::spawn(serve_with(listener, router, header_read_timeout));
        address
    }

    async fn read_all(stream: &mut TcpStream) -> String {
        let mut response = Vec::new();
        tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut response))
            .await
            .expect("server should close the connection")
            .expect("read");
        String::from_utf8_lossy(&response).into_owned()
    }

    #[tokio::test]
    async fn serves_a_complete_request() {
        let address = start(Duration::from_secs(5)).await;
        let mut stream = TcpStream::connect(address).await.expect("connect");
        stream
            .write_all(b"GET /health HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n")
            .await
            .expect("write");
        let response = read_all(&mut stream).await;
        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        assert!(response.ends_with(r#"{"status":"ok"}"#), "{response}");
    }

    #[tokio::test]
    async fn closes_a_connection_that_never_finishes_its_headers() {
        let address = start(Duration::from_millis(200)).await;
        let mut stream = TcpStream::connect(address).await.expect("connect");
        stream
            .write_all(b"POST /v1/decisions/http HTTP/1.1\r\nHost: x\r\n")
            .await
            .expect("write");
        // Without the timer this read would hang until the test's own timeout.
        read_all(&mut stream).await;
    }

    #[tokio::test]
    async fn closes_an_idle_connection() {
        let address = start(Duration::from_millis(200)).await;
        let mut stream = TcpStream::connect(address).await.expect("connect");
        read_all(&mut stream).await;
    }
}
