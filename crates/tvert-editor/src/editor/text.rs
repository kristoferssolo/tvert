use crate::{
    annotation::Annotation,
    edit::Edit,
    editor::{Editor, Message, TextMessage, document::Change},
    figure::{FONT, LINE_HEIGHT, TextFigure, measure},
    size::SizeKind,
    viewport::Viewport,
};
use iced::{
    Background, Border, Color, Element, Padding, Point, Rectangle, Size, Task, Theme,
    advanced::text::Wrapping,
    keyboard::{Key, key::Named},
    widget::{
        Id, operation, pin, space,
        text_editor::{self, Binding, Content, KeyPress, Status, TextEditor},
    },
};

/// Identifies the in-place editor so placing text can focus it
const EDITOR_ID: Id = Id::new("text-annotation");

/// Text being typed in place, with the appearance selected when it began
#[derive(Debug)]
pub(super) struct TextDraft {
    position: Point,
    /// Font size in captured-image pixels
    size: u16,
    color: Color,
    content: Content,
}

impl Editor {
    pub(super) fn update_text(&mut self, message: TextMessage) -> Task<Message> {
        match message {
            TextMessage::Place(position) => {
                if self.text.is_some() {
                    self.commit_text();
                    return Task::none();
                }
                let Some(position) = position else {
                    return Task::none();
                };
                self.text = Some(TextDraft {
                    position,
                    size: self.sizes.get(SizeKind::FontSize),
                    color: Color::from(self.stroke_selection),
                    content: Content::new(),
                });
                return operation::focus(EDITOR_ID);
            }
            TextMessage::Edit(action) => {
                if let Some(draft) = &mut self.text {
                    draft.content.perform(action);
                }
            }
            TextMessage::Commit => self.commit_text(),
        }
        Task::none()
    }

    /// Records the typed text as one history entry and stops typing.
    /// Text that is empty after trailing whitespace is discarded.
    pub(super) fn commit_text(&mut self) {
        let Some(draft) = self.text.take() else {
            return;
        };
        let content = draft.content.text();
        let content = content.trim_end();
        if content.is_empty() {
            return;
        }
        let annotation = Annotation {
            figure: Box::new(TextFigure::new(
                content.to_owned(),
                draft.position,
                f32::from(draft.size),
            )),
            stroke_width: draft.size,
            stroke_color: draft.color,
        };
        self.update_document(|document| {
            document.record(Change::Insert(
                document.edits().len(),
                Edit::Annotation(annotation),
            ))
        });
        self.cache.clear();
    }

    /// The in-place editor over a preview of `size`, positioned and scaled to
    /// match how the committed text will be drawn
    pub(super) fn typing_overlay(&self, size: Size, padding: Padding) -> Element<'_, Message> {
        let Some((draft, viewport)) = self.text.as_ref().zip(Viewport::navigated(
            self.image_size,
            self.visible_area(),
            Rectangle::with_size(size),
            padding,
            self.navigation,
        )) else {
            return space().into();
        };
        let font_size = viewport.canvas_length(f32::from(draft.size));
        let position = viewport.canvas_position(draft.position);
        // Room for the caret after the last character
        let width = measure(&draft.content.text(), font_size).width + font_size;
        let color = draft.color;

        let editor = TextEditor::new(&draft.content)
            .id(EDITOR_ID)
            .on_action(|action| Message::Text(TextMessage::Edit(action)))
            .key_binding(binding)
            .font(FONT)
            .size(font_size)
            .line_height(LINE_HEIGHT)
            .padding(0)
            .wrapping(Wrapping::None)
            .width(width)
            .style(move |theme, status| style(theme, status, color));

        pin(editor).position(position).into()
    }
}

/// Esc records the text. Undo and redo act on the document, so typing
/// swallows them instead of recording the text only to undo it.
fn binding(press: KeyPress) -> Option<Binding<Message>> {
    if !matches!(press.status, Status::Focused { .. }) {
        return None;
    }
    if press.key == Key::Named(Named::Escape) {
        return Some(Binding::Custom(Message::Text(TextMessage::Commit)));
    }
    if press.modifiers.command()
        && matches!(press.key.to_latin(press.physical_key), Some('z' | 'y'))
    {
        return Some(Binding::Sequence(Vec::new()));
    }
    Binding::from_key_press(press)
}

/// Draws only the text, in its annotation color, over the image
fn style(theme: &Theme, status: Status, color: Color) -> text_editor::Style {
    text_editor::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        placeholder: color,
        value: color,
        ..text_editor::default(theme, status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{tool::Tool, viewport::Navigation};
    use claims::{assert_none, assert_ok, assert_some, assert_some_eq};
    use iced::{
        keyboard::{Modifiers, key},
        widget::text_editor::{Action, Edit as TextEdit},
    };
    use std::sync::Arc;

    fn editor() -> Editor {
        let mut editor = Editor::for_test(64, 32, [0, 0, 0, 255].repeat(2048));
        let _ = editor.update(Message::SelectTool(Tool::Text));
        editor
    }

    fn type_text(editor: &mut Editor, text: &str) {
        let _ = editor.update(Message::Text(TextMessage::Place(Some(Point::new(
            2.0, 2.0,
        )))));
        let _ = editor.update(Message::Text(TextMessage::Edit(Action::Edit(
            TextEdit::Paste(Arc::new(text.to_owned())),
        ))));
    }

    #[test]
    fn committed_text_is_one_undoable_edit_drawn_in_exports() {
        let mut editor = editor();
        let blank = assert_ok!(editor.render_image());

        type_text(&mut editor, "Hi\nthere");
        assert_some!(&editor.text);
        assert!(editor.document.edits().is_empty());

        let _ = editor.update(Message::Text(TextMessage::Commit));
        assert_none!(&editor.text);
        assert_eq!(editor.document.edits().len(), 1);

        let image = assert_ok!(editor.render_image());
        let red = |rgba: &[u8]| {
            rgba.as_chunks::<4>()
                .0
                .iter()
                .any(|&[red, green, _, _]| red > 128 && green < 128)
        };
        assert!(!red(blank.rgba()));
        assert!(red(image.rgba()));

        let _ = editor.update(Message::Undo);
        assert!(editor.document.edits().is_empty());
    }

    #[test]
    fn blank_text_is_discarded() {
        let mut editor = editor();
        type_text(&mut editor, " \n\t");

        let _ = editor.update(Message::Text(TextMessage::Commit));

        assert_none!(&editor.text);
        assert!(!editor.document.can_undo());
    }

    #[test]
    fn a_click_while_typing_records_the_text_instead_of_starting_more() {
        let mut editor = editor();
        let _ = editor.update(Message::Text(TextMessage::Place(None)));
        assert_none!(&editor.text);

        type_text(&mut editor, "Tvert");
        let _ = editor.update(Message::Text(TextMessage::Place(Some(Point::new(
            30.0, 20.0,
        )))));

        assert_none!(&editor.text);
        assert_eq!(editor.document.edits().len(), 1);
    }

    #[test]
    fn navigating_keeps_typing_and_other_actions_record_the_text() {
        let mut editor = editor();
        type_text(&mut editor, "Tvert");

        let _ = editor.update(Message::Navigate(Navigation::Manual {
            scale: 4.0,
            center: Point::new(8.0, 8.0),
        }));
        assert_some!(&editor.text);

        let _ = editor.update(Message::SelectTool(Tool::Arrow));
        assert_none!(&editor.text);
        assert_eq!(editor.document.edits().len(), 1);
    }

    fn press(key: Key, modifiers: Modifiers, status: Status) -> KeyPress {
        KeyPress {
            key: key.clone(),
            modified_key: key,
            physical_key: key::Physical::Code(key::Code::KeyZ),
            modifiers,
            text: None,
            status,
        }
    }

    #[test]
    fn escape_records_the_text_and_history_keys_do_nothing_while_typing() {
        let focused = Status::Focused { is_hovered: false };

        assert_some_eq!(
            binding(press(
                Key::Named(Named::Escape),
                Modifiers::empty(),
                focused
            )),
            Binding::Custom(Message::Text(TextMessage::Commit))
        );
        assert_some_eq!(
            binding(press(Key::Character("z".into()), Modifiers::CTRL, focused)),
            Binding::Sequence(Vec::new())
        );
        assert_some_eq!(
            binding(press(Key::Named(Named::Enter), Modifiers::empty(), focused)),
            Binding::Enter
        );
        assert_none!(binding(press(
            Key::Named(Named::Escape),
            Modifiers::empty(),
            Status::Active
        )));
    }
}
