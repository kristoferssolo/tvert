use crate::{
    Tool,
    crop::Crop,
    editor::{Editor, document::Change},
    mode::Mode,
    viewport::Navigation,
};
use iced::Rectangle;

impl Editor {
    pub(super) fn select_tool(&mut self, tool: Tool) {
        self.tool = tool;
        self.set_mode(Mode::Draw);
        self.dismiss_controls();
    }

    pub(super) fn enter_selection_mode(&mut self) {
        if !matches!(self.mode, Mode::Select(_)) {
            self.set_mode(Mode::Select(None));
        }
        self.dismiss_controls();
    }

    pub(super) fn enter_crop_mode(&mut self) {
        if !matches!(self.mode, Mode::Crop(_)) {
            self.set_mode(Mode::Crop(None));
        }
        self.dismiss_controls();
    }

    /// Records the proposed crop, if any, and returns to drawing, where the
    /// view fits the crop. Leaving crop mode any other way discards the
    /// proposal.
    pub(super) fn apply_crop(&mut self) {
        let Mode::Crop(proposed) = self.mode else {
            return;
        };
        if let Some(crop) = proposed
            && self.document.crop() != Some(crop)
        {
            self.update_document(|document| document.record(Change::Crop(Some(crop))));
        }
        self.set_mode(Mode::Draw);
    }

    /// Crop mode shows the whole image and other modes show only the crop,
    /// so crossing that boundary with a crop set refits the view
    fn set_mode(&mut self, mode: Mode) {
        let cropping = |mode| matches!(mode, Mode::Crop(_));
        if cropping(self.mode) != cropping(mode) && self.document.crop().is_some() {
            self.navigation = Navigation::Fit;
        }
        self.mode = mode;
    }

    /// Selects an existing edit, ignoring indices outside the current edits
    pub(super) fn select_edit(&mut self, index: Option<usize>) {
        if let Mode::Select(selected) = &mut self.mode {
            *selected = index.filter(|&index| index < self.document.edits().len());
        }
    }

    /// Returns the tool a drag draws with, or `None` while selecting or cropping
    pub(super) const fn drawing_tool(&self) -> Option<Tool> {
        match self.mode {
            Mode::Draw => Some(self.tool),
            Mode::Select(_) | Mode::Crop(_) => None,
        }
    }

    /// The part of the image shown, in captured-image coordinates, or
    /// `None` for all of it
    pub(super) fn visible_area(&self) -> Option<Rectangle> {
        if matches!(self.mode, Mode::Crop(_)) {
            return None;
        }
        self.document.crop().map(Crop::bounds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        annotation::AnnotationDraft,
        editor::{Message, SizeMessage},
        size::SizeKind,
        style::RED,
    };
    use claims::{assert_none, assert_some, assert_some_eq};
    use iced::Point;
    use rstest::rstest;

    fn editor_with_line() -> Editor {
        let mut editor = Editor::for_test(1, 1, vec![0, 0, 0, 255]);
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

    #[rstest]
    #[case::tool(Message::SelectTool(Tool::Pencil))]
    #[case::selection_mode(Message::SelectionMode)]
    #[case::crop_mode(Message::CropMode)]
    fn changing_mode_dismisses_controls(#[case] change: Message) {
        let mut editor = editor_with_line();

        for message in [
            Message::ToggleColorPalette,
            Message::Size(SizeMessage::Edit(SizeKind::StrokeWidth, "9x".into())),
            change,
        ] {
            let _ = editor.update(message);
        }

        assert_none!(&editor.size_input);
        assert!(!editor.color_palette_open);
    }

    #[test]
    fn crop_mode_shows_the_whole_image_and_applying_fits_the_crop() {
        let mut editor = Editor::for_test(4, 4, vec![0; 64]);
        let crop = assert_some!(Crop::from_drag(
            Point::new(1.0, 1.0),
            Point::new(3.0, 3.0),
            editor.image_size
        ));
        let zoomed = Navigation::Manual {
            scale: 8.0,
            center: Point::new(2.0, 2.0),
        };

        let _ = editor.update(Message::CropMode);
        let _ = editor.update(Message::Crop(crop));
        let _ = editor.update(Message::Navigate(zoomed));
        assert_none!(editor.visible_area());

        let _ = editor.update(Message::ApplyCrop);
        assert_eq!(editor.mode, Mode::Draw);
        assert_eq!(editor.navigation, Navigation::Fit);
        assert_some_eq!(editor.visible_area(), crop.bounds());

        let _ = editor.update(Message::Navigate(zoomed));
        let _ = editor.update(Message::CropMode);
        assert_eq!(editor.navigation, Navigation::Fit);
        assert_none!(editor.visible_area());
    }

    #[test]
    fn leaving_crop_mode_without_enter_discards_the_proposal() {
        let mut editor = Editor::for_test(4, 4, vec![0; 64]);
        let crop = assert_some!(Crop::from_drag(
            Point::new(1.0, 1.0),
            Point::new(3.0, 3.0),
            editor.image_size
        ));

        for message in [
            Message::CropMode,
            Message::Crop(crop),
            Message::SelectTool(Tool::Line),
            Message::ApplyCrop,
        ] {
            let _ = editor.update(message);
        }

        assert_none!(editor.document.crop());
        assert!(!editor.document.can_undo());
    }
}
