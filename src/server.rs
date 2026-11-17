//! Connection handling with the limits `axum::serve` does not apply.
//!
//! `axum::serve` gives hyper no timer, so a client that opens a connection and
//! never finishes its request would hold it forever, without needing a key.
//! Once the head has arrived, `runtime::protect` bounds the decision itself.
//! This server also owns graceful shutdown: on the supplied signal it stops
//! accepting, asks every open connection to finish its in-flight request, and
//! only then returns.

use std::{future::Future, io, sync::Arc, time::Duration};

use axum::Router;
use hyper::server::conn::http1;
use hyper_util::{
    rt::{TokioIo, TokioTimer},
    service::TowerToHyperService,
};
use tokio::{
    net::TcpListener,
    sync::{watch, Semaphore},
    task::JoinSet,
};

/// Connections served at once; beyond this, new ones wait in the OS backlog.
const MAX_CONNECTIONS: usize = 1024;

/// Time a client has to send a complete request head. It also closes
/// keep-alive connections that stay idle this long.
const HEADER_READ_TIMEOUT: Duration = Duration::from_secs(30);

/// Time in-flight connections get to finish after a shutdown signal.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(10);

pub async fn serve<F>(listener: TcpListener, router: Router, shutdown: F) -> io::Result<()>
where
    F: Future<Output = ()> + Send,
{
    serve_with(listener, router, HEADER_READ_TIMEOUT, shutdown).await
}

/// Everything runs until the shutdown signal fires; then no new connection is
/// accepted and open ones are allowed to complete their current request.
async fn serve_with<F>(
    listener: TcpListener,
    router: Router,
    header_read_timeout: Duration,
    shutdown: F,
) -> io::Result<()>
where
    F: Future<Output = ()> + Send,
{
    let mut http = http1::Builder::new();
    http.timer(TokioTimer::new())
        .header_read_timeout(header_read_timeout);
    let slots = Arc::new(Semaphore::new(MAX_CONNECTIONS));
    let (shutdown_tx, shutdown_rx) = watch::channel(());
    let mut connections = JoinSet::new();
    let mut shutdown = std::pin::pin!(shutdown);

    loop {
        tokio::select! {
            // Shutdown wins over accepting a queued connection.
            _ = &mut shutdown => break,
            accepted = async {
                let Ok(slot) = slots.clone().acquire_owned().await else {
                    return None;
                };
                Some((slot, listener.accept().await))
            } => {
                let Some((slot, accepted)) = accepted else { break };
                let stream = match accepted {
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
                let mut graceful = shutdown_rx.clone();
                connections.spawn(async move {
                    let mut connection = std::pin::pin!(connection);
                    tokio::select! {
                        // A failed connection only affects its own client.
                        _ = connection.as_mut() => {}
                        // Sender dropped also means shutdown: close the connection
                        // after answering whatever request is in flight.
                        _ = graceful.changed() => {
                            connection.as_mut().graceful_shutdown();
                            let _ = connection.await;
                        }
                    }
                    drop(slot);
                });
            }
            // Reap finished connections so the set does not grow forever.
            _ = connections.join_next(), if !connections.is_empty() => {}
        }
    }

    // Tell open connections to stop after their current request.
    let _ = shutdown_tx.send(());
    let drained = async { while connections.join_next().await.is_some() {} };
    if tokio::time::timeout(SHUTDOWN_GRACE, drained).await.is_err() {
        connections.abort_all();
        while connections.join_next().await.is_some() {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{future::pending, time::Duration};

    use axum::{routing::get, Router};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
        sync::oneshot,
    };

    use super::serve_with;
    use crate::{app, AppState};

    async fn start(header_read_timeout: Duration) -> std::net::SocketAddr {
        start_with(listener().await, header_read_timeout).await
    }

    async fn listener() -> TcpListener {
        TcpListener::bind("127.0.0.1:0").await.expect("bind")
    }

    async fn start_with(
        listener: TcpListener,
        header_read_timeout: Duration,
    ) -> std::net::SocketAddr {
        let address = listener.local_addr().expect("address");
        let router = app(AppState::new("a".repeat(32), vec![]).expect("valid test config"));
        tokio::spawn(serve_with(
            listener,
            router,
            header_read_timeout,
            pending::<()>(),
        ));
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

    #[tokio::test]
    async fn shutdown_lets_an_in_flight_request_finish() {
        let listener = listener().await;
        let address = listener.local_addr().expect("address");
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let router = Router::new().route(
            "/slow",
            get(|| async {
                tokio::time::sleep(Duration::from_millis(300)).await;
                "finished"
            }),
        );
        let server = tokio::spawn(serve_with(
            listener,
            router,
            Duration::from_secs(5),
            async {
                let _ = shutdown_rx.await;
            },
        ));

        let mut stream = TcpStream::connect(address).await.expect("connect");
        stream
            .write_all(b"GET /slow HTTP/1.1\r\nHost: x\r\nConnection: keep-alive\r\n\r\n")
            .await
            .expect("write");
        // Give the request time to reach the handler, then ask for shutdown.
        tokio::time::sleep(Duration::from_millis(50)).await;
        shutdown_tx.send(()).expect("signal shutdown");

        let response = read_all(&mut stream).await;
        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        assert!(response.ends_with("finished"), "{response}");
        assert!(response.contains("connection: close"), "{response}");

        tokio::time::timeout(Duration::from_secs(5), server)
            .await
            .expect("server returns after draining")
            .expect("server task")
            .expect("server result");
    }

    #[tokio::test]
    async fn shutdown_closes_an_idle_keep_alive_connection() {
        let listener = listener().await;
        let address = listener.local_addr().expect("address");
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let router = app(AppState::new("a".repeat(32), vec![]).expect("valid test config"));
        let server = tokio::spawn(serve_with(
            listener,
            router,
            Duration::from_secs(5),
            async {
                let _ = shutdown_rx.await;
            },
        ));

        let mut stream = TcpStream::connect(address).await.expect("connect");
        stream
            .write_all(b"GET /health HTTP/1.1\r\nHost: x\r\nConnection: keep-alive\r\n\r\n")
            .await
            .expect("write");
        let mut buffer = [0_u8; 4096];
        let read = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buffer))
            .await
            .expect("response")
            .expect("read");
        assert!(String::from_utf8_lossy(&buffer[..read]).starts_with("HTTP/1.1 200"));

        shutdown_tx.send(()).expect("signal shutdown");
        // The idle keep-alive connection must not delay shutdown.
        read_all(&mut stream).await;
        tokio::time::timeout(Duration::from_secs(5), server)
            .await
            .expect("server returns after draining")
            .expect("server task")
            .expect("server result");
    }
}
