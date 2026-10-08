use crate::{
    ExportAction, annotation::DEFAULT_STROKE_COLOR, message::Message, stroke_width::StrokeWidth,
    tool::Tool,
};
use iced::Font;
use iced::widget::{button, container, pick_list, rule, text, tooltip};
use iced::{
    Color, Element, Fill, Theme,
    alignment::{Horizontal, Vertical},
    widget::Row,
};
use lucide_icons::Icon;
use std::fmt::Display;
use strum::VariantArray;

const ICON_FONT: Font = Font::with_name("lucide");

/// Builds the editor's drawing, history, appearance, and export controls
pub fn view(
    selected: Tool,
    stroke_width: StrokeWidth,
    stroke_color: Color,
    can_undo: bool,
    can_redo: bool,
) -> Element<'static, Message> {
    let tools = Tool::VARIANTS.iter().map(|&tool| {
        icon_button(
            tool.icon(),
            tool.to_string(),
            Some(Message::SelectTool(tool)),
            tool == selected,
        )
    });

    let controls = Row::with_children(tools)
        .spacing(4)
        .align_y(Vertical::Center)
        .push(separator())
        .push(icon_button(
            Icon::Undo2,
            "Undo (Ctrl+Z)".into(),
            can_undo.then_some(Message::Undo),
            false,
        ))
        .push(icon_button(
            Icon::Redo2,
            "Redo (Ctrl+Y / Ctrl+Shift+Z)".into(),
            can_redo.then_some(Message::Redo),
            false,
        ))
        .push(separator())
        .push(stroke_color_controls(stroke_color))
        .push(stroke_width_controls(stroke_width))
        .push(icon_button(
            Icon::Copy,
            "Copy (Ctrl+C)".into(),
            Some(Message::Export(ExportAction::Copy)),
            false,
        ))
        .push(icon_button(
            Icon::Save,
            "Save (Ctrl+S)".into(),
            Some(Message::Export(ExportAction::Save)),
            false,
        ));

    container(controls)
        .padding(4)
        .width(Fill)
        .style(container::rounded_box)
        .into()
}

fn stroke_width_controls(selected: StrokeWidth) -> Element<'static, Message> {
    pick_list(
        StrokeWidth::VARIANTS,
        Some(selected),
        Message::SelectStrokeWidth,
    )
    .width(76)
    .text_size(14)
    .padding([9, 10])
    .into()
}

fn stroke_color_controls(selected: Color) -> Element<'static, Message> {
    let colors = [
        ColorPreset::new("Red", DEFAULT_STROKE_COLOR),
        ColorPreset::new("Blue", Color::from_rgb8(64, 128, 255)),
        ColorPreset::new("Green", Color::from_rgb8(64, 200, 96)),
        ColorPreset::new("Yellow", Color::from_rgb8(255, 208, 64)),
        ColorPreset::new("White", Color::WHITE),
        ColorPreset::new("Black", Color::BLACK),
    ];

    let selected = colors
        .iter()
        .copied()
        .find(|preset| preset.color == selected);

    pick_list(colors, selected, |preset| {
        Message::SelectStrokeColor(preset.color)
    })
    .placeholder("Color")
    .width(104)
    .text_size(14)
    .padding([9, 10])
    .into()
}

fn icon_button(
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

fn button_style(theme: &Theme, status: button::Status, selected: bool) -> button::Style {
    if selected {
        return button::primary(theme, status);
    }
    button::text(theme, status)
}

fn separator() -> Element<'static, Message> {
    container(rule::vertical(1))
        .height(24)
        .padding([0, 6])
        .into()
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ColorPreset {
    label: &'static str,
    color: Color,
}

impl ColorPreset {
    const fn new(label: &'static str, color: Color) -> Self {
        Self { label, color }
    }
}

impl Display for ColorPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label)
    }
}
