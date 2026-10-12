use crate::Settings;
use crate::editor::Message;
use crate::size::{SizeKind, Sizes};
use crate::tool::Tool;
use iced::Element;
use iced::{
    alignment::Vertical,
    widget::{container, row, slider, text, tooltip},
};

pub fn for_tool(tool: Tool, value: u16, defaults: Settings) -> Option<Element<'static, Message>> {
    if tool == Tool::Highlighter {
        return None;
    }
    let kind = tool.size_kind();
    Some(view(kind, value, Sizes::from(defaults).get(kind)))
}

fn view(kind: SizeKind, value: u16, default: u16) -> Element<'static, Message> {
    let on_change = move |size| Message::SelectSize(kind, size);
    let control = row![
        slider(kind.range(), value, on_change)
            .step(1u16)
            .default(default)
            .width(92),
        text(format!("{value} px")).size(14).width(44)
    ]
    .spacing(8)
    .align_y(Vertical::Center);

    tooltip(
        container(control).padding([0, 8]).center_y(36),
        text(format!(
            "{} in image pixels. Ctrl+click to reset",
            kind.label()
        ))
        .size(12),
        tooltip::Position::Bottom,
    )
    .gap(6)
    .style(container::rounded_box)
    .into()
}
