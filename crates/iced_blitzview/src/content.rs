//! The state of a [`WebView`](crate::WebView) widget.

use std::cell::RefCell;

use crate::engine::Engine;

/// The HTML document displayed by a [`WebView`](crate::WebView).
///
/// A [`Content`] is owned by your application state — the widget only
/// borrows it, following the same pattern as
/// [`text_editor::Content`](https://docs.rs/iced/0.14/iced/widget/text_editor/struct.Content.html):
///
/// ```no_run
/// use iced_blitzview::{Content, web_view};
///
/// struct App {
///     page: Content,
/// }
///
/// #[derive(Debug, Clone)]
/// enum Message {}
///
/// impl App {
///     fn new() -> Self {
///         Self {
///             page: Content::with_html("<h1>Hello, iced!</h1>"),
///         }
///     }
///
///     fn view(&self) -> iced::Element<'_, Message> {
///         web_view(&self.page).into()
///     }
/// }
/// ```
///
/// Sub-resources referenced by the document (images, stylesheets, fonts) are
/// fetched automatically on background threads and the view repaints as they
/// arrive — loading never blocks the UI thread.
///
/// A [`Content`] should be displayed by at most one [`WebView`] at a time:
/// scroll position, hover state and viewport size live in the document, so
/// two widgets sharing one [`Content`] would fight over them.
pub struct Content(pub(crate) RefCell<Engine>);

impl Content {
    /// Creates an empty [`Content`] (a blank document).
    pub fn new() -> Self {
        Self::with_html("")
    }

    /// Creates a [`Content`] from an HTML string.
    ///
    /// Relative URLs in the document will not resolve; use
    /// [`with_html_and_base_url`](Self::with_html_and_base_url) if the
    /// document references resources by relative path.
    pub fn with_html(html: impl AsRef<str>) -> Self {
        Self(RefCell::new(Engine::new(html.as_ref(), None)))
    }

    /// Creates a [`Content`] from an HTML string, resolving relative URLs
    /// against `base_url`.
    pub fn with_html_and_base_url(html: impl AsRef<str>, base_url: impl Into<String>) -> Self {
        Self(RefCell::new(Engine::new(
            html.as_ref(),
            Some(base_url.into()),
        )))
    }

    /// Replaces the document with the given HTML.
    ///
    /// The viewport (size, zoom, scale factor) is preserved; the scroll
    /// position is reset to the top.
    pub fn load_html(&mut self, html: impl AsRef<str>) {
        self.0.get_mut().load_html(html.as_ref(), None);
    }

    /// Replaces the document with the given HTML, resolving relative URLs
    /// against `base_url`.
    pub fn load_html_with_base_url(&mut self, html: impl AsRef<str>, base_url: impl Into<String>) {
        self.0
            .get_mut()
            .load_html(html.as_ref(), Some(base_url.into()));
    }

    /// The document title, if it has one (`<title>`).
    pub fn title(&self) -> Option<String> {
        self.0.borrow().title()
    }

    /// Whether any sub-resources (images, stylesheets, fonts) are still
    /// being fetched.
    pub fn is_loading(&self) -> bool {
        self.0.borrow().is_loading()
    }

    /// The current zoom level (`1.0` is unzoomed).
    pub fn zoom(&self) -> f32 {
        self.0.borrow().zoom()
    }

    /// Sets the zoom level (`1.0` is unzoomed, clamped to a minimum of
    /// `0.1`).
    pub fn set_zoom(&mut self, zoom: f32) {
        self.0.get_mut().set_zoom(zoom);
    }
}

impl Default for Content {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Content {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Content")
            .field("title", &self.title())
            .field("is_loading", &self.is_loading())
            .finish_non_exhaustive()
    }
}
