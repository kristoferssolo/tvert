use crate::Settings;
use crate::editor::Message;
use crate::style::{BLOCK_SIZE_RANGE, BLUR_SIGMA_RANGE, STROKE_WIDTH_RANGE};
use crate::tool::Tool;
use iced::Element;
use iced::{
    alignment::Vertical,
    widget::{container, row, slider, text, tooltip},
};
use std::ops::RangeInclusive;

pub fn for_tool(tool: Tool, value: u16, defaults: Settings) -> Option<Element<'static, Message>> {
    let (range, default, label, on_change): (_, _, _, fn(u16) -> Message) = match tool {
        Tool::Pixelate => (
            BLOCK_SIZE_RANGE,
            defaults.pixelation_block_size,
            "Pixelation block size",
            Message::SelectBlockSize,
        ),
        Tool::Blur => (
            BLUR_SIGMA_RANGE,
            defaults.blur_sigma,
            "Blur strenght",
            Message::SelectBlurSigma,
        ),
        Tool::Highlighter => return None,
        _ => (
            STROKE_WIDTH_RANGE,
            defaults.stroke_width,
            "Stroke width",
            Message::SelectStrokeWidth,
        ),
    };
    Some(view(range, value, default, label, on_change))
}

fn view(
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
