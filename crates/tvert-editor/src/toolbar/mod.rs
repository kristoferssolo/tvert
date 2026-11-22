mod button;
mod color;
mod input;
mod size;

pub use color::for_tool as color_control;
pub use size::for_tool as size_control;

use crate::{
    ExportAction,
    editor::{
        Message,
        shortcuts::{CROP_KEY, SELECT_KEY, tool_key},
    },
    export::ExportBindings,
    mode::Mode,
    tool::Tool,
    toolbar::button::{icon_button, separator},
    viewport::Navigation,
};
use iced::widget::container;
use iced::{Element, Fill, Font, alignment::Vertical, widget::Row};
use lucide_icons::Icon;
use strum::VariantArray;

const ICON_FONT: Font = Font::with_name("lucide");

pub const HEIGHT: f32 = 44.0;
pub const MIN_WIDTH: f32 = 800.0;

/// Toolbar actions that currently apply; the others render disabled
#[derive(Debug, Clone, Copy)]
pub struct Actions {
    pub undo: bool,
    pub redo: bool,
    pub reset_crop: bool,
}

/// Builds the editor's drawing, history, appearance, and export controls
pub fn view(
    selected: Option<Tool>,
    mode: Mode,
    color_control: Option<Element<'static, Message>>,
    size_control: Option<Element<'static, Message>>,
    actions: Actions,
    exports: ExportBindings,
) -> Element<'static, Message> {
    let select = icon_button(
        Icon::MousePointer2,
        format!("Select ({SELECT_KEY})"),
        Some(Message::SelectionMode),
        matches!(mode, Mode::Select(_)),
    );
    let crop = icon_button(
        Icon::Crop,
        format!("Crop ({CROP_KEY}, Enter to apply)"),
        Some(Message::CropMode),
        matches!(mode, Mode::Crop(_)),
    );
    let tools = Tool::VARIANTS.iter().map(|&tool| {
        icon_button(
            tool_icon(tool),
            format!("{tool} ({})", tool_key(tool)),
            Some(Message::SelectTool(tool)),
            Some(tool) == selected,
        )
    });

    let controls = Row::with_children([select, crop].into_iter().chain(tools))
        .spacing(4)
        .align_y(Vertical::Center)
        .push(separator())
        .push(icon_button(
            Icon::Undo2,
            "Undo (Ctrl+Z)".into(),
            actions.undo.then_some(Message::Undo),
            false,
        ))
        .push(icon_button(
            Icon::Redo2,
            "Redo (Ctrl+Y / Ctrl+Shift+Z)".into(),
            actions.redo.then_some(Message::Redo),
            false,
        ))
        .push(icon_button(
            Icon::Maximize,
            "Fit image (scroll to zoom, middle-drag to pan)".into(),
            Some(Message::Navigate(Navigation::Fit)),
            false,
        ))
        .push(icon_button(
            Icon::Expand,
            "Reset crop".into(),
            actions.reset_crop.then_some(Message::ResetCrop),
            false,
        ))
        .push(separator())
        .extend(color_control)
        .extend(size_control)
        .extend(ExportAction::VARIANTS.iter().map(|&action| {
            icon_button(
                export_icon(action),
                export_tooltip(action, exports),
                Some(Message::Export(action)),
                false,
            )
        }));

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
        Tool::Text => Icon::Type,
        Tool::Pixelate => Icon::Grid2x2,
        Tool::Blur => Icon::CloudFog,
    }
}

const fn export_icon(action: ExportAction) -> Icon {
    match action {
        ExportAction::Copy => Icon::Copy,
        ExportAction::Save => Icon::Save,
        ExportAction::CopyAndSave => Icon::ClipboardCopy,
    }
}

fn export_tooltip(action: ExportAction, exports: ExportBindings) -> String {
    let name = match action {
        ExportAction::Copy => "Copy",
        ExportAction::Save => "Save",
        ExportAction::CopyAndSave => "Copy and save",
    };
    format!("{name} ({})", exports.get(action))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_tooltips_show_each_configured_binding_canonically() {
        let exports = ExportBindings::parse(" alt + 1 ", "super+ctrl+s", "Shift+Ctrl+x");
        let tooltips = ExportAction::VARIANTS
            .iter()
            .map(|&action| export_tooltip(action, exports))
            .collect::<Vec<_>>();

        assert_eq!(
            tooltips,
            [
                "Copy (Alt+1)",
                "Save (Ctrl+Super+S)",
                "Copy and save (Ctrl+Shift+X)"
            ]
        );
    }
}
