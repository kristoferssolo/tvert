use crate::editor::Message;
use iced::{
    Element, Event, Renderer, Theme,
    advanced::{
        Layout, Widget,
        graphics::core::keyboard,
        widget::{Operation, Tree},
    },
    widget::{
        TextInput,
        text_input::{self, Status, Style},
    },
};

pub(super) fn style(valid: bool) -> impl Fn(&Theme, Status) -> Style {
    move |theme, status| {
        let mut style = text_input::default(theme, status);
        if !valid {
            style.border.color = theme.palette().danger;
        }
        style
    }
}

pub(super) fn owns_keyboard(input: TextInput<'static, Message>) -> Element<'static, Message> {
    Element::new(KeyboardOwner {
        input: input.into(),
    })
}

struct KeyboardOwner<'a> {
    input: Element<'a, Message>,
}

impl KeyboardOwner<'_> {
    fn is_focused(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer) -> bool {
        let mut probe = FocusProbe::default();
        self.input
            .as_widget_mut()
            .operate(tree, layout, renderer, &mut probe);
        probe.focused
    }
}

impl Widget<Message, Theme, Renderer> for KeyboardOwner<'_> {
    fn size(&self) -> iced::Size<iced::Length> {
        self.input.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        self.input.as_widget_mut().layout(tree, renderer, limits)
    }

    fn draw(
        &self,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
    ) {
        self.input
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn tag(&self) -> iced::advanced::widget::tree::Tag {
        self.input.as_widget().tag()
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        self.input.as_widget().state()
    }

    fn operate(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        self.input
            .as_widget_mut()
            .operate(tree, layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        event: &iced::Event,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        viewport: &iced::Rectangle,
    ) {
        self.input.as_widget_mut().update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );

        if matches!(event, Event::Keyboard(keyboard::Event::KeyPressed { .. }))
            && self.is_focused(tree, layout, renderer)
        {
            shell.capture_event();
        }
    }

    fn mouse_interaction(
        &self,
        tree: &iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
        renderer: &Renderer,
    ) -> iced::advanced::mouse::Interaction {
        self.input
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }
}

#[derive(Debug, Default)]
struct FocusProbe {
    focused: bool,
}

impl Operation for FocusProbe {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<()>)) {
        operate(self);
    }

    fn focusable(
        &mut self,
        _id: Option<&iced::widget::Id>,
        _bounds: iced::Rectangle,
        state: &mut dyn iced::advanced::widget::operation::Focusable,
    ) {
        self.focused |= state.is_focused();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::{Shell, clipboard, layout};
    use iced::keyboard::{Key, Location, Modifiers, key};
    use iced::{Font, Pixels, Point, event, widget::text_input};
    use iced::{Size, mouse};

    struct Harness {
        field: Element<'static, Message>,
        tree: Tree,
        node: layout::Node,
        renderer: Renderer,
    }

    impl Harness {
        fn new() -> Self {
            let renderer = Renderer::new(Font::DEFAULT, Pixels(14.0));
            let mut field =
                owns_keyboard(text_input("", "#FF4040").on_input(Message::EditColorHex));
            let mut tree = Tree::new(field.as_widget());
            let limits = layout::Limits::new(Size::ZERO, Size::new(120.0, 40.0));
            let node = field.as_widget_mut().layout(&mut tree, &renderer, &limits);

            Self {
                field,
                tree,
                node,
                renderer,
            }
        }

        fn dispatch(&mut self, event: &Event) -> event::Status {
            let mut messages = Vec::new();
            let mut shell = Shell::new(&mut messages);
            let layout = Layout::new(&self.node);
            self.field.as_widget_mut().update(
                &mut self.tree,
                event,
                layout,
                mouse::Cursor::Available(Point::new(4.0, 4.0)),
                &self.renderer,
                &mut clipboard::Null,
                &mut shell,
                &layout.bounds(),
            );
            shell.event_status()
        }
    }

    fn key_press(key: Key, physical: key::Code, modifiers: Modifiers) -> Event {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: key::Physical::Code(physical),
            location: Location::Standard,
            modifiers,
            text: None,
            repeat: false,
        })
    }

    #[test]
    fn captures_shortcuts_only_while_focused() {
        let mut harness = Harness::new();
        let shortcuts = [
            key_press(Key::Character("z".into()), key::Code::KeyZ, Modifiers::CTRL),
            key_press(Key::Character("s".into()), key::Code::KeyS, Modifiers::CTRL),
            key_press(
                Key::Character("b".into()),
                key::Code::KeyB,
                Modifiers::empty(),
            ),
        ];

        for shortcut in &shortcuts {
            assert_eq!(harness.dispatch(shortcut), event::Status::Ignored);
        }

        let _ = harness.dispatch(&Event::Mouse(mouse::Event::ButtonPressed(
            mouse::Button::Left,
        )));
        for shortcut in &shortcuts {
            assert_eq!(harness.dispatch(shortcut), event::Status::Captured);
        }

        let escape = key_press(
            Key::Named(key::Named::Escape),
            key::Code::Escape,
            Modifiers::empty(),
        );
        let _ = harness.dispatch(&escape);
        for shortcut in &shortcuts {
            assert_eq!(harness.dispatch(shortcut), event::Status::Ignored);
        }
    }
}
