use crate::toolbar::ICON_FONT;
use crate::{editor::Message, toolbar::button::button_style};
use iced::widget::pick_list;
use iced::{
    Color, Element, Pixels, Theme,
    widget::{
        button::Status as ButtonStatus,
        text::{LineHeight, Shaping},
    },
};
use lucide_icons::Icon;
use std::fmt::Display;

pub(super) fn view<T>(
    options: &'static [T],
    selected: T,
    on_select: fn(T) -> Message,
) -> Element<'static, Message>
where
    T: Copy + Display + PartialEq + 'static,
{
    pick_list(options, Some(selected), on_select)
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
        .style(picker_style)
        .into()
}

fn picker_style(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let (status, open) = match status {
        pick_list::Status::Active => (ButtonStatus::Active, false),
        pick_list::Status::Hovered => (ButtonStatus::Hovered, false),
        pick_list::Status::Opened { is_hovered } => (
            if is_hovered {
                ButtonStatus::Hovered
            } else {
                ButtonStatus::Active
            },
            true,
        ),
    };
    let style = button_style(theme, status, open);

    pick_list::Style {
        text_color: style.text_color,
        placeholder_color: style.text_color,
        handle_color: style.text_color,
        background: style
            .background
            .unwrap_or_else(|| Color::TRANSPARENT.into()),
        border: style.border,
    }
}
