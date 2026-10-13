use crate::{Tool, editor::Editor, mode::Mode};

impl Editor {
    pub(super) fn select_tool(&mut self, tool: Tool) {
        self.tool = tool;
        self.mode = Mode::Draw;
        self.size_input = None;
        self.close_color_palette();
    }

    pub(super) fn enter_selection_mode(&mut self) {
        if self.mode == Mode::Draw {
            self.mode = Mode::Select(None);
        }
        self.size_input = None;
        self.close_color_palette();
    }

    /// Selects an existing edit, ignoring indices outside the current edits
    pub(super) fn select_edit(&mut self, index: Option<usize>) {
        if let Mode::Select(selected) = &mut self.mode {
            *selected = index.filter(|&index| index < self.document.edits().len());
        }
    }

    /// Returns the tool a drag draws with, or `None` while selecting
    pub(super) const fn drawing_tool(&self) -> Option<Tool> {
        match self.mode {
            Mode::Draw => Some(self.tool),
            Mode::Select(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Settings, ThemePreference, annotation::AnnotationDraft, editor::Message, style::RED,
    };
    use iced::{Point, Size, widget::image::Handle};
    use std::rc::Rc;

    fn editor_with_line() -> Editor {
        let mut editor = Editor::new(
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
        );
        let _ = editor.update(Message::AddEdit(AnnotationDraft {
            tool: Tool::Line,
            size: 4,
            stroke_color: RED,
            points: vec![Point::ORIGIN, Point::new(1.0, 1.0)],
        }));
        editor
    }

    #[test]
    fn selection_requires_selection_mode_and_an_existing_edit() {
        let mut editor = editor_with_line();

        let _ = editor.update(Message::SelectEdit(Some(0)));
        let _ = editor.update(Message::DeleteSelected);
        assert_eq!(editor.mode, Mode::Draw);
        assert_eq!(editor.document.edits().len(), 1);

        let _ = editor.update(Message::SelectionMode);
        let _ = editor.update(Message::SelectEdit(Some(1)));
        assert_eq!(editor.mode, Mode::Select(None));

        let _ = editor.update(Message::SelectEdit(Some(0)));
        let _ = editor.update(Message::SelectionMode);
        assert_eq!(editor.mode, Mode::Select(Some(0)));
        assert_eq!(editor.drawing_tool(), None);
    }

    #[test]
    fn clearing_or_switching_tools_drops_the_selection() {
        let mut editor = editor_with_line();
        let _ = editor.update(Message::SelectionMode);
        let _ = editor.update(Message::SelectEdit(Some(0)));

        let _ = editor.update(Message::SelectEdit(None));
        assert_eq!(editor.mode, Mode::Select(None));

        let _ = editor.update(Message::SelectEdit(Some(0)));
        let _ = editor.update(Message::SelectTool(Tool::Pencil));
        assert_eq!(editor.mode, Mode::Draw);
        assert_eq!(editor.drawing_tool(), Some(Tool::Pencil));

        let _ = editor.update(Message::DeleteSelected);
        assert_eq!(editor.document.edits().len(), 1);
    }

    #[test]
    fn deleting_the_selected_edit_clears_the_selection() {
        let mut editor = editor_with_line();

        for message in [
            Message::SelectionMode,
            Message::SelectEdit(Some(0)),
            Message::DeleteSelected,
        ] {
            let _ = editor.update(message);
        }

        assert!(editor.document.edits().is_empty());
        assert_eq!(editor.mode, Mode::Select(None));
    }
}
