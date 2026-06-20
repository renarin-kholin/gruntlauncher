//! The rendering and interaction engine behind a [`Content`](crate::Content).
//!
//! Owns the Blitz document, converts widget-space input into DOM events, and
//! rasterizes the document into an iced [`image::Handle`] with vello_cpu.
//!
//! Repaints are damage-driven: a frame is only rasterized when something
//! actually changed (document mutation, resource load, hover/active state,
//! scroll, viewport resize, CSS animation). Otherwise the cached frame is
//! reused, which makes the idle cost of the widget effectively zero.

use std::sync::Arc;
use std::time::Instant;

use blitz_dom::{Document as _, DocumentConfig};
use blitz_html::HtmlDocument;
use blitz_traits::events::{
    BlitzPointerEvent, BlitzPointerId, MouseEventButton, MouseEventButtons, PointerCoords,
    PointerDetails, UiEvent,
};
use blitz_traits::navigation::NavigationOptions;
use blitz_traits::net::NetProvider;
use blitz_traits::shell::{ColorScheme, ShellProvider, Viewport};

use anyrender::{ImageRenderer, PaintScene};
use anyrender_vello_cpu::VelloCpuImageRenderer;
use blitz_paint::paint_scene;

use iced::advanced::image;
use iced::{Point, Size};

/// How often to check for finished resource fetches while any are in flight.
///
/// Finished fetches raise a redraw flag from their network thread, but iced
/// only delivers events to widgets when its event loop wakes up — so while
/// fetches are pending the widget keeps itself scheduled at this interval.
/// 10Hz is imperceptible for content loading and costs nothing while idle
/// (the timer is only armed while `is_loading()` is true).
pub(crate) const LOADING_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);

pub(crate) struct Engine {
    doc: HtmlDocument,
    shell: Arc<crate::providers::ShellFlags>,
    net: Arc<crate::providers::Fetcher>,
    navigations: Arc<crate::providers::NavigationQueue>,

    renderer: VelloCpuImageRenderer,
    renderer_size: (u32, u32),
    frame: Option<image::Handle>,
    /// Keeps the current frame resident in the renderer's texture atlas.
    frame_allocation: Option<image::Allocation>,
    needs_repaint: bool,

    /// Whether the document's styles or layout have changed and a full
    /// `resolve()` (style + layout pass) is required before the next paint.
    /// This is distinct from `needs_repaint`: the latter only means "re-rasterize
    /// the current scene" (e.g. scroll offset or hover cursor changed), while
    /// this flag means "the CSS/style/layout data itself changed".
    pub(crate) needs_resolve: bool,

    /// Window scale factor, tracked from `window::Event::Rescaled`.
    scale_factor: f32,
    color_scheme: ColorScheme,

    /// Mouse buttons currently held, mirrored into every pointer event.
    buttons: MouseEventButtons,
    modifiers: keyboard_types::Modifiers,

    /// Whether resources were still in flight at the last poll, to detect
    /// the loading → loaded transition.
    was_loading: bool,

    /// Epoch for CSS animation clocks.
    started_at: Instant,
}

impl Engine {
    pub fn new(html: &str, base_url: Option<String>) -> Self {
        let shell = Arc::new(crate::providers::ShellFlags::default());
        let net = Arc::new(crate::providers::Fetcher::new(shell.clone()));
        let navigations = Arc::new(crate::providers::NavigationQueue::default());

        let doc = build_document(html, base_url, &shell, &net, &navigations);

        Self {
            doc,
            shell,
            net,
            navigations,
            renderer: VelloCpuImageRenderer::new(0, 0),
            renderer_size: (0, 0),
            frame: None,
            frame_allocation: None,
            needs_repaint: true,
            needs_resolve: true,
            scale_factor: 1.0,
            color_scheme: ColorScheme::Light,
            buttons: MouseEventButtons::None,
            modifiers: keyboard_types::Modifiers::default(),
            was_loading: false,
            started_at: Instant::now(),
        }
    }

    /// Replaces the document, preserving the viewport and the network /
    /// navigation / shell wiring (mirrors blitz-shell's `replace_document`).
    pub fn load_html(&mut self, html: &str, base_url: Option<String>) {
        let viewport = self.doc.viewport().clone();

        let mut doc = build_document(html, base_url, &self.shell, &self.net, &self.navigations);
        doc.set_viewport(viewport);

        self.doc = doc;
        self.needs_repaint = true;
        self.needs_resolve = true;
    }

    pub fn title(&self) -> Option<String> {
        self.doc
            .find_title_node()
            .map(|node| node.text_content())
            .filter(|title| !title.is_empty())
            .or_else(|| self.shell.title())
    }

    pub fn zoom(&self) -> f32 {
        self.doc.viewport().zoom()
    }

    pub fn set_zoom(&mut self, zoom: f32) {
        self.doc.zoom_to(zoom.max(0.1));
        self.needs_repaint = true;
        self.needs_resolve = true;
    }

    pub fn is_loading(&self) -> bool {
        self.net.in_flight() > 0 || self.doc.has_pending_critical_resources()
    }

    pub fn is_animating(&self) -> bool {
        self.doc.is_animating()
    }

    /// Returns `true` exactly once when the document transitions from
    /// loading resources to fully loaded.
    pub fn poll_load_completion(&mut self) -> bool {
        let loading = self.is_loading();
        let finished = self.was_loading && !loading;
        self.was_loading = loading;

        finished
    }

    pub fn set_scale_factor(&mut self, scale_factor: f32) {
        self.scale_factor = scale_factor;
    }

    pub fn set_color_scheme(&mut self, color_scheme: ColorScheme) {
        self.color_scheme = color_scheme;
    }

    pub fn mark_dirty(&mut self) {
        self.needs_repaint = true;
        self.needs_resolve = true;
    }

    /// Folds signals raised by the engine (possibly from network threads)
    /// into the repaint state. Returns whether a repaint is now pending.
    pub fn absorb_signals(&mut self) -> bool {
        if self.shell.take_redraw_request() {
            self.needs_repaint = true;
        }

        if self.shell.take_document_changed() {
            // Document content changed (e.g. resource finished loading) --
            // full style+layout resolve needed.
            self.needs_resolve = true;
        }

        if self.doc.is_animating() {
            self.needs_repaint = true;
            self.needs_resolve = true;
        }

        self.needs_repaint
    }

    pub fn drain_navigations(&mut self) -> Vec<NavigationOptions> {
        self.navigations.drain()
    }

    pub fn cursor_icon(&self) -> Option<cursor_icon::CursorIcon> {
        self.doc.get_cursor()
    }

    /// Brings the document viewport in sync with the widget's layout bounds
    /// (logical size) and the tracked scale factor / color scheme.
    pub fn sync_viewport(&mut self, logical_size: Size) {
        let zoom = self.doc.viewport().zoom();
        let scale = self.scale_factor;

        let physical_width = (logical_size.width * scale).round().max(0.0) as u32;
        let physical_height = (logical_size.height * scale).round().max(0.0) as u32;

        let current = self.doc.viewport();
        if current.window_size == (physical_width, physical_height)
            && current.hidpi_scale == scale
            && current.color_scheme == self.color_scheme
        {
            return;
        }

        let mut viewport = Viewport::new(physical_width, physical_height, scale, self.color_scheme);
        viewport.set_zoom(zoom);

        self.doc.set_viewport(viewport);
        self.needs_repaint = true;
        self.needs_resolve = true;
    }

    /// Returns the current frame, rasterizing one if anything changed since
    /// the last call, and ensures it is resident on the renderer.
    ///
    /// The synchronous `load_image` is essential: handing `draw_image` a
    /// handle the renderer has never seen makes it draw *nothing* for that
    /// frame and upload in the background — since every repaint mints a new
    /// handle, the view would blink on every hover/scroll/click. Holding the
    /// returned [`image::Allocation`] also keeps the texture from being
    /// evicted from the atlas between frames.
    pub fn frame(
        &mut self,
        renderer: &impl image::Renderer<Handle = image::Handle>,
    ) -> Option<image::Handle> {
        let handle = self.rasterize()?;
        self.frame_allocation = renderer.load_image(&handle).ok();

        Some(handle)
    }

    /// Returns the current frame, rasterizing one if anything changed since
    /// the last call. Returns `None` when the viewport has no area yet.
    fn rasterize(&mut self) -> Option<image::Handle> {
        let (width, height) = self.doc.viewport().window_size;

        if width == 0 || height == 0 {
            return None;
        }

        if !self.needs_repaint
            && let Some(frame) = &self.frame
        {
            return Some(frame.clone());
        }

        // Only run the expensive style+layout resolve when the document
        // structure or styles actually changed. Scroll offset changes and
        // hover-target changes only need a re-rasterize of the same scene.
        if self.needs_resolve {
            let _scale = self.doc.viewport().scale_f64();
            let animation_time = self.animation_time();
            self.doc.resolve(animation_time);
            self.needs_resolve = false;

            // Painting before stylesheets have arrived would flash unstyled
            // content; keep the previous frame (or nothing) until they land.
            if self.doc.has_pending_critical_resources() {
                return self.frame.clone();
            }
        }

        if self.renderer_size != (width, height) {
            self.renderer.resize(width, height);
            self.renderer_size = (width, height);
        } else {
            self.renderer.reset();
        }

        let mut pixels = Vec::new();
        let scale = self.doc.viewport().scale_f64();
        let doc = &mut *self.doc;
        self.renderer.render_to_vec(
            |scene| {
                // The CSS canvas is white unless the page paints over it;
                // also guarantees the buffer is fully opaque.
                scene.fill(
                    peniko::Fill::NonZero,
                    kurbo::Affine::IDENTITY,
                    peniko::Color::WHITE,
                    None,
                    &kurbo::Rect::new(0.0, 0.0, f64::from(width), f64::from(height)),
                );

                paint_scene(scene, doc, scale, width, height, 0, 0);
            },
            &mut pixels,
        );

        let handle = image::Handle::from_rgba(width, height, pixels);
        self.frame = Some(handle.clone());
        self.needs_repaint = false;

        Some(handle)
    }

    pub fn track_modifiers(&mut self, modifiers: iced::keyboard::Modifiers) {
        use keyboard_types::Modifiers as Mods;

        let mut mods = Mods::default();
        if modifiers.shift() {
            mods.insert(Mods::SHIFT);
        }
        if modifiers.control() {
            mods.insert(Mods::CONTROL);
        }
        if modifiers.alt() {
            mods.insert(Mods::ALT);
        }
        if modifiers.logo() {
            mods.insert(Mods::SUPER);
        }

        self.modifiers = mods;
    }

    pub fn pointer_moved(&mut self, position: Point) {
        let event = self.pointer_event(position, MouseEventButton::default());
        self.doc.handle_ui_event(UiEvent::PointerMove(event));
        // Hover changes only need a repaint, not a full resolve.
        self.needs_repaint = true;
    }

    pub fn pointer_button(&mut self, position: Point, button: MouseEventButton, pressed: bool) {
        if pressed {
            self.buttons.insert(button.into());
        } else {
            self.buttons.remove(button.into());
        }

        let event = self.pointer_event(position, button);
        let event = if pressed {
            UiEvent::PointerDown(event)
        } else {
            UiEvent::PointerUp(event)
        };

        self.doc.handle_ui_event(event);

        // Clicks can toggle form controls, move focus, activate `:active`
        // styles — assume the frame is stale and needs a full resolve.
        self.needs_repaint = true;
        self.needs_resolve = true;
    }

    pub fn pointer_left(&mut self) {
        if self.doc.clear_hover() {
            self.needs_repaint = true;
        }
    }

    pub fn wheel(&mut self, _position: Point, delta: iced::mouse::ScrollDelta) {
        let zoom = f64::from(self.doc.viewport().zoom());

        let (dx, dy) = match delta {
            iced::mouse::ScrollDelta::Lines { x, y } => {
                // Scale line delta to pixels (1 line ≈ 40px, matching typical
                // browser behavior) and adjust for zoom.
                let dx = f64::from(x) * 40.0 / zoom;
                let dy = f64::from(y) * 40.0 / zoom;
                (dx, dy)
            }
            iced::mouse::ScrollDelta::Pixels { x, y } => (f64::from(x) / zoom, f64::from(y) / zoom),
        };

        // Directly scroll the viewport — this is much cheaper than going
        // through the full Blitz event system (which would run the event
        // driver, dispatch DOM events, etc.). Scroll offset changes only
        // need a re-rasterize, not a style/layout resolve.
        let changed = self.doc.scroll_viewport_by_has_changed(dx, dy);
        if changed {
            self.needs_repaint = true;
        }
    }

    fn animation_time(&self) -> f64 {
        self.started_at.elapsed().as_secs_f64()
    }

    /// Converts a widget-relative logical position into Blitz pointer
    /// coordinates: client coordinates are CSS pixels (iced logical pixels
    /// divided by zoom), page coordinates add the viewport scroll offset.
    fn pointer_coords(&self, position: Point) -> PointerCoords {
        let zoom = self.doc.viewport().zoom();
        let client_x = position.x / zoom;
        let client_y = position.y / zoom;

        let scroll = self.doc.viewport_scroll();

        PointerCoords {
            client_x,
            client_y,
            page_x: client_x + scroll.x as f32,
            page_y: client_y + scroll.y as f32,
            screen_x: client_x,
            screen_y: client_y,
        }
    }

    fn pointer_event(&self, position: Point, button: MouseEventButton) -> BlitzPointerEvent {
        BlitzPointerEvent {
            id: BlitzPointerId::Mouse,
            is_primary: true,
            coords: self.pointer_coords(position),
            button,
            buttons: self.buttons,
            mods: self.modifiers,
            details: PointerDetails::default(),
        }
    }
}

fn build_document(
    html: &str,
    base_url: Option<String>,
    shell: &Arc<crate::providers::ShellFlags>,
    net: &Arc<crate::providers::Fetcher>,
    navigations: &Arc<crate::providers::NavigationQueue>,
) -> HtmlDocument {
    HtmlDocument::from_html(
        html,
        DocumentConfig {
            base_url,
            net_provider: Some(net.clone() as Arc<dyn NetProvider>),
            navigation_provider: Some(navigations.clone() as _),
            shell_provider: Some(shell.clone() as Arc<dyn ShellProvider>),
            ..Default::default()
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_fetch_completes() {
        let html = r#"<html><body><img src="https://picsum.photos/id/29/300/180" width="300" height="180"></body></html>"#;
        let mut engine = Engine::new(html, None);
        engine.set_scale_factor(1.0);
        engine.sync_viewport(Size::new(800.0, 600.0));

        for i in 0..60 {
            std::thread::sleep(std::time::Duration::from_millis(250));
            engine.absorb_signals();
            let frame = engine.rasterize();
            println!(
                "tick {i}: in_flight={} critical={} frame={}",
                engine.net.in_flight(),
                engine.doc.has_pending_critical_resources(),
                frame.is_some(),
            );
            if engine.net.in_flight() == 0 && i > 2 {
                break;
            }
        }

        assert_eq!(engine.net.in_flight(), 0, "image fetch never completed");

        // Render and verify the image's pixels actually landed in the frame.
        engine.mark_dirty();
        let animation_time = engine.animation_time();
        engine.doc.resolve(animation_time);
        let mut pixels = Vec::new();
        let (width, height) = engine.doc.viewport().window_size;
        engine.renderer.resize(width, height);
        let doc = &mut *engine.doc;
        engine.renderer.render_to_vec(
            |scene| {
                scene.fill(
                    peniko::Fill::NonZero,
                    kurbo::Affine::IDENTITY,
                    peniko::Color::WHITE,
                    None,
                    &kurbo::Rect::new(0.0, 0.0, f64::from(width), f64::from(height)),
                );
                paint_scene(scene, doc, 1.0, width, height, 0, 0);
            },
            &mut pixels,
        );

        let mut non_white = 0usize;
        for y in 20..190 {
            for x in 20..290 {
                let i = (y * width as usize + x) * 4;
                let (r, g, b) = (pixels[i], pixels[i + 1], pixels[i + 2]);
                if r < 240 || g < 240 || b < 240 {
                    non_white += 1;
                }
            }
        }
        println!("non-white pixels in image region: {non_white}");
        assert!(
            non_white > 5000,
            "image did not paint (non_white={non_white})"
        );
    }
}
