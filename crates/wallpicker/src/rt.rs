//! The async runtime (tokio): `block_on` for the
//! command line, `spawn` for `serve`'s requests. `spawn_blocking` runs on
//! the runtime it's called from, if any, else
//! on this one.

use std::future::Future;
use std::sync::OnceLock;

use tokio::runtime::{Handle, Runtime};
use tokio::task::JoinHandle;

fn runtime() -> &'static Runtime {
    static RT: OnceLock<Runtime> = OnceLock::new();
    RT.get_or_init(|| Runtime::new().expect("failed to start the async runtime"))
}

pub fn block_on<F: Future>(future: F) -> F::Output {
    runtime().block_on(future)
}

pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    runtime().spawn(future)
}

pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    match Handle::try_current() {
        Ok(handle) => handle.spawn_blocking(f),
        Err(_) => runtime().spawn_blocking(f),
    }
}
