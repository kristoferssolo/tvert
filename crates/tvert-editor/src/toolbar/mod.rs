mod button;
mod color;
mod input;
mod size;

use std::iter;

pub use size::for_tool as size_control;

use crate::{
    ExportAction,
    editor::{
        Message,
        shortcuts::{SELECT_KEY, tool_key},
    },
    tool::Tool,
    toolbar::button::{icon_button, separator},
};
use iced::widget::container;
use iced::{Element, Fill, Font, alignment::Vertical, widget::Row};
use iced_color_triangle::Selection;
use lucide_icons::Icon;
use strum::VariantArray;

const ICON_FONT: Font = Font::with_name("lucide");

pub const HEIGHT: f32 = 44.0;
pub const MIN_WIDTH: f32 = 800.0;

/// Builds the editor's drawing, history, appearance, and export controls
pub fn view(
    selected: Option<Tool>,
    size_control: Option<Element<'static, Message>>,
    stroke_selection: Selection,
    color_palette_open: bool,
    color_hex: &str,
    can_undo: bool,
    can_redo: bool,
) -> Element<'static, Message> {
    let select = icon_button(
        Icon::MousePointer2,
        format!("Select ({SELECT_KEY})"),
        Some(Message::SelectionMode),
        selected.is_none(),
    );
    let tools = Tool::VARIANTS.iter().map(|&tool| {
        icon_button(
            tool_icon(tool),
            format!("{tool} ({})", tool_key(tool)),
            Some(Message::SelectTool(tool)),
            Some(tool) == selected,
        )
    });

    let controls = Row::with_children(iter::once(select).chain(tools))
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
        .extend(
            selected
                .is_some_and(|tool| !matches!(tool, Tool::Pixelate | Tool::Blur))
                .then(|| color::view(stroke_selection, color_palette_open, color_hex)),
        )
        .extend(size_control)
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
        ))
        .push(icon_button(
            Icon::ClipboardCopy,
            "Copy and save (Ctrl+Shift+C)".into(),
            Some(Message::Export(ExportAction::CopyAndSave)),
            false,
        ));

    container(controls)
        .padding(4)
        .height(HEIGHT)
        .center_x(Fill)
        .into()
}

const fn tool_icon(tool: Tool) -> Icon {
    match tool {
        Tool::Rectangle => Icon::Square,
        Tool::Ellipse => Icon::Circle,
        Tool::Line => Icon::Minus,
        Tool::Arrow => Icon::ArrowRight,
        Tool::Pencil => Icon::Pencil,
        Tool::Highlighter => Icon::Highlighter,
        Tool::Pixelate => Icon::Grid2x2,
        Tool::Blur => Icon::CloudFog,
    }
}
