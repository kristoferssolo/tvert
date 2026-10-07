use crate::{message::Message, tool::Tool, viewport::Viewport};
use iced::widget::{button, canvas, container, space, text, tooltip};
use iced::{
    Border, Color, Element, Point, Renderer, Size, Theme,
    alignment::Vertical,
    mouse,
    widget::{
        Row,
        canvas::{
            Frame, LineCap, LineJoin, Path, Program, Stroke, path::lyon_path::math::Transform,
        },
    },
};
use strum::VariantArray;

const ICON_SIZE: f32 = 24.0;

/// Builds the tool selector and history controls
pub fn view(selected: Tool, can_undo: bool, can_redo: bool) -> Element<'static, Message> {
    let tools = Tool::VARIANTS.iter().map(|&tool| {
        icon_button(
            tool_path(tool),
            tool.to_string(),
            Some(Message::SelectTool(tool)),
            tool == selected,
        )
    });

    let separator = container(space())
        .width(1)
        .height(24)
        .style(|_| container::Style {
            background: Some(Color::from_rgb8(58, 57, 66).into()),
            ..Default::default()
        });

    let controls = Row::with_children(tools)
        .spacing(4)
        .align_y(Vertical::Center)
        .push(container(separator).padding([0, 6]))
        .push(icon_button(
            undo_path(),
            "Undo (Ctrl+Z)".into(),
            can_undo.then_some(Message::Undo),
            false,
        ))
        .push(icon_button(
            undo_path().transform(&Transform::new(-1.0, 0.0, 0.0, 1.0, ICON_SIZE, 0.0)),
            "Redo (Ctrl+Y / Ctrl+Shift+Z)".into(),
            can_redo.then_some(Message::Redo),
            false,
        ));

    container(controls)
        .padding(4)
        .style(|_| container::Style {
            background: Some(Color::from_rgb8(35, 35, 40).into()),
            border: Border {
                radius: 10.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

fn icon_button(
    path: Path,
    label: String,
    action: Option<Message>,
    selected: bool,
) -> Element<'static, Message> {
    let icon = canvas(Icon {
        path,
        enabled: action.is_some(),
    })
    .width(ICON_SIZE)
    .height(ICON_SIZE);

    let control = button(icon)
        .width(40)
        .height(40)
        .padding(8)
        .on_press_maybe(action)
        .style(move |theme, status| button_style(theme, status, selected));

    tooltip(control, text(label).size(12), tooltip::Position::Bottom)
        .gap(8)
        .style(container::rounded_box)
        .into()
}

fn button_style(theme: &Theme, status: button::Status, selected: bool) -> button::Style {
    let background = match (selected, status) {
        (true, button::Status::Hovered) => Some(Color::from_rgb8(82, 76, 128).into()),
        (true, button::Status::Pressed) => Some(Color::from_rgb8(61, 56, 100).into()),
        (true, _) => Some(Color::from_rgb8(70, 65, 112).into()),
        (false, button::Status::Hovered | button::Status::Pressed) => {
            Some(Color::from_rgb8(52, 51, 60).into())
        }
        (false, _) => None,
    };

    button::Style {
        background,
        text_color: theme.palette().text,
        border: Border {
            radius: 6.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn tool_path(tool: Tool) -> Path {
    let size = Size::new(ICON_SIZE, ICON_SIZE);
    let Some(viewport) = Viewport::new(size, size) else {
        return Path::new(|_| {});
    };

    let (start, end) = match tool {
        Tool::Rectangle | Tool::Ellipse => (Point::new(5.0, 5.0), Point::new(19.0, 19.0)),
        Tool::Line | Tool::Arrow => (Point::new(4.0, 12.0), Point::new(20.0, 12.0)),
    };

    tool.figure(start, end)
        .map_or_else(|| Path::new(|_| {}), |figure| figure.path(viewport))
}

fn undo_path() -> Path {
    Path::new(|builder| {
        builder.move_to(Point::new(5.0, 9.0));
        builder.line_to(Point::new(14.0, 9.0));
        builder.bezier_curve_to(
            Point::new(21.0, 9.0),
            Point::new(21.0, 19.0),
            Point::new(14.0, 19.0),
        );
        builder.move_to(Point::new(9.0, 5.0));
        builder.line_to(Point::new(5.0, 9.0));
        builder.line_to(Point::new(9.0, 13.0));
    })
}

struct Icon {
    path: Path,
    enabled: bool,
}

impl Program<Message> for Icon {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<iced::widget::canvas::Geometry<Renderer>> {
        let mut frame = Frame::new(renderer, bounds.size());
        let color = if self.enabled {
            theme.palette().text
        } else {
            Color::from_rgb8(104, 104, 116)
        };

        frame.stroke(
            &self.path,
            Stroke::default()
                .with_color(color)
                .with_width(1.5)
                .with_line_cap(LineCap::Round)
                .with_line_join(LineJoin::Round),
        );

        vec![frame.into_geometry()]
    }
}
