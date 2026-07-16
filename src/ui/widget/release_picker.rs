use iced::advanced::layout::{Layout, Limits, Node};
use iced::advanced::renderer::{self, Quad};
use iced::advanced::text::{self, Text};
use iced::advanced::widget::{Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay};
use iced::{
    Border, Color, Element, Event, Length, Pixels, Point, Rectangle, Size, Vector, alignment,
    keyboard, mouse,
};

use crate::services::game_mod::Release;
use crate::ui::theme::grunt_theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Ok,
    Maybe,
    Old,
}

fn status(release: &Release, game_version: &semver::Version) -> Status {
    if release.tags.iter().any(|t| t == game_version) {
        return Status::Ok;
    }
    let minor_prefix = match game_version.to_string().rfind('.') {
        Some(i) => &game_version.to_string()[..=i],
        None => &game_version.to_string(),
    };
    if release
        .tags
        .iter()
        .any(|t| t.to_string().starts_with(minor_prefix))
    {
        return Status::Maybe;
    }
    Status::Old
}

fn dot_color(s: Status) -> Color {
    let palette = grunt_theme().palette();
    match s {
        Status::Ok => palette.success,
        Status::Maybe => palette.warning,
        Status::Old => palette.danger,
    }
}

fn accent_color(s: Status) -> Color {
    let palette = grunt_theme().palette();
    match s {
        Status::Ok => palette.success,
        Status::Maybe => palette.warning,
        Status::Old => palette.danger,
    }
}

const POPOVER_W: f32 = 470.0;
const POPOVER_H: f32 = 340.0;
const LIST_W: f32 = 150.0;
const ROW_H: f32 = 26.0;
const ROW_GAP: f32 = 4.0;
const ROW_PITCH: f32 = ROW_H + ROW_GAP;
const LIST_PAD_Y: f32 = 6.0;
const ANCHOR_GAP: f32 = 10.0;
const DETAIL_PAD: f32 = 10.0;
const CHIP_H: f32 = 18.0;
const CHIP_GAP: f32 = 4.0;
const USE_BTN_W: f32 = 80.0;
const USE_BTN_H: f32 = 30.0;

#[derive(Default)]
struct PickerState {
    open: bool,
    highlighted: Option<String>,
    scroll: f32,
    use_pressed: bool,
}

pub struct ReleasePicker<'a, Message> {
    releases: &'a [Release],
    game_version: semver::Version,
    selected: Option<&'a Release>,
    on_select: Option<Box<dyn Fn(Release) -> Message + 'a>>,
    text_size: f32,
}

impl<'a, Message> ReleasePicker<'a, Message> {
    pub fn new(
        releases: &'a [Release],
        game_version: semver::Version,
        selected: Option<&'a Release>,
    ) -> Self {
        Self {
            releases,
            game_version,
            selected,
            on_select: None,
            text_size: 13.0,
        }
    }

    pub fn on_select(mut self, f: impl Fn(Release) -> Message + 'a) -> Self {
        self.on_select = Some(Box::new(f));
        self
    }

    pub fn text_size(mut self, size: f32) -> Self {
        self.text_size = size;
        self
    }

    fn effective_selected(&self) -> Option<&Release> {
        if let Some(v) = self.selected
            && let Some(r) = self.releases.iter().find(|r| r.modversion == v.modversion)
        {
            return Some(r);
        }
        self.releases
            .iter()
            .find(|r| status(r, &self.game_version) == Status::Ok)
            .or_else(|| self.releases.first())
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer> for ReleasePicker<'a, Message>
where
    Renderer: text::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<PickerState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(PickerState::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(120.0), Length::Fixed(30.0))
    }

    fn layout(&mut self, _tree: &mut Tree, _renderer: &Renderer, limits: &Limits) -> Node {
        Node::new(limits.resolve(
            Length::Fixed(120.0),
            Length::Fixed(30.0),
            Size::new(120.0, 30.0),
        ))
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        __clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<PickerState>();

        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
            && cursor.is_over(layout.bounds())
        {
            state.open = !state.open;
            state.highlighted = None;
            shell.capture_event();
            shell.request_redraw();
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
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::None
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<PickerState>();
        let bounds = layout.bounds();

        let (label, st) = match self.effective_selected() {
            Some(r) => (r.modversion.to_string(), status(r, &self.game_version)),
            None => ("-".to_string(), Status::Old),
        };

        let palette = grunt_theme().palette();
        let border_color = match st {
            Status::Ok => palette.success,
            Status::Maybe => palette.warning,
            Status::Old => palette.danger,
        };

        renderer.fill_quad(
            Quad {
                bounds,
                border: Border {
                    color: border_color,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Quad::default()
            },
            palette.background,
        );

        let dot = Rectangle {
            x: bounds.x + 9.0,
            y: bounds.y + bounds.height / 2.0 - 3.5,
            width: 7.0,
            height: 7.0,
        };
        renderer.fill_quad(
            Quad {
                bounds: dot,
                border: Border {
                    radius: 3.5.into(),
                    ..Border::default()
                },
                ..Quad::default()
            },
            dot_color(st),
        );

        fill_label(
            renderer,
            &label,
            Point::new(bounds.x + 22.0, bounds.y + bounds.height / 2.0),
            self.text_size,
            palette.text,
            text::Alignment::Left,
            *viewport,
        );

        fill_label(
            renderer,
            if state.open { "\u{25B2}" } else { "\u{25BC}" },
            Point::new(
                bounds.x + bounds.width - 12.0,
                bounds.y + bounds.height / 2.0,
            ),
            10.0,
            border_color,
            text::Alignment::Center,
            *viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        _renderer: &Renderer,
        _viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let state = tree.state.downcast_mut::<PickerState>();
        if !state.open {
            return None;
        }

        let anchor = layout.bounds() + translation;

        Some(overlay::Element::new(Box::new(Popover {
            releases: self.releases,
            game_version: &self.game_version,
            selected: self.selected,
            on_select: self.on_select.as_deref(),
            state,
            anchor,
            text_size: self.text_size,
        })))
    }
}

impl<'a, Message, Theme, Renderer> From<ReleasePicker<'a, Message>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: text::Renderer + 'a,
{
    fn from(picker: ReleasePicker<'a, Message>) -> Self {
        Element::new(picker)
    }
}

struct Popover<'a, 'b, Message> {
    releases: &'a [Release],
    game_version: &'a semver::Version,
    selected: Option<&'a Release>,
    on_select: Option<&'b (dyn Fn(Release) -> Message + 'a)>,
    state: &'b mut PickerState,
    anchor: Rectangle,
    text_size: f32,
}

impl<'a, 'b, Message> Popover<'a, 'b, Message> {
    fn detail_release(&self) -> Option<&'a Release> {
        if let Some(hi) = &self.state.highlighted
            && let Some(r) = self
                .releases
                .iter()
                .find(|r| &r.modversion.to_string() == hi)
        {
            return Some(r);
        }
        if let Some(v) = self.selected
            && let Some(r) = self.releases.iter().find(|r| r.modversion == v.modversion)
        {
            return Some(r);
        }
        self.releases
            .iter()
            .find(|r| status(r, self.game_version) == Status::Ok)
            .or_else(|| self.releases.first())
    }

    fn list_bounds(&self, bounds: Rectangle) -> Rectangle {
        Rectangle {
            width: LIST_W,
            ..bounds
        }
    }

    fn detail_bounds(&self, bounds: Rectangle) -> Rectangle {
        Rectangle {
            x: bounds.x + LIST_W,
            width: bounds.width - LIST_W,
            ..bounds
        }
    }

    fn max_scroll(&self) -> f32 {
        let content = self.releases.len() as f32 * ROW_PITCH - ROW_GAP + 2.0 * LIST_PAD_Y;
        (content - POPOVER_H).max(0.0)
    }

    fn row_at(&self, bounds: Rectangle, pos: Point) -> Option<usize> {
        let list = self.list_bounds(bounds);
        if !list.contains(pos) {
            return None;
        }
        let y = pos.y - list.y - LIST_PAD_Y + self.state.scroll;
        if y < 0.0 {
            return None;
        }
        let idx = (y / ROW_PITCH) as usize;
        (idx < self.releases.len()).then_some(idx)
    }

    fn use_button_label(&self) -> Option<(String, Status)> {
        let rel = self.detail_release()?;
        let st = status(rel, self.game_version);
        let label = if st == Status::Ok {
            format!("Use {}", rel.modversion)
        } else {
            format!("Use {} anyway", rel.modversion)
        };
        Some((label, st))
    }

    fn use_button_bounds(&self, bounds: Rectangle) -> Rectangle {
        let detail = self.detail_bounds(bounds);
        let width = self
            .use_button_label()
            .map(|(label, _)| estimate_width(&label, 12.5) + 24.0)
            .unwrap_or(USE_BTN_W)
            .max(USE_BTN_W);
        Rectangle {
            x: detail.x + detail.width - DETAIL_PAD - width,
            y: detail.y + detail.height - DETAIL_PAD - USE_BTN_H,
            width,
            height: USE_BTN_H,
        }
    }

    fn commit(&mut self, version: Release, shell: &mut Shell<'_, Message>) {
        if let Some(on_select) = self.on_select {
            shell.publish(on_select(version));
        }
        self.state.open = false;
        self.state.highlighted = None;
    }

    fn move_highlight(&mut self, delta: i32) {
        let current = self
            .detail_release()
            .map(|r| r.modversion.to_string())
            .unwrap_or_default();
        let idx = self
            .releases
            .iter()
            .position(|r| r.modversion.to_string() == current)
            .unwrap_or(0) as i32;
        let next = (idx + delta).clamp(0, self.releases.len().saturating_sub(1) as i32) as usize;
        self.state.highlighted = Some(self.releases[next].modversion.to_string());

        let row_top = next as f32 * ROW_PITCH;
        let view_h = POPOVER_H - 2.0 * LIST_PAD_Y;
        if row_top < self.state.scroll {
            self.state.scroll = row_top;
        } else if row_top + ROW_H > self.state.scroll + view_h {
            self.state.scroll = row_top + ROW_H - view_h;
        }
    }
}

impl<'a, 'b, Message, Theme, Renderer> overlay::Overlay<Message, Theme, Renderer>
    for Popover<'a, 'b, Message>
where
    Renderer: text::Renderer,
{
    fn layout(&mut self, _renderer: &Renderer, bounds: Size) -> Node {
        let size = Size::new(POPOVER_W, POPOVER_H);

        let x = (self.anchor.x + self.anchor.width - size.width).clamp(
            ANCHOR_GAP,
            (bounds.width - size.width - ANCHOR_GAP).max(ANCHOR_GAP),
        );

        let above = self.anchor.y - size.height - ANCHOR_GAP;
        let y = if above >= ANCHOR_GAP {
            above
        } else {
            (self.anchor.y + self.anchor.height + ANCHOR_GAP)
                .min(bounds.height - size.height - ANCHOR_GAP)
        };

        Node::new(size).move_to(Point::new(x, y))
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        __clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let bounds = layout.bounds();

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(pos) = cursor.position() else {
                    return;
                };

                if !bounds.contains(pos) {
                    if self.anchor.contains(pos) {
                        return;
                    }
                    self.state.open = false;
                    self.state.highlighted = None;
                    shell.capture_event();
                    shell.request_redraw();
                    return;
                }

                if let Some(idx) = self.row_at(bounds, pos) {
                    self.state.highlighted = Some(self.releases[idx].modversion.to_string());
                    shell.request_redraw();
                } else if self.use_button_bounds(bounds).contains(pos) {
                    self.state.use_pressed = true;
                    shell.request_redraw();
                }
                shell.capture_event();
            }

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if self.state.use_pressed =>
            {
                self.state.use_pressed = false;
                if cursor.is_over(self.use_button_bounds(bounds))
                    && let Some(r) = self.detail_release()
                {
                    self.commit(r.clone(), shell);
                }
                shell.capture_event();
                shell.request_redraw();
            }

            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                shell.request_redraw();
            }

            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let Some(pos) = cursor.position() else {
                    return;
                };
                if self.list_bounds(bounds).contains(pos) {
                    let dy = match delta {
                        mouse::ScrollDelta::Lines { y, .. } => y * ROW_PITCH * 1.5,
                        mouse::ScrollDelta::Pixels { y, .. } => *y,
                    };
                    self.state.scroll = (self.state.scroll - dy).clamp(0.0, self.max_scroll());
                    shell.capture_event();
                    shell.request_redraw();
                }
            }

            Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => match key.as_ref() {
                keyboard::Key::Named(keyboard::key::Named::Escape) => {
                    self.state.open = false;
                    self.state.highlighted = None;
                    shell.capture_event();
                    shell.request_redraw();
                }
                keyboard::Key::Named(keyboard::key::Named::ArrowDown) => {
                    self.move_highlight(1);
                    shell.capture_event();
                    shell.request_redraw();
                }
                keyboard::Key::Named(keyboard::key::Named::ArrowUp) => {
                    self.move_highlight(-1);
                    shell.capture_event();
                    shell.request_redraw();
                }
                keyboard::Key::Named(keyboard::key::Named::Enter) => {
                    if let Some(r) = self.detail_release() {
                        self.commit(r.clone(), shell);
                    }
                    shell.capture_event();
                    shell.request_redraw();
                }
                _ => {}
            },

            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let bounds = layout.bounds();
        if let Some(pos) = cursor.position()
            && (self.row_at(bounds, pos).is_some() || self.use_button_bounds(bounds).contains(pos))
        {
            return mouse::Interaction::Pointer;
        }
        mouse::Interaction::None
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let bounds = layout.bounds();

        let palette = grunt_theme();
        let palette = palette.extended_palette();
        renderer.fill_quad(
            Quad {
                bounds,
                border: Border {
                    color: palette.background.strong.color,
                    width: 1.0,
                    radius: 8.0.into(),
                },
                shadow: iced::Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.55),
                    offset: Vector::new(0.0, 8.0),
                    blur_radius: 24.0,
                },
                ..Quad::default()
            },
            palette.background.base.color,
        );

        let list = self.list_bounds(bounds);

        renderer.fill_quad(
            Quad {
                bounds: Rectangle {
                    x: list.x + list.width,
                    y: list.y,
                    width: 1.0,
                    height: list.height,
                },
                ..Quad::default()
            },
            palette.background.strong.color,
        );

        let detail_rel = self.detail_release();
        let hovered_row = cursor.position().and_then(|p| self.row_at(bounds, p));

        renderer.with_layer(list, |renderer| {
            for (i, release) in self.releases.iter().enumerate() {
                let row_y = list.y + LIST_PAD_Y + i as f32 * ROW_PITCH - self.state.scroll;
                if row_y + ROW_H < list.y || row_y > list.y + list.height {
                    continue;
                }
                let row = Rectangle {
                    x: list.x,
                    y: row_y,
                    width: list.width,
                    height: ROW_H,
                };

                let st = status(release, self.game_version);
                let is_detail = detail_rel.is_some_and(|r| r.modversion == release.modversion);
                let is_committed = self.selected == Some(release);

                let highlight = Rectangle {
                    x: row.x + 4.0,
                    width: row.width - 8.0,
                    ..row
                };
                if is_detail {
                    renderer.fill_quad(
                        Quad {
                            bounds: highlight,
                            border: Border {
                                radius: 4.0.into(),
                                ..Border::default()
                            },
                            ..Quad::default()
                        },
                        palette.primary.base.color,
                    );
                } else if hovered_row == Some(i) {
                    renderer.fill_quad(
                        Quad {
                            bounds: highlight,
                            border: Border {
                                radius: 4.0.into(),
                                ..Border::default()
                            },
                            ..Quad::default()
                        },
                        palette.background.strong.color,
                    );
                }

                let (fg, dot) = if is_detail {
                    (palette.primary.base.text, palette.primary.base.text)
                } else {
                    (palette.background.base.text, dot_color(st))
                };

                renderer.fill_quad(
                    Quad {
                        bounds: Rectangle {
                            x: row.x + 10.0,
                            y: row.y + ROW_H / 2.0 - 3.5,
                            width: 7.0,
                            height: 7.0,
                        },
                        ..Quad::default()
                    },
                    dot,
                );

                fill_label(
                    renderer,
                    &release.modversion.to_string(),
                    Point::new(row.x + 24.0, row.y + ROW_H / 2.0),
                    self.text_size,
                    fg,
                    text::Alignment::Left,
                    list,
                );

                if is_committed {
                    fill_label(
                        renderer,
                        "\u{25CF}",
                        Point::new(row.x + row.width - 14.0, row.y + ROW_H / 2.0),
                        9.0,
                        fg,
                        text::Alignment::Center,
                        list,
                    );
                }
            }
        });

        let Some(rel) = detail_rel else { return };
        let detail = self.detail_bounds(bounds);
        let st = status(rel, self.game_version);
        let x0 = detail.x + DETAIL_PAD;
        let mut y = detail.y + DETAIL_PAD + 8.0;

        fill_label(
            renderer,
            &rel.modversion.to_string(),
            Point::new(x0, y),
            15.0,
            palette.background.base.text,
            text::Alignment::Left,
            detail,
        );
        fill_label(
            renderer,
            &format!("released {}", rel.created),
            Point::new(
                x0 + estimate_width(&rel.modversion.to_string(), 15.0) + 10.0,
                y + 1.5,
            ),
            11.0,
            palette.background.weak.text,
            text::Alignment::Left,
            detail,
        );
        y += 24.0;

        let (icon, line) = match st {
            Status::Ok => (
                "\u{2713}".to_string(),
                format!("Marked for game {}", self.game_version),
            ),
            Status::Maybe => (
                "\u{26A0}".to_string(),
                format!(
                    "Not marked for {}. Closest target is {}. Patch releases usually still work.",
                    self.game_version,
                    rel.tags
                        .first()
                        .map(|t| t.to_string())
                        .unwrap_or("-".to_string())
                ),
            ),
            Status::Old => (
                "\u{2715}".to_string(),
                format!(
                    "Only targets older game versions (newest: {})",
                    rel.tags
                        .first()
                        .map(|t| t.to_string())
                        .unwrap_or("-".to_string())
                ),
            ),
        };
        fill_label(
            renderer,
            &icon,
            Point::new(x0, y),
            12.0,
            accent_color(st),
            text::Alignment::Left,
            detail,
        );
        let line_x = x0 + 18.0;
        let line_w = detail.x + detail.width - DETAIL_PAD - line_x;
        for wrapped in wrap_text(&line, 12.0, line_w) {
            fill_label(
                renderer,
                &wrapped,
                Point::new(line_x, y),
                12.0,
                accent_color(st),
                text::Alignment::Left,
                detail,
            );
            y += 16.0;
        }
        y += 6.0;

        fill_label(
            renderer,
            &format!("Targets {} game version(s)", rel.tags.len()),
            Point::new(x0, y),
            11.0,
            palette.background.weak.text,
            text::Alignment::Left,
            detail,
        );
        y += 20.0;

        let mut cx = x0;
        let max_x = detail.x + detail.width - DETAIL_PAD;
        for (i, target) in rel.tags.iter().enumerate() {
            let is_gv = target == self.game_version;
            let is_closest = st != Status::Ok && i == 0;
            let label = if is_closest {
                format!("{} \u{00B7} closest", target)
            } else {
                target.to_string()
            };
            let w = estimate_width(&label, 10.5) + 12.0;

            if cx + w > max_x {
                cx = x0;
                y += CHIP_H + CHIP_GAP;
            }

            let (bg, bd, fg) = if is_gv {
                (
                    palette.success.weak.color,
                    palette.success.base.color,
                    palette.success.weak.text,
                )
            } else if is_closest {
                (
                    palette.warning.weak.color,
                    palette.warning.base.color,
                    palette.warning.weak.text,
                )
            } else {
                (
                    palette.danger.base.color,
                    Color::TRANSPARENT,
                    palette.danger.base.text,
                )
            };

            let chip = Rectangle {
                x: cx,
                y: y - CHIP_H / 2.0,
                width: w,
                height: CHIP_H,
            };
            renderer.fill_quad(
                Quad {
                    bounds: chip,
                    border: Border {
                        color: bd,
                        width: 1.0,
                        radius: (CHIP_H / 2.0).into(),
                    },
                    ..Quad::default()
                },
                bg,
            );
            fill_label(
                renderer,
                &label,
                Point::new(cx + w / 2.0, chip.y + CHIP_H / 2.0 + 0.5),
                10.5,
                fg,
                text::Alignment::Center,
                detail,
            );

            cx += w + CHIP_GAP;
        }

        let mut btn = self.use_button_bounds(bounds);
        let Some((label, st)) = self.use_button_label() else {
            return;
        };
        let is_hovered = cursor.is_over(btn);
        let is_pressed = self.state.use_pressed;

        let (bg, bd, fg) = if st == Status::Ok {
            let pair = if is_pressed {
                palette.primary.weak
            } else if is_hovered {
                palette.primary.strong
            } else {
                palette.primary.base
            };
            (pair.color, palette.primary.strong.color, pair.text)
        } else if is_pressed {
            (
                palette.warning.base.color,
                palette.warning.base.color,
                palette.warning.base.text,
            )
        } else if is_hovered {
            (
                palette.warning.weak.color,
                palette.warning.base.color,
                palette.warning.weak.text,
            )
        } else {
            (
                Color::TRANSPARENT,
                palette.warning.base.color,
                palette.warning.base.color,
            )
        };

        if is_pressed {
            btn = Rectangle {
                x: btn.x + btn.width * 0.02,
                y: btn.y + btn.height * 0.02,
                width: btn.width * 0.96,
                height: btn.height * 0.96,
            };
        }
        renderer.fill_quad(
            Quad {
                bounds: btn,
                border: Border {
                    color: bd,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Quad::default()
            },
            bg,
        );
        fill_label(
            renderer,
            &label,
            Point::new(btn.x + btn.width / 2.0, btn.y + btn.height / 2.0),
            12.5,
            fg,
            text::Alignment::Center,
            bounds,
        );
    }
}

fn fill_label<Renderer: text::Renderer>(
    renderer: &mut Renderer,
    content: &str,
    position: Point,
    size: f32,
    color: Color,
    align: text::Alignment,
    clip: Rectangle,
) {
    renderer.fill_text(
        Text {
            content: content.to_string(),
            bounds: Size::new(f32::INFINITY, size * 1.4),
            size: Pixels(size),
            line_height: text::LineHeight::default(),
            font: renderer.default_font(),
            align_x: align,
            align_y: alignment::Vertical::Center,
            shaping: text::Shaping::Advanced,
            wrapping: text::Wrapping::None,
        },
        position,
        color,
        clip,
    );
}

fn wrap_text(s: &str, size: f32, max_width: f32) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();

    for word in s.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };

        if current.is_empty() || estimate_width(&candidate, size) <= max_width {
            current = candidate;
        } else {
            lines.push(std::mem::replace(&mut current, word.to_string()));
        }
    }

    if !current.is_empty() || lines.is_empty() {
        lines.push(current);
    }

    lines
}

fn estimate_width(s: &str, size: f32) -> f32 {
    s.chars().count() as f32 * size * 0.58
}
