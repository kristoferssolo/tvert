use crate::{editor::Message, style::DEFAULT_STROKE_COLOR, toolbar::ICON_FONT};
use iced::widget::{button, column, container, row, space, text};
use iced::{Border, Color, Element, Fill, Theme, alignment::Vertical, widget::Row};
use iced_aw::{DropDown, drop_down};
use iced_color_triangle::{Selection, color_triangle};
use lucide_icons::Icon;

const COLORS: &[Color] = &[
    DEFAULT_STROKE_COLOR,
    Color::from_rgb8(64, 128, 255),
    Color::from_rgb8(64, 200, 96),
    Color::from_rgb8(255, 208, 64),
    Color::WHITE,
    Color::BLACK,
];

pub(super) fn view(selected: Selection, draft: Option<Selection>) -> Element<'static, Message> {
    let selected = Color::from(selected);
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
    .style(button::text);

    let palette = draft.map_or_else(|| space().into(), |selection| picker(selected, selection));

    DropDown::new(trigger, palette, draft.is_some())
        .width(264)
        .alignment(drop_down::Alignment::Bottom)
        .on_dismiss(Message::CloseColorPalette)
        .into()
}

fn picker(selected: Color, selection: Selection) -> Element<'static, Message> {
    container(
        column![
            presets(selected),
            color_triangle(selection, Message::PreviewStrokeColor),
            preview(selection),
            row![
                space().width(Fill),
                button("Cancel")
                    .on_press(Message::CloseColorPalette)
                    .style(button::text),
                button("Apply").on_press(Message::ApplyStrokeColor)
            ],
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

fn preview(selection: Selection) -> Element<'static, Message> {
    let color = Color::from(selection);
    let [red, green, blue, _] = color.into_rgba8();
    row![
        color_swatch(color),
        text(format!("#{red:02X}{green:02X}{blue:02X}"))
    ]
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
