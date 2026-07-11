use iced::advanced::text::{self, Text};
use iced::{Color, Pixels, Point, Rectangle, Size, alignment};

pub(crate) fn fill_label<Renderer: text::Renderer>(
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

pub(crate) fn estimate_width(s: &str, size: f32) -> f32 {
    s.chars().count() as f32 * size * 0.58
}
