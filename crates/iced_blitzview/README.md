# iced_blitzview

An HTML/CSS view widget for [iced] 0.14, powered by DioxusLabs' [Blitz] engine.

Not an embedded browser — no JavaScript, no separate process — a *document
view*: real CSS layout (flexbox, grid, gradients, shadows, web fonts, SVG)
rendered natively inside your iced application. Ideal for help pages,
reports, e-mail/HTML previews, and simple browsing.

[iced]: https://github.com/iced-rs/iced
[Blitz]: https://github.com/DioxusLabs/blitz

## Quick start

```rust
use iced_blitzview::{Content, web_view};

struct App {
    page: Content,
}

#[derive(Debug, Clone)]
enum Message {
    LinkClicked(iced_blitzview::Url),
    PageLoaded,
}

impl App {
    fn new() -> Self {
        Self {
            page: Content::with_html("<h1>Hello!</h1>"),
        }
    }

    fn view(&self) -> iced::Element<'_, Message> {
        web_view(&self.page)
            .on_navigate(Message::LinkClicked)
            .on_load(Message::PageLoaded)
            .into()
    }
}
```

Run the bundled mini-browser to see it in action:

```sh
cargo run --example browser
```

## Design

The crate follows iced's Elm-flavored philosophy throughout:

- **State lives in your application.** A `Content` owns the document; the
  `WebView` widget borrows it in `view`, exactly like iced's own
  `text_editor::Content`.
- **Semantics become messages.** Link clicks and form submissions arrive
  through `on_navigate`; load completion through `on_load`. The widget never
  navigates by itself — following a link is your `update` logic's decision
  (load it with `fetch_html`, hand it to the system browser, ignore it).
- **Nothing blocks the UI.** Images, stylesheets and fonts are fetched on a
  private background tokio runtime; the view repaints incrementally as they
  arrive. No executor requirements are imposed on your application.
- **Presentation is handled internally.** Scrolling, hover styles, cursor
  icons and form-control state are document concerns and work out of the box.
- **The theme flows into the page.** CSS `prefers-color-scheme` follows the
  luminance of your iced theme, so pages with dark-mode support match your
  application automatically.

### Rendering

Documents are rasterized with [vello_cpu] (SIMD, multithreaded) and presented
through iced's image primitive, which works on both the wgpu and tiny-skia
backends. Repaints are damage-driven — an idle page costs nothing per frame.

GPU rendering via vello/wgpu is not currently possible: iced 0.14 uses wgpu
27 while vello 0.9 requires wgpu 29, so the two cannot share a device.

[vello_cpu]: https://github.com/linebender/vello

## Limitations

- No JavaScript.
- Keyboard input (text fields, focus traversal) is not yet forwarded to the
  document.
- One `WebView` per `Content` at a time (scroll/hover state lives in the
  document).
- Blitz is pinned to `0.3.0-alpha.4`; it is pre-1.0 and moving quickly.

## License

MIT or Apache-2.0, at your option.
