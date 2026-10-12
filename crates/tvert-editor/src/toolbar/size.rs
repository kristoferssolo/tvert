use crate::{
    Settings,
    editor::{Message, SizeMessage},
    size::{SizeKind, Sizes},
    tool::Tool,
    toolbar::input as input_,
};
use iced::{
    Element,
    alignment::{Horizontal, Vertical},
    widget::{container, row, slider, text, text_input, tooltip},
};

pub fn for_tool(
    tool: Tool,
    value: u16,
    input: Option<&str>,
    defaults: Settings,
) -> Option<Element<'static, Message>> {
    if tool == Tool::Highlighter {
        return None;
    }
    let kind = tool.size_kind();
    Some(view(kind, value, input, Sizes::from(defaults).get(kind)))
}

fn view(
    kind: SizeKind,
    value: u16,
    input: Option<&str>,
    default: u16,
) -> Element<'static, Message> {
    let on_change = move |size| Message::Size(SizeMessage::Select(kind, size));
    let control = row![
        slider(kind.range(), value, on_change)
            .step(1u16)
            .default(default)
            .width(92),
        size_input(kind, value, input)
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

fn size_input(kind: SizeKind, value: u16, input: Option<&str>) -> Element<'static, Message> {
    let value = value.to_string();
    let input = input.unwrap_or(&value);
    let field = text_input(&value, input)
        .on_input(move |input| Message::Size(SizeMessage::Edit(kind, input)))
        .on_submit(Message::Size(SizeMessage::Submit))
        .size(14)
        .padding([4, 6])
        .width(44)
        .align_x(Horizontal::Center)
        .style(input_::style(kind.parse(input).is_some()));
    input_::owns_keyboard(field)
}
