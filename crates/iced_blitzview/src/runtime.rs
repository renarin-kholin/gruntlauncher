//! A private tokio runtime that drives all network I/O.
//!
//! Blitz's network provider (`blitz_net::Provider`) spawns its fetches with
//! `tokio::spawn`, which requires an ambient tokio runtime. Relying on the
//! host application's executor would force every `iced_blitzview` user to
//! enable iced's `tokio` feature *and* would break whenever a `Content` is
//! created outside an async context (e.g. in the application's `new`).
//!
//! Owning a small dedicated runtime keeps resource fetching fully off the UI
//! thread with zero requirements on the embedding application.

use std::sync::OnceLock;
use tokio::runtime::Runtime;

pub(crate) fn handle() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();

    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("iced-blitzview-io")
            .enable_all()
            .build()
            .expect("iced_blitzview: failed to start network runtime")
    })
}
