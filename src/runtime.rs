//! Tokio lives on its own runtime, separate from gpui.
//!
//! gpui has its own executors but no IO reactor, so anything that talks to the
//! network (reqwest, axum, eventsource) runs on this runtime. UI code awaits
//! the result through [`spawn`] and then applies it on the gpui side.

use std::future::Future;
use std::pin::Pin;
use std::sync::OnceLock;
use std::task::{Context, Poll};
use std::time::Duration;

use tokio::task::{JoinError, JoinHandle};

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
static HTTP: OnceLock<reqwest::Client> = OnceLock::new();

pub fn runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("mailbox-io")
            .enable_all()
            .build()
            .expect("Failed to start tokio runtime")
    })
}

/// One shared HTTP client so connections (and TLS sessions) get reused
/// instead of doing a fresh handshake on every request.
pub fn http() -> &'static reqwest::Client {
    HTTP.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client")
    })
}

/// Client for long-lived streams (SSE). No overall timeout, or the stream
/// would be cut off after 30 seconds.
pub fn http_streaming() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to build streaming HTTP client")
}

/// Run a future on the tokio runtime. The returned handle aborts the tokio
/// task when dropped, so dropping the gpui `Task` that awaits it cancels the
/// network work too.
pub fn spawn<F>(future: F) -> AbortOnDrop<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    AbortOnDrop(runtime().spawn(future))
}

pub struct AbortOnDrop<T>(JoinHandle<T>);

impl<T> Drop for AbortOnDrop<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl<T> Future for AbortOnDrop<T> {
    type Output = Result<T, JoinError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.0).poll(cx)
    }
}
