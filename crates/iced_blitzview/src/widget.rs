//! The [`WebView`] widget.

use iced::advanced::image;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{Tree, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::theme;
use iced::window;
use iced::{Element, Event, Length, Rectangle, Size, mouse};

use blitz_traits::events::MouseEventButton;
use blitz_traits::shell::ColorScheme;
use url::Url;

use crate::Content;
use crate::engine::LOADING_POLL_INTERVAL;

/// Creates a new [`WebView`] displaying the given [`Content`].
pub fn web_view<Message>(content: &Content) -> WebView<'_, Message> {
    WebView::new(content)
}

/// A widget that renders an HTML/CSS document.
///
/// The widget is a pure view over a [`Content`] owned by your application
/// state. It handles scrolling, hover styles and form control interaction
/// internally; semantically meaningful events — the user clicking a link or
/// submitting a form — are surfaced as messages via
/// [`on_navigate`](Self::on_navigate), leaving your `update` logic in charge
/// of what navigation means.
#[allow(missing_debug_implementations)]
pub struct WebView<'a, Message> {
    content: &'a Content,
    width: Length,
    height: Length,
    on_navigate: Option<Box<dyn Fn(Url) -> Message + 'a>>,
    on_load: Option<Message>,
}

impl<'a, Message> WebView<'a, Message> {
    /// Creates a new [`WebView`] displaying the given [`Content`].
    pub fn new(content: &'a Content) -> Self {
        Self {
            content,
            width: Length::Fill,
            height: Length::Fill,
            on_navigate: None,
            on_load: None,
        }
    }

    /// Sets the width of the [`WebView`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [`WebView`].
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the message produced when the user clicks a link or submits a
    /// form.
    ///
    /// The [`WebView`] never navigates on its own. Typically your `update`
    /// logic will fetch the URL (e.g. with [`fetch_html`](crate::fetch_html))
    /// and load the result into the [`Content`], or hand the URL to the
    /// system browser.
    ///
    /// If this is not set, link clicks are ignored.
    pub fn on_navigate(mut self, on_navigate: impl Fn(Url) -> Message + 'a) -> Self {
        self.on_navigate = Some(Box::new(on_navigate));
        self
    }

    /// Sets the message produced when the document finishes loading all of
    /// its resources (images, stylesheets, fonts).
    ///
    /// Useful to refresh widgets that display loading state derived from
    /// [`Content::is_loading`] or [`Content::title`], or to defer work until
    /// the page is fully rendered.
    pub fn on_load(mut self, on_load: Message) -> Self {
        self.on_load = Some(on_load);
        self
    }

    fn publish_navigations(
        &self,
        engine: &mut crate::engine::Engine,
        shell: &mut Shell<'_, Message>,
    ) {
        let navigations = engine.drain_navigations();

        let Some(on_navigate) = &self.on_navigate else {
            return;
        };

        for navigation in navigations {
            shell.publish(on_navigate(navigation.url));
        }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for WebView<'_, Message>
where
    Message: Clone,
    Theme: theme::Base,
    Renderer: image::Renderer<Handle = image::Handle>,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, self.width, self.height)
    }

    fn update(
        &mut self,
        _tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let mut engine = self.content.0.borrow_mut();

        match event {
            Event::Window(window::Event::RedrawRequested(now)) => {
                self.publish_navigations(&mut engine, shell);

                if engine.poll_load_completion()
                    && let Some(on_load) = &self.on_load
                {
                    shell.publish(on_load.clone());
                }

                // While fetches are in flight their completion only raises a
                // flag on a network thread; keep the event loop ticking at a
                // low frequency so the flag is picked up promptly.
                // (`absorb_signals` is folded in by `draw` right after this.)
                if engine.is_animating() {
                    engine.mark_dirty();
                    shell.request_redraw();
                } else if engine.is_loading() {
                    shell.request_redraw_at(*now + LOADING_POLL_INTERVAL);
                }

                return;
            }
            Event::Window(window::Event::Rescaled(scale_factor)) => {
                engine.set_scale_factor(*scale_factor);
            }
            Event::Keyboard(iced::keyboard::Event::ModifiersChanged(modifiers)) => {
                engine.track_modifiers(*modifiers);
            }
            Event::Mouse(mouse_event) => match mouse_event {
                mouse::Event::CursorMoved { .. } => {
                    if let Some(position) = cursor.position_in(bounds) {
                        engine.pointer_moved(position);
                    } else {
                        engine.pointer_left();
                    }
                }
                mouse::Event::CursorLeft => {
                    engine.pointer_left();
                }
                mouse::Event::ButtonPressed(button) | mouse::Event::ButtonReleased(button) => {
                    if let Some(position) = cursor.position_in(bounds)
                        && let Some(button) = mouse_button(*button)
                    {
                        let pressed = matches!(mouse_event, mouse::Event::ButtonPressed(_));

                        engine.pointer_button(position, button, pressed);
                        self.publish_navigations(&mut engine, shell);
                        shell.capture_event();
                    }
                }
                mouse::Event::WheelScrolled { delta } => {
                    if let Some(position) = cursor.position_in(bounds) {
                        engine.wheel(position, *delta);
                        shell.capture_event();
                    }
                }
                mouse::Event::CursorEntered => {}
            },
            _ => {}
        }

        // Interaction may have dirtied the document (hover styles, scroll,
        // form state). Repaint on the next frame if so.
        if engine.absorb_signals() {
            shell.request_redraw();
        }
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        let Some(clip_bounds) = bounds.intersection(viewport) else {
            return;
        };

        let mut engine = self.content.0.borrow_mut();

        // `prefers-color-scheme` follows the iced theme.
        engine.set_color_scheme(color_scheme(theme));
        engine.sync_viewport(bounds.size());
        engine.absorb_signals();

        if let Some(frame) = engine.frame(renderer) {
            renderer.draw_image(
                image::Image::new(frame)
                    .filter_method(image::FilterMethod::Linear)
                    .snap(true),
                bounds,
                clip_bounds,
            );
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if !cursor.is_over(layout.bounds()) {
            return mouse::Interaction::None;
        }

        self.content
            .0
            .borrow()
            .cursor_icon()
            .map(interaction)
            .unwrap_or(mouse::Interaction::None)
    }
}

impl<'a, Message, Theme, Renderer> From<WebView<'a, Message>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: theme::Base + 'a,
    Renderer: image::Renderer<Handle = image::Handle> + 'a,
{
    fn from(web_view: WebView<'a, Message>) -> Self {
        Self::new(web_view)
    }
}

/// Derives the CSS `prefers-color-scheme` from the iced theme's base
/// background luminance.
fn color_scheme(theme: &impl theme::Base) -> ColorScheme {
    let background = theme.base().background_color;

    // Relative luminance, sufficient for a light/dark decision.
    let luminance = 0.2126 * background.r + 0.7152 * background.g + 0.0722 * background.b;

    if luminance < 0.5 {
        ColorScheme::Dark
    } else {
        ColorScheme::Light
    }
}

fn mouse_button(button: mouse::Button) -> Option<MouseEventButton> {
    match button {
        mouse::Button::Left => Some(MouseEventButton::Main),
        mouse::Button::Right => Some(MouseEventButton::Secondary),
        mouse::Button::Middle => Some(MouseEventButton::Auxiliary),
        mouse::Button::Back => Some(MouseEventButton::Fourth),
        mouse::Button::Forward => Some(MouseEventButton::Fifth),
        mouse::Button::Other(_) => None,
    }
}

fn interaction(icon: cursor_icon::CursorIcon) -> mouse::Interaction {
    use cursor_icon::CursorIcon;

    match icon {
        CursorIcon::Default => mouse::Interaction::None,
        CursorIcon::Pointer => mouse::Interaction::Pointer,
        CursorIcon::Text | CursorIcon::VerticalText => mouse::Interaction::Text,
        CursorIcon::Crosshair => mouse::Interaction::Crosshair,
        CursorIcon::Grab => mouse::Interaction::Grab,
        CursorIcon::Grabbing => mouse::Interaction::Grabbing,
        CursorIcon::NotAllowed | CursorIcon::NoDrop => mouse::Interaction::NotAllowed,
        CursorIcon::ZoomIn => mouse::Interaction::ZoomIn,
        CursorIcon::ZoomOut => mouse::Interaction::ZoomOut,
        CursorIcon::EwResize | CursorIcon::ColResize => mouse::Interaction::ResizingHorizontally,
        CursorIcon::NsResize | CursorIcon::RowResize => mouse::Interaction::ResizingVertically,
        CursorIcon::Move => mouse::Interaction::Move,
        CursorIcon::AllScroll => mouse::Interaction::AllScroll,
        CursorIcon::Wait => mouse::Interaction::Wait,
        CursorIcon::Progress => mouse::Interaction::Progress,
        CursorIcon::Cell => mouse::Interaction::Cell,
        CursorIcon::Copy => mouse::Interaction::Copy,
        CursorIcon::Alias => mouse::Interaction::Alias,
        CursorIcon::ContextMenu => mouse::Interaction::ContextMenu,
        CursorIcon::Help => mouse::Interaction::Help,
        _ => mouse::Interaction::None,
    }
}
