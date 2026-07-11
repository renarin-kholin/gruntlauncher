use iced::advanced::layout::{Layout, Limits, Node};
use iced::advanced::renderer::{self, Quad};
use iced::advanced::text;
use iced::advanced::widget::{Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay};
use iced::{
    Border, Color, Element, Event, Length, Point, Rectangle, Size, Vector, keyboard, mouse,
};

use super::util::{estimate_width, fill_label};
use crate::core::account::{Account, AccountStatus};
use crate::ui::theme::grunt_theme;

pub type Email = String;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginRequest {
    AddAccount,
    Relogin(Email),
}

const CHIP_H: f32 = 30.0;
const MENU_W: f32 = 270.0;
const ROW_H: f32 = 30.0;
const ROW_GAP: f32 = 4.0;
const ROW_PITCH: f32 = ROW_H + ROW_GAP;
const PAD_Y: f32 = 6.0;
const ANCHOR_GAP: f32 = 6.0;
const TEXT_SIZE: f32 = 13.0;

#[derive(Default)]
struct SwitcherState {
    open: bool,
    confirm_remove: Option<String>,
}

pub struct AccountSwitcher<'a, Message> {
    accounts: &'a [Account],
    active: Option<&'a str>,
    on_select: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_remove: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_login: Option<Box<dyn Fn(LoginRequest) -> Message + 'a>>,
}

impl<'a, Message> AccountSwitcher<'a, Message> {
    pub fn new(accounts: &'a [Account], active: Option<&'a str>) -> Self {
        Self {
            accounts,
            active,
            on_select: None,
            on_remove: None,
            on_login: None,
        }
    }

    pub fn on_select(mut self, f: impl Fn(String) -> Message + 'a) -> Self {
        self.on_select = Some(Box::new(f));
        self
    }

    //Emitted after the inline confirmation, so the app can remove directly.
    pub fn on_remove(mut self, f: impl Fn(String) -> Message + 'a) -> Self {
        self.on_remove = Some(Box::new(f));
        self
    }

    pub fn on_login(mut self, f: impl Fn(LoginRequest) -> Message + 'a) -> Self {
        self.on_login = Some(Box::new(f));
        self
    }

    fn active_account(&self) -> Option<&'a Account> {
        self.active
            .and_then(|name| self.accounts.iter().find(|a| a.username == name))
            .or_else(|| self.accounts.first())
    }

    fn chip_width(&self) -> f32 {
        match self.active_account() {
            None => estimate_width("Sign in", TEXT_SIZE) + 28.0,
            Some(acc) => {
                let warn = if acc.status == AccountStatus::Expired {
                    18.0
                } else {
                    0.0
                };
                // dot + gap + name + (warn) + gap + caret + padding
                10.0 + 8.0
                    + 7.0
                    + estimate_width(&acc.username, TEXT_SIZE)
                    + warn
                    + 10.0
                    + 10.0
                    + 10.0
            }
        }
    }
}

fn status_color(status: AccountStatus, palette: &iced::theme::palette::Extended) -> Color {
    match status {
        AccountStatus::Ok => palette.success.base.color,
        AccountStatus::Expired => palette.warning.base.color,
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer> for AccountSwitcher<'a, Message>
where
    Renderer: text::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<SwitcherState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(SwitcherState::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Fixed(CHIP_H))
    }

    fn layout(&mut self, _tree: &mut Tree, _renderer: &Renderer, limits: &Limits) -> Node {
        let width = self.chip_width();
        Node::new(limits.resolve(
            Length::Shrink,
            Length::Fixed(CHIP_H),
            Size::new(width, CHIP_H),
        ))
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<SwitcherState>();

        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
            && cursor.is_over(layout.bounds())
        {
            if self.accounts.is_empty() {
                if let Some(on_login) = &self.on_login {
                    shell.publish(on_login(LoginRequest::AddAccount));
                }
            } else {
                state.open = !state.open;
                state.confirm_remove = None;
            }
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
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<SwitcherState>();
        let bounds = layout.bounds();
        let theme = grunt_theme();
        let palette = theme.extended_palette();
        let hovered = cursor.is_over(bounds);

        let Some(acc) = self.active_account() else {
            return;
        };

        let expired = acc.status == AccountStatus::Expired;
        let border_color = if state.open {
            palette.primary.base.color
        } else if expired {
            palette.warning.base.color
        } else {
            palette.background.strong.color
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
            if hovered {
                palette.background.weak.color
            } else {
                palette.background.weakest.color
            },
        );

        let cy = bounds.y + bounds.height / 2.0;
        renderer.fill_quad(
            Quad {
                bounds: Rectangle {
                    x: bounds.x + 10.0,
                    y: cy - 4.0,
                    width: 8.0,
                    height: 8.0,
                },
                border: Border {
                    radius: 4.0.into(),
                    ..Border::default()
                },
                ..Quad::default()
            },
            status_color(acc.status, palette),
        );

        let mut x = bounds.x + 10.0 + 8.0 + 7.0;
        fill_label(
            renderer,
            &acc.username,
            Point::new(x, cy),
            TEXT_SIZE,
            palette.background.base.text,
            text::Alignment::Left,
            *viewport,
        );
        x += estimate_width(&acc.username, TEXT_SIZE);

        if expired {
            fill_label(
                renderer,
                "\u{26A0}",
                Point::new(x + 10.0, cy),
                11.0,
                palette.warning.base.color,
                text::Alignment::Center,
                *viewport,
            );
        }

        fill_label(
            renderer,
            if state.open { "\u{25B2}" } else { "\u{25BC}" },
            Point::new(bounds.x + bounds.width - 14.0, cy),
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
        let state = tree.state.downcast_mut::<SwitcherState>();
        if !state.open || self.accounts.is_empty() {
            return None;
        }

        let anchor = layout.bounds() + translation;

        Some(overlay::Element::new(Box::new(Menu {
            accounts: self.accounts,
            active: self.active,
            on_select: self.on_select.as_deref(),
            on_remove: self.on_remove.as_deref(),
            on_login: self.on_login.as_deref(),
            state,
            anchor,
        })))
    }
}

impl<'a, Message, Theme, Renderer> From<AccountSwitcher<'a, Message>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: text::Renderer + 'a,
{
    fn from(switcher: AccountSwitcher<'a, Message>) -> Self {
        Element::new(switcher)
    }
}

struct Menu<'a, 'b, Message> {
    accounts: &'a [Account],
    active: Option<&'a str>,
    on_select: Option<&'b (dyn Fn(String) -> Message + 'a)>,
    on_remove: Option<&'b (dyn Fn(String) -> Message + 'a)>,
    on_login: Option<&'b (dyn Fn(LoginRequest) -> Message + 'a)>,
    state: &'b mut SwitcherState,
    anchor: Rectangle,
}

impl<'a, 'b, Message> Menu<'a, 'b, Message> {
    fn menu_height(&self) -> f32 {
        // rows (with trailing gap) + separator + footer row
        PAD_Y + self.accounts.len() as f32 * ROW_PITCH + 1.0 + ROW_H + PAD_Y
    }

    fn row_rect(&self, bounds: Rectangle, i: usize) -> Rectangle {
        Rectangle {
            x: bounds.x + 4.0,
            y: bounds.y + PAD_Y + i as f32 * ROW_PITCH,
            width: bounds.width - 8.0,
            height: ROW_H,
        }
    }

    fn separator_y(&self, bounds: Rectangle) -> f32 {
        bounds.y + PAD_Y + self.accounts.len() as f32 * ROW_PITCH
    }

    fn footer_rect(&self, bounds: Rectangle) -> Rectangle {
        Rectangle {
            x: bounds.x + 4.0,
            y: self.separator_y(bounds) + 1.0,
            width: bounds.width - 8.0,
            height: ROW_H,
        }
    }

    //Full-height hit area for the ✕ at the row's right edge.
    fn x_button_rect(row: Rectangle) -> Rectangle {
        Rectangle {
            x: row.x + row.width - 28.0,
            y: row.y,
            width: 28.0,
            height: row.height,
        }
    }

    fn relogin_rect(row: Rectangle) -> Rectangle {
        let w = estimate_width("Re-login", 11.5) + 14.0;
        let x_btn = Self::x_button_rect(row);
        Rectangle {
            x: x_btn.x - 4.0 - w,
            y: row.y + (row.height - 20.0) / 2.0,
            width: w,
            height: 20.0,
        }
    }

    fn confirm_button_rects(row: Rectangle) -> (Rectangle, Rectangle) {
        let cancel_w = estimate_width("Cancel", 12.0) + 12.0;
        let remove_w = estimate_width("Remove", 12.0) + 16.0;
        let cancel = Rectangle {
            x: row.x + row.width - 6.0 - cancel_w,
            y: row.y + (row.height - 20.0) / 2.0,
            width: cancel_w,
            height: 20.0,
        };
        let remove = Rectangle {
            x: cancel.x - 6.0 - remove_w,
            y: cancel.y,
            width: remove_w,
            height: 20.0,
        };
        (remove, cancel)
    }

    fn close(&mut self) {
        self.state.open = false;
        self.state.confirm_remove = None;
    }
}

impl<'a, 'b, Message, Theme, Renderer> overlay::Overlay<Message, Theme, Renderer>
    for Menu<'a, 'b, Message>
where
    Renderer: text::Renderer,
{
    fn layout(&mut self, _renderer: &Renderer, bounds: Size) -> Node {
        let size = Size::new(MENU_W, self.menu_height());

        // Right-align to the chip, clamped to the window.
        let x = (self.anchor.x + self.anchor.width - size.width).clamp(
            ANCHOR_GAP,
            (bounds.width - size.width - ANCHOR_GAP).max(ANCHOR_GAP),
        );

        // Prefer below the chip; flip above if there is no room.
        let below = self.anchor.y + self.anchor.height + ANCHOR_GAP;
        let y = if below + size.height <= bounds.height - ANCHOR_GAP {
            below
        } else {
            (self.anchor.y - size.height - ANCHOR_GAP).max(ANCHOR_GAP)
        };

        Node::new(size).move_to(Point::new(x, y))
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
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
                    self.close();
                    shell.capture_event();
                    shell.request_redraw();
                    return;
                }

                for (i, acc) in self.accounts.iter().enumerate() {
                    let row = self.row_rect(bounds, i);
                    if !row.contains(pos) {
                        continue;
                    }

                    if self.state.confirm_remove.as_deref() == Some(acc.username.as_str()) {
                        let (remove, cancel) = Self::confirm_button_rects(row);
                        if remove.contains(pos) {
                            let name = acc.username.clone();
                            self.state.confirm_remove = None;
                            if let Some(on_remove) = self.on_remove {
                                shell.publish(on_remove(name));
                            }
                        } else if cancel.contains(pos) {
                            self.state.confirm_remove = None;
                        }
                    } else if Self::x_button_rect(row).contains(pos) {
                        self.state.confirm_remove = Some(acc.username.clone());
                    } else if acc.status == AccountStatus::Expired
                        && Self::relogin_rect(row).contains(pos)
                    {
                        let name = acc.username.clone();
                        self.close();
                        if let Some(on_login) = self.on_login {
                            shell.publish(on_login(LoginRequest::Relogin(name)));
                        }
                    } else {
                        let name = acc.username.clone();
                        self.close();
                        if let Some(on_select) = self.on_select {
                            shell.publish(on_select(name));
                        }
                    }
                    shell.capture_event();
                    shell.request_redraw();
                    return;
                }

                if self.footer_rect(bounds).contains(pos) {
                    self.close();
                    if let Some(on_login) = self.on_login {
                        shell.publish(on_login(LoginRequest::AddAccount));
                    }
                }
                shell.capture_event();
                shell.request_redraw();
            }

            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                shell.request_redraw();
            }

            Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                if let keyboard::Key::Named(keyboard::key::Named::Escape) = key.as_ref() {
                    self.close();
                    shell.capture_event();
                    shell.request_redraw();
                }
            }

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
        let Some(pos) = cursor.position() else {
            return mouse::Interaction::None;
        };
        let over_row = (0..self.accounts.len()).any(|i| self.row_rect(bounds, i).contains(pos));
        if over_row || self.footer_rect(bounds).contains(pos) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::None
        }
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
        let theme = grunt_theme();
        let palette = theme.extended_palette();

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

        let cursor_pos = cursor.position();

        for (i, acc) in self.accounts.iter().enumerate() {
            let row = self.row_rect(bounds, i);
            let cy = row.y + row.height / 2.0;
            let is_active = Some(acc.username.as_str()) == self.active;
            let confirming = self.state.confirm_remove.as_deref() == Some(acc.username.as_str());

            if confirming {
                // Inline "Remove {name}?" confirmation row.
                renderer.fill_quad(
                    Quad {
                        bounds: row,
                        border: Border {
                            radius: 4.0.into(),
                            ..Border::default()
                        },
                        ..Quad::default()
                    },
                    palette.danger.weak.color,
                );
                fill_label(
                    renderer,
                    &format!("Remove {}?", acc.username),
                    Point::new(row.x + 10.0, cy),
                    12.5,
                    palette.danger.weak.text,
                    text::Alignment::Left,
                    bounds,
                );

                let (remove, cancel) = Self::confirm_button_rects(row);
                let remove_hovered = cursor_pos.is_some_and(|p| remove.contains(p));
                let (remove_bg, remove_fg) = if remove_hovered {
                    (palette.danger.base.color, palette.danger.base.text)
                } else {
                    (Color::TRANSPARENT, palette.danger.base.text)
                };
                renderer.fill_quad(
                    Quad {
                        bounds: remove,
                        border: Border {
                            color: if remove_hovered {
                                palette.danger.base.color
                            } else {
                                palette.danger.base.text
                            },
                            width: 1.0,
                            radius: 4.0.into(),
                        },
                        ..Quad::default()
                    },
                    remove_bg,
                );
                fill_label(
                    renderer,
                    "Remove",
                    Point::new(remove.x + remove.width / 2.0, cy),
                    12.0,
                    remove_fg,
                    text::Alignment::Center,
                    bounds,
                );

                let cancel_hovered = cursor_pos.is_some_and(|p| cancel.contains(p));
                fill_label(
                    renderer,
                    "Cancel",
                    Point::new(cancel.x + cancel.width / 2.0, cy),
                    12.0,
                    if cancel_hovered {
                        palette.background.base.text
                    } else {
                        palette.background.weak.text
                    },
                    text::Alignment::Center,
                    bounds,
                );
                continue;
            }

            let hovered = cursor_pos.is_some_and(|p| row.contains(p));
            if is_active {
                renderer.fill_quad(
                    Quad {
                        bounds: row,
                        border: Border {
                            radius: 4.0.into(),
                            ..Border::default()
                        },
                        ..Quad::default()
                    },
                    palette.background.stronger.color,
                );
            } else if hovered {
                renderer.fill_quad(
                    Quad {
                        bounds: row,
                        border: Border {
                            radius: 4.0.into(),
                            ..Border::default()
                        },
                        ..Quad::default()
                    },
                    palette.background.strong.color,
                );
            }
            let fg = if is_active {
                palette.background.stronger.text
            } else {
                palette.background.base.text
            };

            renderer.fill_quad(
                Quad {
                    bounds: Rectangle {
                        x: row.x + 10.0,
                        y: cy - 4.0,
                        width: 8.0,
                        height: 8.0,
                    },
                    border: Border {
                        radius: 4.0.into(),
                        ..Border::default()
                    },
                    ..Quad::default()
                },
                status_color(acc.status, palette),
            );
            fill_label(
                renderer,
                &acc.username,
                Point::new(row.x + 26.0, cy),
                TEXT_SIZE,
                fg,
                text::Alignment::Left,
                bounds,
            );

            let x_btn = Self::x_button_rect(row);
            let x_hovered = cursor_pos.is_some_and(|p| x_btn.contains(p));
            fill_label(
                renderer,
                "\u{2715}",
                Point::new(x_btn.x + x_btn.width / 2.0, cy),
                12.0,
                if x_hovered {
                    palette.danger.base.color
                } else {
                    palette.background.weak.text
                },
                text::Alignment::Center,
                bounds,
            );

            if acc.status == AccountStatus::Expired {
                let relogin = Self::relogin_rect(row);
                let relogin_hovered = cursor_pos.is_some_and(|p| relogin.contains(p));
                let (bg, fg) = if relogin_hovered {
                    (palette.warning.weak.color, palette.warning.weak.text)
                } else {
                    (Color::TRANSPARENT, palette.warning.base.color)
                };
                renderer.fill_quad(
                    Quad {
                        bounds: relogin,
                        border: Border {
                            color: palette.warning.base.color,
                            width: 1.0,
                            radius: 4.0.into(),
                        },
                        ..Quad::default()
                    },
                    bg,
                );
                fill_label(
                    renderer,
                    "Re-login",
                    Point::new(relogin.x + relogin.width / 2.0, cy),
                    11.5,
                    fg,
                    text::Alignment::Center,
                    bounds,
                );
            } else if is_active {
                fill_label(
                    renderer,
                    "\u{2713}",
                    Point::new(x_btn.x - 12.0, cy),
                    12.0,
                    palette.success.base.color,
                    text::Alignment::Center,
                    bounds,
                );
            }
        }

        renderer.fill_quad(
            Quad {
                bounds: Rectangle {
                    x: bounds.x + 1.0,
                    y: self.separator_y(bounds),
                    width: bounds.width - 2.0,
                    height: 1.0,
                },
                ..Quad::default()
            },
            palette.background.strong.color,
        );

        let footer = self.footer_rect(bounds);
        let footer_hovered = cursor_pos.is_some_and(|p| footer.contains(p));
        if footer_hovered {
            renderer.fill_quad(
                Quad {
                    bounds: footer,
                    border: Border {
                        radius: 4.0.into(),
                        ..Border::default()
                    },
                    ..Quad::default()
                },
                palette.background.strong.color,
            );
        }
        let fcy = footer.y + footer.height / 2.0;
        fill_label(
            renderer,
            "+",
            Point::new(footer.x + 14.0, fcy),
            14.0,
            palette.primary.base.color,
            text::Alignment::Center,
            bounds,
        );
        fill_label(
            renderer,
            "Add account\u{2026}",
            Point::new(footer.x + 26.0, fcy),
            TEXT_SIZE,
            palette.primary.base.color,
            text::Alignment::Left,
            bounds,
        );
    }
}
