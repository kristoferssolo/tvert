use crate::{
    edit::Edit,
    editor::{Editor, MoveMessage, document::Change},
    mode::Mode,
};
use iced::Vector;

/// The selected edit while a drag moves it.
///
/// Previews, cancellation, and the recorded history entry all use the
/// untouched starting edit, so geometry never drifts through inverse moves.
#[derive(Debug)]
pub struct Movement {
    index: usize,
    original: Edit,
    offset: Vector,
}

impl Editor {
    pub(super) fn update_move(&mut self, message: MoveMessage) {
        match message {
            MoveMessage::Drag(offset) => self.drag_selected(offset),
            MoveMessage::Finish(offset) => {
                self.drag_selected(offset);
                self.finish_move();
            }
            MoveMessage::Cancel => self.cancel_move(),
        }
    }

    /// Restores the moved edit to where the gesture began without history
    pub(super) fn cancel_move(&mut self) {
        let Some(Movement {
            index,
            original,
            offset,
        }) = self.movement.take()
        else {
            return;
        };
        if offset == Vector::ZERO {
            return;
        }
        self.error = self.document.restore_edit(index, original).err();
        self.cache.clear();
    }
    /// Previews the selected edit at `requested` from its starting position.
    ///
    /// Keeps the last rendered position if the preview cannot render.
    fn drag_selected(&mut self, requested: Vector) {
        let Some(mut movement) = self.movement.take().or_else(|| self.start_move()) else {
            return;
        };
        let offset = movement
            .original
            .constrain_offset(requested, self.image_size);
        if offset != movement.offset
            && self.update_document(|document| {
                let moved = movement.original.translated(offset);
                document
                    .apply(Change::Replace(movement.index, moved))
                    .map(drop)
            })
        {
            movement.offset = offset;
        }
        self.movement = Some(movement);
    }

    fn start_move(&self) -> Option<Movement> {
        let Mode::Select(Some(index)) = self.mode else {
            return None;
        };
        Some(Movement {
            index,
            original: self.document.edits().get(index)?.translated(Vector::ZERO),
            offset: Vector::ZERO,
        })
    }

    fn finish_move(&mut self) {
        let Some(Movement {
            index,
            original,
            offset,
        }) = self.movement.take()
        else {
            return;
        };
        if offset == Vector::ZERO {
            return;
        }
        self.document
            .record_applied(Change::Replace(index, original));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Settings, ThemePreference, Tool, annotation::AnnotationDraft, editor::Message,
        render::render_edits, style::RED, viewport::Viewport,
    };
    use claims::{assert_ok, assert_some};
    use iced::{Color, Point, Rectangle, Size, widget::image::Handle};
    use rstest::rstest;
    use std::rc::Rc;

    const SETTINGS: Settings = Settings {
        theme: ThemePreference::Dark,
        tool: Tool::Arrow,
        stroke_color: RED,
        stroke_width: 4,
        pixelation_block_size: 4,
        blur_sigma: 8,
    };

    fn draft(tool: Tool, points: &[Point]) -> AnnotationDraft {
        AnnotationDraft {
            tool,
            size: 4,
            stroke_color: Color::WHITE,
            points: points.to_vec(),
        }
    }

    fn editor(drafts: impl IntoIterator<Item = AnnotationDraft>) -> Editor {
        let mut editor = Editor::new(
            original(),
            Size::new(32.0, 16.0),
            SETTINGS,
            Rc::new(|_, _| Ok(())),
        );
        send(&mut editor, drafts.into_iter().map(Message::AddEdit));
        editor
    }

    fn original() -> Handle {
        let pixels = (0..512)
            .flat_map(|index| {
                [
                    assert_ok!(u8::try_from(index % 32)),
                    assert_ok!(u8::try_from(index / 32)),
                    0,
                    255,
                ]
            })
            .collect::<Vec<u8>>();
        Handle::from_rgba(32, 16, pixels)
    }

    fn send(editor: &mut Editor, messages: impl IntoIterator<Item = Message>) {
        for message in messages {
            let _ = editor.update(message);
        }
    }

    fn select(index: usize) -> [Message; 2] {
        [Message::SelectionMode, Message::SelectEdit(Some(index))]
    }

    fn drag(x: f32, y: f32) -> Message {
        Message::Move(MoveMessage::Drag(Vector::new(x, y)))
    }

    fn finish(x: f32, y: f32) -> Message {
        Message::Move(MoveMessage::Finish(Vector::new(x, y)))
    }

    fn assert_edits(editor: &Editor, drafts: &[AnnotationDraft]) {
        // Unit dimensions avoid rounding coordinates through a scale conversion.
        let unit = Size::new(1.0, 1.0);
        let viewport = assert_some!(Viewport::new(unit, Rectangle::with_size(unit)));
        let expected = drafts
            .iter()
            .cloned()
            .map(|draft| assert_ok!(Edit::try_from(draft)))
            .collect::<Vec<_>>();
        assert_eq!(editor.document.edits().len(), expected.len());

        for (actual, expected) in editor.document.edits().iter().zip(&expected) {
            match (actual, expected) {
                (Edit::Annotation(actual), Edit::Annotation(expected)) => {
                    let actual_path = actual.figure.path(viewport);
                    let expected_path = expected.figure.path(viewport);
                    assert_eq!(
                        actual_path.raw().iter().collect::<Vec<_>>(),
                        expected_path.raw().iter().collect::<Vec<_>>()
                    );
                    assert_eq!(actual.figure.filled(), expected.figure.filled());
                    assert_eq!(actual.stroke_width, expected.stroke_width);
                    assert_eq!(actual.stroke_color, expected.stroke_color);
                }
                (Edit::PixelEffect(actual), Edit::PixelEffect(expected)) => {
                    assert_eq!(
                        actual.region().canvas_bounds(viewport),
                        expected.region().canvas_bounds(viewport)
                    );
                }
                _ => panic!("retained edit kind changed"),
            }
        }

        assert_eq!(
            pixels(editor),
            assert_ok!(render_edits(&original(), &expected)).into_rgba()
        );
    }

    fn pixels(editor: &Editor) -> Vec<u8> {
        assert_ok!(editor.render_image()).into_rgba()
    }

    #[test]
    fn a_move_is_one_history_entry_that_restores_exact_geometry() {
        let points = [
            Point::new(1.1, 2.3),
            Point::new(7.7, 9.9),
            Point::new(13.3, 4.4),
        ];
        let offset = Vector::new(0.7, -0.3);
        let start = draft(Tool::Pencil, &points);
        let mut editor = editor([start.clone()]);

        send(&mut editor, select(0));
        send(
            &mut editor,
            [drag(0.1, 0.2), drag(-3.3, 1.7), finish(offset.x, offset.y)],
        );
        let moved = draft(Tool::Pencil, &points.map(|point| point + offset));

        assert_edits(&editor, std::slice::from_ref(&moved));
        assert_eq!(editor.mode, Mode::Select(Some(0)));

        send(&mut editor, [Message::Undo]);
        assert_edits(&editor, &[start]);

        send(&mut editor, [Message::Redo]);
        assert_edits(&editor, &[moved]);
        assert!(!editor.document.can_redo());
    }

    #[test]
    fn a_move_back_to_its_start_records_nothing_and_keeps_redo() {
        let points = [Point::new(2.0, 2.0), Point::new(20.0, 10.0)];
        let start = draft(Tool::Line, &points);
        let mut editor = editor([start.clone(), draft(Tool::Rectangle, &points)]);
        send(&mut editor, [Message::Undo]);

        send(&mut editor, select(0));
        send(&mut editor, [drag(6.0, 3.0), finish(0.0, 0.0)]);

        assert_edits(&editor, &[start]);
        assert_eq!(editor.mode, Mode::Select(Some(0)));
        assert!(editor.document.can_redo());

        send(&mut editor, [Message::Undo]);
        assert_edits(&editor, &[]);
    }

    #[rstest]
    #[case::cancel(Message::Move(MoveMessage::Cancel), Mode::Select(Some(0)))]
    #[case::tool_switch(Message::SelectTool(Tool::Line), Mode::Draw)]
    fn cancelling_restores_the_start_without_history(#[case] cancel: Message, #[case] mode: Mode) {
        let start = draft(Tool::Pixelate, &[Point::ORIGIN, Point::new(8.0, 8.0)]);
        let mut editor = editor([start.clone()]);
        let original = pixels(&editor);

        send(&mut editor, select(0));
        send(&mut editor, [drag(8.0, 4.0)]);
        assert_ne!(pixels(&editor), original);

        send(&mut editor, [cancel]);
        assert_edits(&editor, &[start]);
        assert_eq!(editor.mode, mode);

        send(&mut editor, [Message::Undo]);
        assert_edits(&editor, &[]);
    }

    #[test]
    fn a_moved_pixel_effect_rerenders_in_order_within_the_image() {
        let line = draft(Tool::Line, &[Point::new(0.0, 8.0), Point::new(32.0, 8.0)]);
        let mut editor = editor([
            draft(Tool::Pixelate, &[Point::ORIGIN, Point::new(8.0, 8.0)]),
            line.clone(),
        ]);
        let expected = [
            draft(
                Tool::Pixelate,
                &[Point::new(24.0, 4.0), Point::new(32.0, 12.0)],
            ),
            line,
        ];
        let original = pixels(&editor);

        send(&mut editor, select(0));
        send(&mut editor, [drag(40.0, 4.0)]);
        assert_edits(&editor, &expected);
        assert_ne!(pixels(&editor), original);

        send(&mut editor, [finish(40.0, 4.0)]);
        assert_edits(&editor, &expected);
    }
}
