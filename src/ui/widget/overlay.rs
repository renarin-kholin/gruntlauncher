use iced::{
    Border, Element, Length, Theme, alignment,
    border::Radius,
    padding,
    widget::{column, container, opaque, stack, text},
};

use crate::ui::theme::grunt_theme;

pub fn overlay_container<'a, TMessage: 'a>(
    base: Element<'a, TMessage>,
    panel_children: Option<Element<'a, TMessage>>,
    panel_title: Option<String>,
) -> Element<'a, TMessage> {
    let mut container_stack = stack![base,];
    if let (Some(children), Some(title)) = (panel_children, panel_title) {
        container_stack = container_stack.push(opaque(
            container(overlay_pane(children, title))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(padding::all(40.0))
                .style(|_theme| container::Style {
                    background: Some(iced::Background::Color(iced::Color {
                        a: 0.8,
                        ..iced::Color::BLACK
                    })),
                    ..Default::default()
                })
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center),
        ))
    }
    container_stack.into()
}

fn overlay_pane<'a, TMessage: 'a>(
    children: Element<'a, TMessage>,
    title: String,
) -> Element<'a, TMessage> {
    container(column![
        container(
            text!("{}", title).color(grunt_theme().extended_palette().secondary.strong.color)
        )
        .padding(padding::vertical(5.0).horizontal(15.0))
        .height(Length::Shrink)
        .style(|theme: &Theme| container::Style {
            background: Some(theme.extended_palette().background.weaker.color.into()),
            border: Border {
                radius: Radius::default().bottom(0.0).top(4.0),
                color: theme.extended_palette().background.weak.color,
                width: 1.0,
            },
            ..container::rounded_box(theme)
        })
        .width(Length::Fill),
        children
    ])
    .width(Length::Fill)
    .height(Length::Fill)
    .style(container::bordered_box)
    .into()
}
