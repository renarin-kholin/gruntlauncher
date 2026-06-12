//! Implementations of Blitz's embedder traits.
//!
//! Blitz talks to its embedder through three trait objects configured on the
//! document: a [`ShellProvider`] (redraw requests, cursor, window title), a
//! [`NetProvider`] (resource fetching) and a [`NavigationProvider`] (link
//! clicks and form submissions).
//!
//! All three implementations here are passive: they record what the engine
//! asked for and let the widget pick the requests up during its normal
//! `update`/`draw` cycle. This is what keeps the integration non-blocking —
//! background fetch threads only ever touch atomics and mutex-guarded
//! queues, never the UI.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use blitz_traits::navigation::{NavigationOptions, NavigationProvider};
use blitz_traits::net::{NetHandler, NetProvider, NetWaker, Request};
use blitz_traits::shell::ShellProvider;
use cursor_icon::CursorIcon;

/// Signals emitted by the engine (often from background threads) that the
/// widget consumes on the UI thread.
#[derive(Default)]
pub(crate) struct ShellFlags {
    redraw_requested: AtomicBool,
    /// Set only when the document's content or styles actually changed
    /// (e.g. a network resource finished loading). This is distinct from
    /// `redraw_requested` which is also set for hover/scroll changes that
    /// only need a re-rasterize, not a full style+layout resolve.
    document_changed: AtomicBool,
    cursor: Mutex<CursorIcon>,
    title: Mutex<Option<String>>,
}

impl ShellFlags {
    /// Returns whether a redraw was requested since the last call, and
    /// clears the flag.
    pub fn take_redraw_request(&self) -> bool {
        self.redraw_requested.swap(false, Ordering::AcqRel)
    }

    /// Returns whether the document content changed since the last call,
    /// and clears the flag.
    pub fn take_document_changed(&self) -> bool {
        self.document_changed.swap(false, Ordering::AcqRel)
    }

    /// Signal that the document content has changed (not just scroll/hover).
    /// This triggers a full style+layout resolve on the next frame.
    pub fn notify_document_changed(&self) {
        self.document_changed.store(true, Ordering::Release);
        self.redraw_requested.store(true, Ordering::Release);
    }

    pub fn title(&self) -> Option<String> {
        self.title.lock().unwrap().clone()
    }
}

impl ShellProvider for ShellFlags {
    fn request_redraw(&self) {
        self.redraw_requested.store(true, Ordering::Release);
    }

    fn set_cursor(&self, icon: CursorIcon) {
        *self.cursor.lock().unwrap() = icon;
    }

    fn set_window_title(&self, title: String) {
        *self.title.lock().unwrap() = Some(title);
    }
}

/// A [`NetProvider`] that delegates to [`blitz_net::Provider`] (HTTP(S),
/// `file://` and `data:` URLs) while guaranteeing the fetch is spawned on the
/// crate's private tokio runtime, regardless of the calling context.
pub(crate) struct Fetcher {
    inner: blitz_net::Provider,
}

impl Fetcher {
    pub fn new(shell: Arc<ShellFlags>) -> Self {
        // The waker runs on a network thread each time a resource finishes;
        // we signal both redraw and document_changed so the widget knows
        // a full style+layout resolve is needed (not just a re-rasterize).
        let waker: Arc<dyn NetWaker> = Arc::new(move |_doc_id: usize| {
            shell.notify_document_changed();
        });

        let _runtime = crate::runtime::handle().enter();
        Self {
            inner: blitz_net::Provider::new(Some(waker)),
        }
    }

    /// The number of in-flight fetches.
    pub fn in_flight(&self) -> usize {
        self.inner.count()
    }
}

impl NetProvider for Fetcher {
    fn fetch(&self, doc_id: usize, request: Request, handler: Box<dyn NetHandler>) {
        // Report completions under the URL that was *requested*, not the URL
        // reqwest landed on after redirects. blitz-dom 0.3.0-alpha.4 tracks
        // nodes waiting for an image in a map keyed by the requested URL but
        // looks them up by the URL the provider reports back — so any image
        // served via a redirect would silently never be applied.
        let handler = Box::new(PreserveRequestUrl {
            url: request.url.to_string(),
            inner: handler,
        });

        let _runtime = crate::runtime::handle().enter();
        self.inner.fetch(doc_id, request, handler);
    }
}

struct PreserveRequestUrl {
    url: String,
    inner: Box<dyn NetHandler>,
}

impl NetHandler for PreserveRequestUrl {
    fn bytes(self: Box<Self>, _resolved_url: String, bytes: blitz_traits::net::Bytes) {
        self.inner.bytes(self.url, bytes);
    }
}

/// Queues navigation requests (link clicks, form submissions) so the widget
/// can surface them to the application as messages.
///
/// The webview deliberately never navigates by itself: in the Elm
/// architecture the application owns all state transitions, so following a
/// link is the application's decision to make.
#[derive(Default)]
pub(crate) struct NavigationQueue {
    pending: Mutex<Vec<NavigationOptions>>,
}

impl NavigationQueue {
    pub fn drain(&self) -> Vec<NavigationOptions> {
        std::mem::take(&mut *self.pending.lock().unwrap())
    }
}

impl NavigationProvider for NavigationQueue {
    fn navigate_to(&self, options: NavigationOptions) {
        self.pending.lock().unwrap().push(options);
    }
}
