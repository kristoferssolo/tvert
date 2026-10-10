use crate::editor::Message;
use iced::Element;
use iced::{
    alignment::Vertical,
    widget::{container, row, slider, text, tooltip},
};
use std::ops::RangeInclusive;

pub(super) fn view(
    range: RangeInclusive<u16>,
    value: u16,
    default: u16,
    label: &'static str,
    on_change: fn(u16) -> Message,
) -> Element<'static, Message> {
    let control = row![
        slider(range, value, on_change)
            .step(1u16)
            .default(default)
            .width(92),
        text(format!("{value} px")).size(14).width(44)
    ]
    .spacing(8)
    .align_y(Vertical::Center);

    tooltip(
        container(control).padding([0, 8]).center_y(36),
        text(format!("{label} in image pixels. Ctrl+click to reset")).size(12),
        tooltip::Position::Bottom,
    )
    .gap(6)
    .style(container::rounded_box)
    .into()
}
