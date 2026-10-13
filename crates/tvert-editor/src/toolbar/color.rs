use crate::{
    color::parse_hex,
    editor::Message,
    style::RED,
    toolbar::{ICON_FONT, button::button_style, input},
};
use iced::widget::{button, column, container, row, space, text, text_input};
use iced::{Border, Color, Element, Fill, Theme, alignment::Vertical, widget::Row};
use iced_aw::{DropDown, drop_down};
use iced_color_triangle::{Selection, color_triangle};
use lucide_icons::Icon;

const COLORS: &[Color] = &[
    RED,
    Color::from_rgb8(64, 128, 255),
    Color::from_rgb8(64, 200, 96),
    Color::from_rgb8(255, 208, 64),
    Color::WHITE,
    Color::BLACK,
];

pub(super) fn view(selection: Selection, open: bool, hex: &str) -> Element<'static, Message> {
    let selected = Color::from(selection);
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
    .on_press(Message::ToggleColorPalette)
    .style(|theme, status| button_style(theme, status, false));

    DropDown::new(trigger, picker(selection, hex), open)
        .width(264)
        .alignment(drop_down::Alignment::Bottom)
        .on_dismiss(Message::CloseColorPalette)
        .into()
}

fn picker(selection: Selection, hex: &str) -> Element<'static, Message> {
    container(
        column![
            presets(selection.into()),
            color_triangle(selection, Message::SelectStrokeSelection),
            hex_input(selection, hex),
        ]
        .spacing(8)
        .width(240),
    )
    .padding(12)
    .style(container::rounded_box)
    .into()
}

fn presets(selected: Color) -> Element<'static, Message> {
    Row::with_children(COLORS.iter().copied().map(|color| {
        button(color_swatch(color))
            .width(36)
            .height(36)
            .padding(8)
            .on_press(Message::SelectStrokeColor(color))
            .style(move |theme, status| {
                if color.into_rgba8() == selected.into_rgba8() {
                    button::primary(theme, status)
                } else {
                    button::background(theme, status)
                }
            })
            .into()
    }))
    .spacing(4)
    .into()
}

fn hex_input(selection: Selection, hex: &str) -> Element<'static, Message> {
    let input = text_input("#RRGGBB", hex)
        .on_input(Message::EditColorHex)
        .on_paste(Message::EditColorHex)
        .padding(8)
        .width(Fill)
        .style(input::style(parse_hex(hex).is_some()));

    row![color_swatch(selection.into()), input::owns_keyboard(input)]
        .spacing(8)
        .align_y(Vertical::Center)
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
