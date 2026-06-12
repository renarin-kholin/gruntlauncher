//! A minimal browser built on `iced_blitzview`.
//!
//! Demonstrates the full integration surface:
//! * `Content` owned by the application state, displayed with `web_view`
//! * link clicks surfaced as messages via `on_navigate`
//! * remote pages loaded with `fetch_html` through `Task::perform`
//! * images and stylesheets streaming in without blocking the UI
//!
//! Run with: `cargo run --example browser`

use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Length, Task};
use iced_blitzview::{Content, Url, fetch, fetch_html, web_view};

fn main() -> iced::Result {
    iced::application(Browser::new, Browser::update, Browser::view)
        .title(Browser::title)
        .run()
}

struct Browser {
    page: Content,
    address: String,
    status: Status,
}

enum Status {
    Idle,
    Fetching,
    Error(String),
}

#[derive(Debug, Clone)]
enum Message {
    AddressChanged(String),
    Submitted,
    LinkClicked(Url),
    PageFetched(Result<fetch::Page, fetch::Error>),
    PageLoaded,
}

impl Browser {
    fn new() -> Self {
        Self {
            page: Content::with_html(WELCOME_PAGE),
            address: String::new(),
            status: Status::Idle,
        }
    }

    fn title(&self) -> String {
        match self.page.title() {
            Some(title) => format!("{title} — blitzview"),
            None => String::from("blitzview"),
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AddressChanged(address) => {
                self.address = address;

                Task::none()
            }
            Message::Submitted => {
                let url = normalize(&self.address);
                self.address = url.clone();

                self.fetch(url)
            }
            Message::LinkClicked(url) => {
                self.address = url.to_string();

                self.fetch(url)
            }
            Message::PageFetched(Ok(page)) => {
                self.status = Status::Idle;
                self.address = page.url.clone();
                self.page.load_html_with_base_url(&page.html, page.url);

                Task::none()
            }
            Message::PageFetched(Err(error)) => {
                self.status = Status::Error(error.to_string());

                Task::none()
            }
            // The message itself carries no data — its arrival makes iced
            // re-run `view`, refreshing the title and the loading status.
            Message::PageLoaded => Task::none(),
        }
    }

    fn fetch(&mut self, url: impl Into<String>) -> Task<Message> {
        self.status = Status::Fetching;

        Task::perform(fetch_html(url.into()), Message::PageFetched)
    }

    fn view(&self) -> Element<'_, Message> {
        let address_bar = row![
            text_input("Enter a URL...", &self.address)
                .on_input(Message::AddressChanged)
                .on_submit(Message::Submitted)
                .padding(10),
            button(text("Go")).on_press(Message::Submitted).padding(10),
        ]
        .spacing(10);

        let status = match &self.status {
            Status::Idle if self.page.is_loading() => text("Loading resources..."),
            Status::Idle => text(""),
            Status::Fetching => text("Fetching page..."),
            Status::Error(error) => text(error.clone()),
        };

        let page = web_view(&self.page)
            .on_navigate(Message::LinkClicked)
            .on_load(Message::PageLoaded);

        column![
            address_bar,
            status.size(14),
            container(page).width(Length::Fill).height(Length::Fill),
        ]
        .spacing(10)
        .padding(10)
        .into()
    }
}

fn normalize(input: &str) -> String {
    let input = input.trim();

    if input.contains("://") {
        input.to_owned()
    } else {
        format!("https://{input}")
    }
}

const WELCOME_PAGE: &str = r#"
<!DOCTYPE html>
<html>
<head>
  <title>Welcome</title>
  <style>
    body {
      font-family: sans-serif;
      max-width: 680px;
      margin: 0 auto;
      padding: 24px;
      line-height: 1.6;
      color: #1a1a2e;
    }
    h1 {
      background: linear-gradient(90deg, #3b82f6, #8b5cf6);
      color: white;
      padding: 16px 24px;
      border-radius: 12px;
    }
    .card {
      border: 1px solid #ddd;
      border-radius: 12px;
      padding: 16px;
      margin: 12px 0;
      box-shadow: 0 2px 8px rgba(0, 0, 0, 0.06);
    }
    .card:hover { border-color: #8b5cf6; }
    img { border-radius: 8px; }
    a { color: #3b82f6; }
    code {
      background: #f1f5f9;
      padding: 2px 6px;
      border-radius: 4px;
      font-size: 0.9em;
    }
  </style>
</head>
<body>
  <h1>iced × blitz</h1>
  <p>
    This page is rendered by <code>iced_blitzview</code> — the
    <a href="https://github.com/DioxusLabs/blitz">Blitz</a> HTML/CSS engine
    embedded as an <a href="https://github.com/iced-rs/iced">iced</a> widget.
  </p>

  <div class="card">
    <h3>Asynchronous images</h3>
    <p>These images are fetched in the background while the UI stays responsive:</p>
    <img src="https://picsum.photos/id/29/300/180" width="300" height="180" alt="mountains">
    <img src="https://picsum.photos/id/42/300/180" width="300" height="180" alt="cafe">
  </div>

  <div class="card">
    <h3>Navigation as messages</h3>
    <p>
      Clicking a link doesn't navigate by itself — it sends a message to the
      application, which decides what to do. Try
      <a href="https://example.com">example.com</a> or
      <a href="https://en.wikipedia.org/wiki/Rust_(programming_language)">Wikipedia</a>.
    </p>
  </div>

  <div class="card">
    <h3>It's just CSS</h3>
    <p>
      Flexbox, grid, gradients, shadows, border-radius, web fonts and
      <em>hover styles</em> all work — hover over these cards.
    </p>
  </div>
</body>
</html>
"#;
