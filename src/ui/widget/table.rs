use iced::Element;
use iced::Event;
use iced::Length;
use iced::Pixels;
use iced::Point;
use iced::Rectangle;
use iced::Size;
use iced::advanced::Widget;
use iced::advanced::layout;
use iced::advanced::mouse;
use iced::advanced::renderer;
use iced::advanced::text;
use iced::advanced::widget::Tree;
use iced::advanced::widget::tree;
use iced::alignment::Horizontal;
use iced::alignment::Vertical;
#[derive(Debug, Clone, PartialEq)]
pub struct TableColumn {
    pub header: String,
    pub initial_width: f32,
    pub min_width: f32,
}
impl TableColumn {
    pub fn new(header: impl Into<String>, initial_width: f32) -> Self {
        Self {
            header: header.into(),
            initial_width,
            min_width: 40.0,
        }
    }
    pub fn min_width(mut self, min: f32) -> Self {
        self.min_width = min;
        self
    }
}

pub struct Table<'a, Message> {
    columns: &'a [TableColumn],
    rows: &'a [Vec<String>],
    on_select: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    header_height: f32,
    row_height: f32,
    divider_grab_width: f32,
}

impl<'a, Message> Table<'a, Message> {
    pub fn new(columns: &'a [TableColumn], rows: &'a [Vec<String>]) -> Self {
        Self {
            columns,
            rows,
            on_select: None,
            header_height: 36.0,
            row_height: 32.0,
            divider_grab_width: 8.0,
        }
    }
    pub fn on_select(mut self, f: impl Fn(usize) -> Message + 'a) -> Self {
        self.on_select = Some(Box::new(f));
        self
    }
    pub fn row_height(mut self, h: f32) -> Self {
        self.row_height = h;
        self
    }
    pub fn header_height(mut self, h: f32) -> Self {
        self.header_height = h;
        self
    }
}
const SCROLLBAR_W: f32 = 6.0;
const SCROLLBAR_PAD: f32 = 2.0;
const MIN_THUMB_H: f32 = 24.0;

#[derive(Clone)]
struct TableState {
    column_widths: Vec<f32>,
    selected_row: Option<usize>,
    hovered_row: Option<usize>,
    hovered_divider: Option<usize>,
    resizing: Option<ColumnResize>,
    scroll: f32,
    scrollbar_drag: Option<f32>,
}
impl TableState {
    pub fn new() -> Self {
        Self {
            column_widths: vec![],
            selected_row: None,
            hovered_row: None,
            hovered_divider: None,
            resizing: None,
            scroll: 0.0,
            scrollbar_drag: None,
        }
    }
    pub fn init(&self, columns: &[TableColumn]) -> Self {
        Self {
            column_widths: columns.iter().map(|c| c.initial_width).collect(),
            ..Self::new()
        }
    }
}

#[derive(Clone)]
struct ColumnResize {
    column_index: usize,
    start_cursor_x: f32,
    start_width: f32,
}
impl<'a, Message, Renderer> Widget<Message, iced::Theme, Renderer> for Table<'a, Message>
where
    Renderer: renderer::Renderer + text::Renderer,
    Message: Clone,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<TableState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(TableState::new().init(self.columns))
    }
    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<TableState>();
        if state.column_widths.len() != self.columns.len() {
            *state = TableState::new().init(self.columns);
        }
    }
    fn size(&self) -> iced::Size<iced::Length> {
        Size {
            width: Length::Fill,
            height: Length::Shrink,
        }
    }
    fn draw(
        &self,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        theme: &iced::Theme,
        _style: &renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        _viewport: &iced::Rectangle,
    ) {
        let state = tree.state.downcast_ref::<TableState>();
        let bounds = layout.bounds();

        let widths = effective_widths(
            bounds,
            &state.column_widths,
            self.columns.last().map_or(0.0, |c| c.min_width),
        );
        let col_xs = col_start_xs(bounds, &widths);
        let div_xs = divider_xs(bounds, &widths);
        let body = body_bounds(bounds, self.header_height);
        let scroll = state.scroll.clamp(
            0.0,
            max_scroll(bounds, self.header_height, self.row_height, self.rows.len()),
        );

        renderer.fill_quad(
            renderer::Quad {
                bounds,
                ..renderer::Quad::default()
            },
            theme.palette().background,
        );
        for (i, col) in self.columns.iter().enumerate() {
            let cell = Rectangle {
                x: col_xs[i],
                y: bounds.y,
                width: widths[i],
                height: self.header_height,
            };
            renderer.fill_quad(
                renderer::Quad {
                    bounds: cell,
                    ..Default::default()
                },
                theme.extended_palette().background.weak.color,
            );
            renderer.fill_text(
                text::Text {
                    content: col.header.clone(),
                    bounds: Size::new(widths[i] - 16.0, self.header_height),
                    size: Pixels(13.0),
                    line_height: text::LineHeight::default(),
                    font: renderer.default_font(),
                    align_x: Horizontal::Left.into(),
                    align_y: Vertical::Center,
                    shaping: text::Shaping::Auto,
                    wrapping: text::Wrapping::None,
                },
                Point::new(cell.x + 8.0, cell.y + self.header_height / 2.0),
                theme.extended_palette().background.weak.text,
                cell,
            );
        }

        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: bounds.x,
                    y: bounds.y + self.header_height - 1.0,
                    width: bounds.width,
                    height: 2.0,
                },
                ..Default::default()
            },
            theme.extended_palette().background.strongest.color,
        );
        renderer.with_layer(body, |renderer| {
            for (row_idx, row) in self.rows.iter().enumerate() {
                let row_y = body.y + row_idx as f32 * self.row_height - scroll;
                if row_y + self.row_height < body.y || row_y > body.y + body.height {
                    continue;
                }
                let row_bg = if state.selected_row == Some(row_idx) {
                    theme.extended_palette().background.stronger
                } else if state.hovered_row == Some(row_idx) {
                    theme.extended_palette().background.weaker
                } else if row_idx % 2 == 0 {
                    theme.extended_palette().background.weakest
                } else {
                    theme.extended_palette().background.base
                };
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle {
                            x: bounds.x,
                            y: row_y,
                            width: bounds.width,
                            height: self.row_height,
                        },
                        ..Default::default()
                    },
                    row_bg.color,
                );
                for (col_idx, cell_text) in row.iter().take(self.columns.len()).enumerate() {
                    let cell = Rectangle {
                        x: col_xs[col_idx],
                        y: row_y,
                        width: widths[col_idx],
                        height: self.row_height,
                    };
                    renderer.fill_text(
                        text::Text {
                            content: cell_text.clone(),
                            bounds: Size::new(cell.width - 16.0, self.row_height),
                            size: Pixels(13.0),
                            line_height: text::LineHeight::default(),
                            font: renderer.default_font(),
                            align_x: Horizontal::Left.into(),
                            align_y: Vertical::Center,
                            shaping: text::Shaping::Auto,
                            wrapping: text::Wrapping::None,
                        },
                        Point::new(cell.x + 8.0, row_y + self.row_height / 2.0),
                        row_bg.text,
                        cell,
                    );
                }
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle {
                            x: bounds.x,
                            y: row_y + self.row_height - 0.5,
                            width: bounds.width,
                            height: 0.5,
                        },
                        ..Default::default()
                    },
                    theme.extended_palette().background.weak.color,
                );
            }
        });
        renderer.with_layer(bounds, |renderer| {
            for (i, &div_x) in div_xs.iter().enumerate() {
                let is_active = state.hovered_divider == Some(i)
                    || state.resizing.as_ref().is_some_and(|r| r.column_index == i);
                let color = if is_active {
                    theme.extended_palette().background.stronger.color
                } else {
                    theme.extended_palette().background.strong.color
                };
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle {
                            x: div_x - if is_active { 3.0 } else { 1.0 },
                            y: bounds.y,
                            width: if is_active { 6.0 } else { 2.0 },
                            height: bounds.height,
                        },
                        ..Default::default()
                    },
                    color,
                );
            }
            if let Some(thumb) = thumb_bounds(
                bounds,
                self.header_height,
                self.row_height,
                self.rows.len(),
                scroll,
            ) {
                let is_active = state.scrollbar_drag.is_some()
                    || cursor.position().is_some_and(|p| thumb.contains(p));
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: thumb,
                        border: iced::Border {
                            radius: (SCROLLBAR_W / 2.0).into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    if is_active {
                        theme.extended_palette().background.stronger.color
                    } else {
                        theme.extended_palette().background.strong.color
                    },
                );
            }
        });
    }
    fn layout(
        &mut self,
        _tree: &mut iced::advanced::widget::Tree,
        _renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        let total_height = self.header_height + self.rows.len() as f32 * self.row_height;
        let size = limits.resolve(
            Length::Fill,
            Length::Shrink,
            Size::new(limits.max().width, total_height),
        );

        layout::Node::new(size)
    }
    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced::Event,
        layout: layout::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let Event::Mouse(mouse_event) = event else {
            return;
        };
        let bounds = layout.bounds();
        let state = tree.state.downcast_mut::<TableState>();

        let widths = effective_widths(
            bounds,
            &state.column_widths,
            self.columns.last().map_or(0.0, |c| c.min_width),
        );
        let div_xs = divider_xs(bounds, &widths);

        let n_rows = self.rows.len();
        let scroll_limit = max_scroll(bounds, self.header_height, self.row_height, n_rows);
        state.scroll = state.scroll.clamp(0.0, scroll_limit);
        let thumb = thumb_bounds(
            bounds,
            self.header_height,
            self.row_height,
            n_rows,
            state.scroll,
        );

        match mouse_event {
            mouse::Event::CursorMoved { .. } => {
                let Some(pos) = cursor.position() else { return };
                if let Some(grab) = state.scrollbar_drag {
                    let body = body_bounds(bounds, self.header_height);
                    if let Some(thumb) = thumb {
                        let track_h = body.height - 2.0 * SCROLLBAR_PAD;
                        let range = track_h - thumb.height;
                        if range > 0.0 {
                            let progress = (pos.y - grab - (body.y + SCROLLBAR_PAD)) / range;
                            state.scroll = (progress * scroll_limit).clamp(0.0, scroll_limit);
                        }
                    }
                    shell.capture_event();
                    shell.request_redraw();
                } else if let Some(ref resize) = state.resizing.clone() {
                    let delta = pos.x - resize.start_cursor_x;
                    let min_w = self.columns[resize.column_index].min_width;
                    let n = state.column_widths.len();
                    let others: f32 = state.column_widths[..n - 1]
                        .iter()
                        .enumerate()
                        .filter(|(j, _)| *j != resize.column_index)
                        .map(|(_, w)| w)
                        .sum();
                    let last_min = self.columns.last().map_or(0.0, |c| c.min_width);
                    let max_w = (bounds.width - others - last_min).max(min_w);
                    state.column_widths[resize.column_index] =
                        (resize.start_width + delta).clamp(min_w, max_w);
                    state.hovered_divider = Some(resize.column_index);
                    shell.capture_event();
                    shell.request_redraw();
                } else {
                    let new_row = hit_row(
                        pos,
                        bounds,
                        self.header_height,
                        self.row_height,
                        n_rows,
                        state.scroll,
                    );
                    let new_div = hit_divider(pos, &div_xs, bounds, self.divider_grab_width);
                    let changed = new_row != state.hovered_row || new_div != state.hovered_divider;
                    state.hovered_row = new_row;
                    state.hovered_divider = new_div;

                    if changed {
                        shell.request_redraw();
                    }
                }
            }
            mouse::Event::WheelScrolled { delta } => {
                let Some(pos) = cursor.position() else { return };
                if scroll_limit > 0.0 && bounds.contains(pos) {
                    let dy = match delta {
                        mouse::ScrollDelta::Lines { y, .. } => y * self.row_height * 1.5,
                        mouse::ScrollDelta::Pixels { y, .. } => *y,
                    };
                    state.scroll = (state.scroll - dy).clamp(0.0, scroll_limit);
                    state.hovered_row = hit_row(
                        pos,
                        bounds,
                        self.header_height,
                        self.row_height,
                        n_rows,
                        state.scroll,
                    );
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            mouse::Event::ButtonPressed(mouse::Button::Left) => {
                let Some(pos) = cursor.position() else { return };
                if !bounds.contains(pos) {
                    return;
                }
                if let Some(thumb) = thumb.filter(|t| t.contains(pos)) {
                    state.scrollbar_drag = Some(pos.y - thumb.y);
                    shell.capture_event();
                    shell.request_redraw();
                } else if let Some(div_idx) =
                    hit_divider(pos, &div_xs, bounds, self.divider_grab_width)
                {
                    state.resizing = Some(ColumnResize {
                        column_index: div_idx,
                        start_cursor_x: pos.x,
                        start_width: widths[div_idx],
                    });
                    shell.capture_event();
                    shell.request_redraw();
                } else if let Some(row) = hit_row(
                    pos,
                    bounds,
                    self.header_height,
                    self.row_height,
                    n_rows,
                    state.scroll,
                ) {
                    state.selected_row = Some(row);
                    if let Some(ref on_select) = self.on_select {
                        shell.publish(on_select(row));
                    }
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            mouse::Event::ButtonReleased(mouse::Button::Left)
                if state.resizing.take().is_some() || state.scrollbar_drag.take().is_some() =>
            {
                shell.capture_event();
                shell.request_redraw();
            }
            _ => {}
        }
    }
    fn mouse_interaction(
        &self,
        tree: &Tree,
        _layout: layout::Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<TableState>();
        if state.resizing.is_some() || state.hovered_divider.is_some() {
            mouse::Interaction::ResizingHorizontally
        } else {
            mouse::Interaction::None
        }
    }
}

impl<'a, Message, Renderer> From<Table<'a, Message>> for Element<'a, Message, iced::Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + renderer::Renderer + text::Renderer,
{
    fn from(table: Table<'a, Message>) -> Self {
        Self::new(table)
    }
}

fn effective_widths(bounds: Rectangle, col_widths: &[f32], last_min: f32) -> Vec<f32> {
    let n = col_widths.len();
    if n == 0 {
        return vec![];
    };
    let mut widths = col_widths.to_vec();
    let sum_except_last: f32 = widths[..n - 1].iter().sum();
    widths[n - 1] = (bounds.width - sum_except_last).max(last_min);
    widths
}
fn col_start_xs(bounds: Rectangle, widths: &[f32]) -> Vec<f32> {
    let mut x = bounds.x;
    widths
        .iter()
        .map(|&w| {
            let start = x;
            x += w;
            start
        })
        .collect()
}
fn divider_xs(bounds: Rectangle, widths: &[f32]) -> Vec<f32> {
    let mut x = bounds.x;
    widths
        .iter()
        .take(widths.len().saturating_sub(1))
        .map(|&w| {
            x += w;
            x
        })
        .collect()
}
fn body_bounds(bounds: Rectangle, header_h: f32) -> Rectangle {
    Rectangle {
        y: bounds.y + header_h,
        height: (bounds.height - header_h).max(0.0),
        ..bounds
    }
}

fn max_scroll(bounds: Rectangle, header_h: f32, row_h: f32, n_rows: usize) -> f32 {
    (n_rows as f32 * row_h - body_bounds(bounds, header_h).height).max(0.0)
}

fn thumb_bounds(
    bounds: Rectangle,
    header_h: f32,
    row_h: f32,
    n_rows: usize,
    scroll: f32,
) -> Option<Rectangle> {
    let body = body_bounds(bounds, header_h);
    let content_h = n_rows as f32 * row_h;
    if content_h <= body.height {
        return None;
    }
    let track_h = body.height - 2.0 * SCROLLBAR_PAD;
    let thumb_h = (track_h * body.height / content_h).max(MIN_THUMB_H);
    let progress = (scroll / (content_h - body.height)).clamp(0.0, 1.0);
    Some(Rectangle {
        x: bounds.x + bounds.width - SCROLLBAR_PAD - SCROLLBAR_W,
        y: body.y + SCROLLBAR_PAD + progress * (track_h - thumb_h),
        width: SCROLLBAR_W,
        height: thumb_h,
    })
}

fn hit_row(
    pos: Point,
    bounds: Rectangle,
    header_h: f32,
    row_h: f32,
    n_rows: usize,
    scroll: f32,
) -> Option<usize> {
    let body = body_bounds(bounds, header_h);
    if !body.contains(pos) {
        return None;
    }
    let rel_y = pos.y - body.y + scroll;
    if rel_y < 0.0 {
        return None;
    }
    let row = (rel_y / row_h) as usize;
    if row < n_rows { Some(row) } else { None }
}
fn hit_divider(pos: Point, div_xs: &[f32], bounds: Rectangle, grab_w: f32) -> Option<usize> {
    if pos.y < bounds.y || pos.y > bounds.y + bounds.height {
        return None;
    }

    div_xs
        .iter()
        .position(|&dx| (pos.x - dx).abs() <= grab_w / 2.0)
}
