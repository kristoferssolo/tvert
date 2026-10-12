use crate::editor::{Editor, message::SizeMessage};

impl Editor {
    pub(super) fn update_size(&mut self, message: SizeMessage) {
        match message {
            SizeMessage::Select(kind, size) => {
                self.sizes.set(kind, size);
                self.size_input = None;
            }
            SizeMessage::Edit(kind, input) => {
                if let Some(size) = kind.parse(&input) {
                    self.sizes.set(kind, size);
                }
                self.size_input = Some(input);
            }
            // Valid text was already applied, so this shows it normalized
            // or restores the last valid size
            SizeMessage::Submit => self.size_input = None,
        }
    }

    pub(super) const fn selected_size(&self) -> u16 {
        self.sizes.get(self.tool.size_kind())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Settings, ThemePreference, Tool, editor::Message, size::SizeKind, style::RED};
    use claims::assert_none;
    use iced::{Size, widget::image::Handle};
    use rstest::rstest;
    use std::rc::Rc;

    fn editor() -> Editor {
        Editor::new(
            Handle::from_rgba(1, 1, vec![0, 0, 0, 255]),
            Size::new(1.0, 1.0),
            Settings {
                theme: ThemePreference::Dark,
                tool: Tool::Arrow,
                stroke_color: RED,
                stroke_width: 4,
                pixelation_block_size: 16,
                blur_sigma: 8,
            },
            Rc::new(|_, _| Ok(())),
        )
    }

    fn edit(editor: &mut Editor, kind: SizeKind, input: &str) {
        let _ = editor.update(Message::Size(SizeMessage::Edit(kind, input.into())));
    }

    #[test]
    fn typed_text_is_preserved_and_only_valid_sizes_apply() {
        let mut editor = editor();

        for input in ["", "-", "4.", "0", "33", "x"] {
            edit(&mut editor, SizeKind::StrokeWidth, input);
            assert_eq!(editor.size_input.as_deref(), Some(input));
            assert_eq!(editor.selected_size(), 4);
        }

        edit(&mut editor, SizeKind::StrokeWidth, "012");
        assert_eq!(editor.selected_size(), 12);

        edit(&mut editor, SizeKind::StrokeWidth, "120");
        assert_eq!(editor.size_input.as_deref(), Some("120"));
        assert_eq!(editor.selected_size(), 12);
    }

    #[rstest]
    #[case("012", 12)]
    #[case("33", 4)]
    #[case("", 4)]
    fn submitting_shows_the_drawing_size(#[case] input: &str, #[case] size: u16) {
        let mut editor = editor();

        edit(&mut editor, SizeKind::StrokeWidth, input);
        let _ = editor.update(Message::Size(SizeMessage::Submit));

        assert_none!(&editor.size_input);
        assert_eq!(editor.selected_size(), size);
    }

    #[test]
    fn slider_changes_replace_unsubmitted_text() {
        let mut editor = editor();

        edit(&mut editor, SizeKind::StrokeWidth, "40");
        let _ = editor.update(Message::Size(SizeMessage::Select(SizeKind::StrokeWidth, 9)));

        assert_none!(&editor.size_input);
        assert_eq!(editor.selected_size(), 9);
    }

    #[test]
    fn switching_tools_discards_text_and_shows_the_tools_size() {
        let mut editor = editor();

        edit(&mut editor, SizeKind::StrokeWidth, "9");
        edit(&mut editor, SizeKind::StrokeWidth, "9x");
        let _ = editor.update(Message::SelectTool(Tool::Pixelate));

        assert_none!(&editor.size_input);
        assert_eq!(editor.selected_size(), 16);

        edit(&mut editor, SizeKind::BlockSize, "2");
        let _ = editor.update(Message::SelectTool(Tool::Rectangle));

        assert_none!(&editor.size_input);
        assert_eq!(editor.selected_size(), 9);
    }
}
