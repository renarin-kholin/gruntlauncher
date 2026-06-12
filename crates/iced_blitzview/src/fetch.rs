//! A convenience HTTP fetcher for loading pages by URL.
//!
//! This is deliberately a thin helper: production applications often have
//! their own HTTP stack (authentication, caching, retries) and only need
//! [`Content::load_html_with_base_url`](crate::Content::load_html_with_base_url)
//! — sub-resources are always fetched by the engine itself either way.

use std::sync::OnceLock;

/// A fetched HTML page.
#[derive(Debug, Clone)]
pub struct Page {
    /// The final URL of the page, after following any redirects.
    ///
    /// Use this — not the URL you requested — as the base URL when loading
    /// the page into a [`Content`](crate::Content), so relative resource
    /// paths resolve correctly.
    pub url: String,
    /// The body of the response.
    pub html: String,
}

/// An error returned by [`fetch_html`].
#[derive(Debug, Clone)]
pub enum Error {
    /// The URL could not be parsed.
    InvalidUrl(String),
    /// The request failed (connection, TLS, timeout, ...).
    Request(String),
    /// The server responded with a non-success status code.
    Status(u16),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidUrl(url) => write!(f, "invalid URL: {url}"),
            Error::Request(reason) => write!(f, "request failed: {reason}"),
            Error::Status(code) => write!(f, "server responded with status {code}"),
        }
    }
}

impl std::error::Error for Error {}

/// Fetches an HTML page over HTTP(S).
///
/// The request runs on the crate's private network runtime, so the returned
/// future can be awaited from any executor — pass it straight to
/// [`Task::perform`](https://docs.rs/iced/0.14/iced/struct.Task.html#method.perform):
///
/// ```no_run
/// # use iced::Task;
/// # use iced_blitzview::{fetch, fetch_html};
/// # #[derive(Debug, Clone)]
/// # enum Message { PageLoaded(Result<fetch::Page, fetch::Error>) }
/// let task: Task<Message> = Task::perform(
///     fetch_html("https://example.com"),
///     Message::PageLoaded,
/// );
/// ```
pub fn fetch_html(
    url: impl Into<String>,
) -> impl Future<Output = Result<Page, Error>> + Send + 'static {
    let url = url.into();

    let join = crate::runtime::handle().spawn(async move {
        let url: url::Url = url.parse().map_err(|_| Error::InvalidUrl(url))?;

        let response = client()
            .get(url)
            .send()
            .await
            .map_err(|error| Error::Request(error.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(Error::Status(status.as_u16()));
        }

        let final_url = response.url().to_string();
        let html = response
            .text()
            .await
            .map_err(|error| Error::Request(error.to_string()))?;

        Ok(Page {
            url: final_url,
            html,
        })
    });

    async move {
        join.await
            .unwrap_or_else(|error| Err(Error::Request(error.to_string())))
    }
}

fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent(concat!("iced_blitzview/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("iced_blitzview: failed to build HTTP client")
    })
}
