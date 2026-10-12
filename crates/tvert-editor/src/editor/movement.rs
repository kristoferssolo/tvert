use crate::{
    edit::Edit,
    editor::{Editor, MoveMessage, history::Change},
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
        // The starting edits rendered before, so a failure only leaves the
        // preview stale, never the edits or history
        if let Some(edit) = self.edits.get_mut(index) {
            *edit = original;
        }
        self.refresh_preview();
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
            && self
                .apply(Change::Replace(
                    movement.index,
                    movement.original.translated(offset),
                ))
                .is_ok()
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
            original: self.edits.get(index)?.translated(Vector::ZERO),
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
        self.undo_stack.push(Change::Replace(index, original));
        self.redo_stack.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Settings, ThemePreference, Tool, annotation::AnnotationDraft, editor::Message, style::RED,
    };
    use claims::assert_ok;
    use iced::{Color, Point, Size, widget::image::Handle};
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

    /// A 32×16 image of alternating red and black columns
    fn striped(drafts: impl IntoIterator<Item = AnnotationDraft>) -> Editor {
        let stripes = (0..512)
            .flat_map(|index| [if index % 2 == 0 { 255 } else { 0 }, 0, 0, 255])
            .collect::<Vec<u8>>();
        let mut editor = Editor::new(
            Handle::from_rgba(32, 16, stripes),
            Size::new(32.0, 16.0),
            SETTINGS,
            Rc::new(|_, _| Ok(())),
        );
        send(&mut editor, drafts.into_iter().map(Message::AddEdit));
        editor
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

    /// Exact retained geometry and appearance of every edit
    fn snapshot(editor: &Editor) -> String {
        format!("{:?}", editor.edits)
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
        let mut editor = striped([draft(Tool::Pencil, &points)]);
        let start = snapshot(&editor);

        send(&mut editor, select(0));
        send(
            &mut editor,
            [drag(0.1, 0.2), drag(-3.3, 1.7), finish(offset.x, offset.y)],
        );
        let moved = snapshot(&editor);

        let expected = striped([draft(Tool::Pencil, &points.map(|point| point + offset))]);
        assert_eq!(moved, snapshot(&expected));
        assert_eq!(pixels(&editor), pixels(&expected));
        assert_eq!(editor.mode, Mode::Select(Some(0)));
        assert_eq!(editor.undo_stack.len(), 2);

        send(&mut editor, [Message::Undo]);
        assert_eq!(snapshot(&editor), start);

        send(&mut editor, [Message::Redo]);
        assert_eq!(snapshot(&editor), moved);
        assert!(editor.redo_stack.is_empty());
    }

    #[test]
    fn a_move_back_to_its_start_records_nothing_and_keeps_redo() {
        let points = [Point::new(2.0, 2.0), Point::new(20.0, 10.0)];
        let mut editor = striped([draft(Tool::Line, &points), draft(Tool::Rectangle, &points)]);
        send(&mut editor, [Message::Undo]);
        let start = snapshot(&editor);

        send(&mut editor, select(0));
        send(&mut editor, [drag(6.0, 3.0), finish(0.0, 0.0)]);

        assert_eq!(snapshot(&editor), start);
        assert_eq!(editor.mode, Mode::Select(Some(0)));
        assert_eq!(editor.undo_stack.len(), 1);
        assert_eq!(editor.redo_stack.len(), 1);
    }

    #[rstest]
    #[case::cancel(Message::Move(MoveMessage::Cancel), Mode::Select(Some(0)))]
    #[case::tool_switch(Message::SelectTool(Tool::Line), Mode::Draw)]
    fn cancelling_restores_the_start_without_history(#[case] cancel: Message, #[case] mode: Mode) {
        let region = [Point::ORIGIN, Point::new(8.0, 8.0)];
        let mut editor = striped([draft(Tool::Pixelate, &region)]);
        let (start, original) = (snapshot(&editor), pixels(&editor));

        send(&mut editor, select(0));
        send(&mut editor, [drag(8.0, 4.0)]);
        assert_ne!(pixels(&editor), original);

        send(&mut editor, [cancel]);
        assert_eq!(snapshot(&editor), start);
        assert_eq!(pixels(&editor), original);
        assert_eq!(editor.mode, mode);
        assert_eq!(editor.undo_stack.len(), 1);
    }

    #[test]
    fn a_moved_pixel_effect_rerenders_in_order_within_the_image() {
        let line = draft(Tool::Line, &[Point::new(0.0, 8.0), Point::new(32.0, 8.0)]);
        let mut editor = striped([
            draft(Tool::Pixelate, &[Point::ORIGIN, Point::new(8.0, 8.0)]),
            line.clone(),
        ]);
        let expected = striped([
            draft(
                Tool::Pixelate,
                &[Point::new(24.0, 4.0), Point::new(32.0, 12.0)],
            ),
            line,
        ]);
        assert_ne!(pixels(&editor), pixels(&expected));

        send(&mut editor, select(0));
        send(&mut editor, [drag(40.0, 4.0)]);
        assert_eq!(pixels(&editor), pixels(&expected));

        send(&mut editor, [finish(40.0, 4.0)]);
        assert_eq!(snapshot(&editor), snapshot(&expected));
        assert_eq!(pixels(&editor), pixels(&expected));
    }
}
