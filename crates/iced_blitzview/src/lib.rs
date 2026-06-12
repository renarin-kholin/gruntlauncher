//! An HTML/CSS view widget for [iced], powered by the [Blitz] engine.
//!
//! [iced]: https://github.com/iced-rs/iced
//! [Blitz]: https://github.com/DioxusLabs/blitz
//!
//! `iced_blitzview` renders HTML and CSS inside an iced application. It is
//! not an embedded browser — there is no JavaScript and no process
//! isolation — it is a *document view*: ideal for rendering rich content
//! (help pages, reports, e-mails, previews, simple browsing) with real CSS
//! layout, web fonts, SVG, and images.
//!
//! # Design
//!
//! The crate follows iced's Elm-flavored philosophy:
//!
//! * **State lives in your application.** A [`Content`] owns the document;
//!   the [`WebView`] widget borrows it in `view`, exactly like
//!   `text_editor::Content`.
//! * **Semantics become messages.** Link clicks and form submissions are
//!   surfaced through [`WebView::on_navigate`]; the widget never navigates
//!   by itself.
//! * **Nothing blocks the UI.** Images, stylesheets and fonts are fetched on
//!   a private background runtime and the view repaints incrementally as
//!   they arrive.
//! * **Presentation is handled internally.** Scrolling, hover styles, cursor
//!   changes and form-control state are document concerns; they work out of
//!   the box without plumbing messages through your `update`.
//!
//! Rendering uses [vello_cpu] and repaints only when the document actually
//! changes, so an idle page costs nothing per frame.
//!
//! [vello_cpu]: https://github.com/linebender/vello
//!
//! # Example
//!
//! ```no_run
//! use iced_blitzview::{Content, web_view};
//!
//! struct App {
//!     page: Content,
//! }
//!
//! #[derive(Debug, Clone)]
//! enum Message {
//!     LinkClicked(iced_blitzview::Url),
//! }
//!
//! impl App {
//!     fn view(&self) -> iced::Element<'_, Message> {
//!         web_view(&self.page)
//!             .on_navigate(Message::LinkClicked)
//!             .into()
//!     }
//! }
//! ```
//!
//! To load remote pages, see [`fetch_html`].
//!
//! # Limitations
//!
//! * No JavaScript.
//! * Keyboard input (text fields, focus traversal) is not yet forwarded to
//!   the document.
//! * One [`WebView`] per [`Content`] at a time.

mod content;
mod engine;
mod providers;
mod runtime;
mod widget;

pub mod fetch;

pub use content::Content;
pub use fetch::fetch_html;
pub use widget::{WebView, web_view};

pub use url::Url;
