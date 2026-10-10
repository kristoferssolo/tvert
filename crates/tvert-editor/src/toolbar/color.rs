use crate::{editor::Message, style::DEFAULT_STROKE_COLOR, toolbar::ICON_FONT};
use iced::widget::{Column, button, container, row, space, text};
use iced::{Border, Color, Element, Theme, alignment::Vertical, widget::Row};
use iced_aw::{DropDown, drop_down};
use lucide_icons::Icon;

const COLORS: &[Color] = &[
    DEFAULT_STROKE_COLOR,
    Color::from_rgb8(64, 128, 255),
    Color::from_rgb8(64, 200, 96),
    Color::from_rgb8(255, 208, 64),
    Color::WHITE,
    Color::BLACK,
];

pub(super) fn view(selected: Color, open: bool) -> Element<'static, Message> {
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
    .style(button::text);

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
