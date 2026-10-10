use iced::{
    Color, Element, Theme,
    widget::{column, container, row, slider, text},
};
use iced_color_triangle::{Selection, color_triangle};

struct Picker {
    selection: Selection,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    Pick(Selection),
}

impl Picker {
    fn new() -> Self {
        Self {
            selection: Color::from_rgb8(255, 64, 64).into(),
        }
    }

    const fn update(&mut self, Message::Pick(selection): Message) {
        self.selection = selection;
    }

    fn view(&self) -> Element<'_, Message> {
        let selection = self.selection;
        let [r, g, b, _] = Color::from(selection).into_rgba8();
        container(
            column![
                color_triangle(selection, Message::Pick),
                row![
                    text("Hue").width(80),
                    slider(0.0..=360.0, selection.hue(), move |hue| Message::Pick(
                        selection.with_hue(hue)
                    ))
                ],
                row![
                    text("Saturation").width(80),
                    slider(0.0..=1.0, selection.saturation(), move |value| {
                        Message::Pick(selection.with_saturation(value))
                    })
                    .step(0.01_f32)
                ],
                row![
                    text("Lightness").width(80),
                    slider(
                        0.0..=1.0,
                        selection.lightness(),
                        move |value| Message::Pick(selection.with_lightness(value))
                    )
                    .step(0.01_f32)
                ],
                text(format!("#{r:02X}{g:02X}{b:02X}")).color(Color::from(selection)),
            ]
            .spacing(8)
            .width(240),
        )
        .padding(12)
        .into()
    }
}

fn main() -> iced::Result {
    iced::application(Picker::new, Picker::update, Picker::view)
        .title("Triangle color picker")
        .theme(Theme::Dark)
        .window_size((264.0, 400.0))
        .run()
}
