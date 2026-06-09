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
#[derive(Clone)]
struct TableState {
    column_widths: Vec<f32>,
    selected_row: Option<usize>,
    hovered_row: Option<usize>,
    hovered_divider: Option<usize>,
    resizing: Option<ColumnResize>,
}
impl TableState {
    pub fn new() -> Self {
        Self {
            column_widths: vec![],
            selected_row: None,
            hovered_row: None,
            hovered_divider: None,
            resizing: None,
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
        _cursor: iced::advanced::mouse::Cursor,
        _viewport: &iced::Rectangle,
    ) {
        let state = tree.state.downcast_ref::<TableState>();
        let bounds = layout.bounds();

        let widths = effective_widths(bounds, &state.column_widths);
        let col_xs = col_start_xs(bounds, &widths);
        let div_xs = divider_xs(bounds, &widths);
        let total_h = self.header_height + self.rows.len() as f32 * self.row_height;

        renderer.fill_quad(
            renderer::Quad {
                bounds,
                // border: Border {
                //     width: 4.0,
                //     color: theme.extended_palette().primary.base.color,
                //     ..Default::default()
                // },
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
        for (row_idx, row) in self.rows.iter().enumerate() {
            let row_y = bounds.y + self.header_height + row_idx as f32 * self.row_height;
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
                        height: total_h,
                    },
                    ..Default::default()
                },
                color,
            );
        }
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

        let widths = effective_widths(bounds, &state.column_widths);
        let div_xs = divider_xs(bounds, &widths);

        match mouse_event {
            mouse::Event::CursorMoved { .. } => {
                let Some(pos) = cursor.position() else { return };
                if let Some(ref resize) = state.resizing.clone() {
                    let delta = pos.x - resize.start_cursor_x;
                    let min_w = self.columns[resize.column_index].min_width;
                    state.column_widths[resize.column_index] =
                        (resize.start_width + delta).max(min_w);
                    state.hovered_divider = Some(resize.column_index);
                    shell.capture_event();
                    shell.request_redraw();
                } else {
                    let new_row = hit_row(
                        pos,
                        bounds,
                        self.header_height,
                        self.row_height,
                        self.rows.len(),
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
            mouse::Event::ButtonPressed(mouse::Button::Left) => {
                let Some(pos) = cursor.position() else { return };
                if !bounds.contains(pos) {
                    return;
                }
                if let Some(div_idx) = hit_divider(pos, &div_xs, bounds, self.divider_grab_width) {
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
                    self.rows.len(),
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
                if state.resizing.take().is_some() =>
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

fn effective_widths(bounds: Rectangle, col_widths: &[f32]) -> Vec<f32> {
    let n = col_widths.len();
    if n == 0 {
        return vec![];
    };
    let mut widths = col_widths.to_vec();
    let sum_except_last: f32 = widths[..n - 1].iter().sum();
    widths[n - 1] = (bounds.width - sum_except_last).max(widths[n - 1]);
    widths
}
//X pos for pixels where the column starts
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
//X pos for divider of a column
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
//Which row is the pos at
fn hit_row(
    pos: Point,
    bounds: Rectangle,
    header_h: f32,
    row_h: f32,
    n_rows: usize,
) -> Option<usize> {
    if pos.x < bounds.x || pos.x > bounds.x + bounds.width {
        return None;
    }
    let rel_y = pos.y - (bounds.y + header_h);
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
// impl<'a, Message: 'a, Renderer> From<Table<'a, Message>>
//     for Element<'a, Message, iced::Theme, Renderer>
// where
//     Renderer: renderer::Renderer,
// {
//     fn from(table: Table<'a, Message>) -> Self {
//     }
// }
