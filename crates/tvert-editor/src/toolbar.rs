use crate::{
    ExportAction, annotation::DEFAULT_STROKE_COLOR, message::Message, stroke_width::StrokeWidth,
    tool::Tool,
};
use iced::Pixels;
use iced::widget::text::{LineHeight, Shaping};
use iced::widget::{Column, button, container, pick_list, row, rule, space, text, tooltip};
use iced::{
    Border, Color, Element, Fill, Font, Theme,
    alignment::{Horizontal, Vertical},
    widget::Row,
};
use iced_aw::{DropDown, drop_down};
use lucide_icons::Icon;
use strum::VariantArray;

const COLORS: &[Color] = &[
    DEFAULT_STROKE_COLOR,
    Color::from_rgb8(64, 128, 255),
    Color::from_rgb8(64, 200, 96),
    Color::from_rgb8(255, 208, 64),
    Color::WHITE,
    Color::BLACK,
];

const ICON_FONT: Font = Font::with_name("lucide");

pub const HEIGHT: f32 = 44.0;
pub const MIN_WIDTH: f32 = 640.0;

/// Builds the editor's drawing, history, appearance, and export controls
pub fn view(
    selected: Tool,
    stroke_width: StrokeWidth,
    stroke_color: Color,
    color_pallete_open: bool,
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
        .push(stroke_color_controls(stroke_color, color_pallete_open))
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

fn stroke_width_controls(selected: StrokeWidth) -> Element<'static, Message> {
    pick_list(
        StrokeWidth::VARIANTS,
        Some(selected),
        Message::SelectStrokeWidth,
    )
    .width(76)
    .text_size(14)
    .text_line_height(Pixels(18.0))
    .handle(pick_list::Handle::Static(pick_list::Icon {
        font: ICON_FONT,
        code_point: char::from(Icon::ChevronDown),
        size: Some(Pixels(14.0)),
        line_height: LineHeight::default(),
        shaping: Shaping::default(),
    }))
    .padding([9, 10])
    .into()
}

fn stroke_color_controls(selected: Color, open: bool) -> Element<'static, Message> {
    let trigger = button(
        row![
            color_swatch(selected),
            text(char::from(Icon::ChevronDown).to_string())
                .font(ICON_FONT)
                .size(14)
        ]
        .spacing(6)
        .align_y(Vertical::Center),
    )
    .width(60)
    .height(36)
    .padding(8)
    .on_press(Message::ToggleColorPallete)
    .style(move |theme, status| color_trigger_style(theme, status, open));

    let rows = COLORS.chunks(3).map(|colors| {
        Row::with_children(colors.iter().copied().map(|color| {
            button(color_swatch(color))
                .width(36)
                .height(36)
                .padding(8)
                .on_press(Message::SelectStrokeColor(color))
                .style(move |theme, status| {
                    if color == selected {
                        button::primary(theme, status)
                    } else {
                        button::background(theme, status)
                    }
                })
                .into()
        }))
        .spacing(4)
        .into()
    });

    let palette = container(Column::with_children(rows).spacing(4))
        .padding(8)
        .style(container::rounded_box);

    DropDown::new(trigger, palette, open)
        .width(132)
        .alignment(drop_down::Alignment::Bottom)
        .on_dismiss(Message::CloseColorPallete)
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

fn color_swatch(color: Color) -> Element<'static, Message> {
    container(space())
        .width(20)
        .height(20)
        .style(move |theme: &Theme| container::Style {
            background: Some(color.into()),
            border: Border {
                color: theme.palette().text.scale_alpha(0.5),
                width: 1.0,
                radius: 3.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn color_trigger_style(theme: &Theme, status: button::Status, open: bool) -> button::Style {
    let is_hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);

    let status = if open {
        pick_list::Status::Opened { is_hovered }
    } else if is_hovered {
        pick_list::Status::Hovered
    } else {
        pick_list::Status::Active
    };
    let style = pick_list::default(theme, status);

    button::Style {
        background: Some(style.background),
        text_color: style.handle_color,
        border: style.border,
        ..Default::default()
    }
}
