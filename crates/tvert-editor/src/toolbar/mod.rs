mod button;
mod color;
mod size;

use crate::{
    ExportAction,
    editor::Message,
    style::{
        BLOCK_SIZE_RANGE, BLUR_SIGMA_RANGE, DEFAULT_BLOCK_SIZE, DEFAULT_BLUR_SIGMA,
        DEFAULT_STROKE_WIDTH, STROKE_WIDTH_RANGE,
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
    selected: Tool,
    selected_size: u16,
    stroke_selection: Selection,
    color_palette_open: bool,
    color_hex: &str,
    can_undo: bool,
    can_redo: bool,
) -> Element<'static, Message> {
    let tools = Tool::VARIANTS.iter().map(|&tool| {
        icon_button(
            tool_icon(tool),
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
        .extend(
            (!matches!(selected, Tool::Pixelate | Tool::Blur))
                .then(|| color::view(stroke_selection, color_palette_open, color_hex)),
        )
        .extend(match selected {
            Tool::Pixelate => Some(size::view(
                BLOCK_SIZE_RANGE,
                selected_size,
                DEFAULT_BLOCK_SIZE,
                "Pixelation block size",
                Message::SelectBlockSize,
            )),
            Tool::Blur => Some(size::view(
                BLUR_SIGMA_RANGE,
                selected_size,
                DEFAULT_BLUR_SIGMA,
                "Blur strength",
                Message::SelectBlurSigma,
            )),
            Tool::Highlighter => None,
            _ => Some(size::view(
                STROKE_WIDTH_RANGE,
                selected_size,
                DEFAULT_STROKE_WIDTH,
                "Stroke width",
                Message::SelectStrokeWidth,
            )),
        })
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
