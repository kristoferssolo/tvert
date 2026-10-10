use crate::{editor::Message, toolbar::ICON_FONT};
use iced::widget::{button, container, rule, text, tooltip};
use iced::{
    Element, Fill, Theme,
    alignment::{Horizontal, Vertical},
};
use lucide_icons::Icon;

pub(super) fn icon_button(
    icon: Icon,
    label: String,
    action: Option<Message>,
    selected: bool,
) -> Element<'static, Message> {
    let icon = text(char::from(icon).to_string())
        .font(ICON_FONT)
        .size(20)
        .line_height(1.0)
        .width(Fill)
        .height(Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center);

    let control = button(icon)
        .width(36)
        .height(36)
        .padding(8)
        .on_press_maybe(action)
        .style(move |theme, status| button_style(theme, status, selected));

    tooltip(control, text(label).size(12), tooltip::Position::Bottom)
        .gap(6)
        .style(container::rounded_box)
        .into()
}

pub(super) fn separator() -> Element<'static, Message> {
    container(rule::vertical(1))
        .height(24)
        .padding([0, 6])
        .into()
}

pub(super) fn button_style(theme: &Theme, status: button::Status, selected: bool) -> button::Style {
    if selected {
        return button::primary(theme, status);
    }
    button::text(theme, status)
}
