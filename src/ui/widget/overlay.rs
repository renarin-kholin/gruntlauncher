use iced::{
    Border, Element, Length, Theme,
    alignment::{self, Vertical},
    border::Radius,
    padding,
    widget::{button, column, container, opaque, right, row, stack, text},
};

use crate::ui::theme::grunt_theme;

pub fn overlay_container<'a, TMessage: 'a + Clone>(
    base: Element<'a, TMessage>,
    panel_children: Option<Element<'a, TMessage>>,
    panel_title: Option<String>,
    on_close_maybe: Option<TMessage>,
) -> Element<'a, TMessage> {
    let mut container_stack = stack![base,];
    if let (Some(children), Some(title)) = (panel_children, panel_title) {
        container_stack = container_stack.push(opaque(
            container(overlay_pane(children, title, on_close_maybe))
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

fn overlay_pane<'a, TMessage: 'a + Clone>(
    children: Element<'a, TMessage>,
    title: String,
    on_close_maybe: Option<TMessage>,
) -> Element<'a, TMessage> {
    let mut top_bar =
        row![text!("{}", title).color(grunt_theme().extended_palette().secondary.strong.color)]
            .align_y(Vertical::Center);
    if let Some(on_close) = on_close_maybe {
        top_bar = top_bar.push(right(
            button("×")
                .on_press(on_close)
                .style(|theme, style| button::Style {
                    border: Border {
                        radius: Radius::new(50.0),
                        ..Default::default()
                    },

                    ..button::danger(theme, style)
                })
                .padding(padding::horizontal(11.0).vertical(5.0)),
        ));
    }

    container(column![
        container(top_bar)
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
