mod change;

pub(super) use change::Change;

use crate::{
    Error,
    edit::Edit,
    render::{handle_from_image, render_edits},
};
use iced::widget::image::Handle;

/// Retained edits, their undo history, and their full-resolution rendering
/// from an untouched source.
///
/// Applying a change commits both edits and preview or leaves both unchanged.
/// Cancelling a gesture restores its geometry even when rendering fails.
pub(super) struct Document {
    original: Handle,
    preview: Handle,
    edits: Vec<Edit>,
    undo: Vec<Change>,
    redo: Vec<Change>,
}

/// Why a change left the edits and preview unchanged.
#[derive(Debug)]
pub(super) enum ApplyError {
    /// The change does not fit the edits, or there is nothing to undo or redo.
    Rejected,
    /// The edits could not render, so the change was rolled back.
    Render(Error),
}

impl Document {
    pub(super) fn new(original: Handle) -> Self {
        Self {
            preview: original.clone(),
            original,
            edits: Vec::new(),
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub(super) const fn preview(&self) -> &Handle {
        &self.preview
    }

    pub(super) fn edits(&self) -> &[Edit] {
        &self.edits
    }

    pub(super) const fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub(super) const fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Applies `change` without recording history and returns its inverse.
    pub(super) fn apply(&mut self, change: Change) -> Result<Change, ApplyError> {
        self.try_apply(change).map_err(|(_, error)| error)
    }

    /// Applies `change` as a completed operation.
    /// A failed change leaves the redo branch intact.
    pub(super) fn record(&mut self, change: Change) -> Result<(), ApplyError> {
        let inverse = self.apply(change)?;
        self.record_applied(inverse);
        Ok(())
    }

    /// Records the inverse of a change already applied and rendered.
    ///
    /// Call only for a completed operation, never a gesture preview.
    pub(super) fn record_applied(&mut self, inverse: Change) {
        self.undo.push(inverse);
        self.redo.clear();
    }

    /// A failed undo keeps its entry for another attempt.
    pub(super) fn undo(&mut self) -> Result<(), ApplyError> {
        let change = self.undo.pop().ok_or(ApplyError::Rejected)?;
        match self.try_apply(change) {
            Ok(inverse) => {
                self.redo.push(inverse);
                Ok(())
            }
            Err((change, error)) => {
                self.undo.push(change);
                Err(error)
            }
        }
    }

    /// A failed redo keeps its entry for another attempt.
    pub(super) fn redo(&mut self) -> Result<(), ApplyError> {
        let change = self.redo.pop().ok_or(ApplyError::Rejected)?;
        match self.try_apply(change) {
            Ok(inverse) => {
                self.undo.push(inverse);
                Ok(())
            }
            Err((change, error)) => {
                self.redo.push(change);
                Err(error)
            }
        }
    }

    /// Restores cancelled geometry without rollback or a history entry.
    /// A rendering failure leaves the previous preview in place.
    pub(super) fn restore_edit(&mut self, index: usize, original: Edit) -> Result<(), Error> {
        if let Some(edit) = self.edits.get_mut(index) {
            *edit = original;
        }
        self.refresh_preview()
    }

    // Returns the unapplied change on failure so history can keep it.
    fn try_apply(&mut self, change: Change) -> Result<Change, (Change, ApplyError)> {
        let inverse = change
            .apply(&mut self.edits)
            .map_err(|change| (change, ApplyError::Rejected))?;
        if let Err(error) = self.refresh_preview() {
            let change = inverse
                .apply(&mut self.edits)
                .unwrap_or_else(|inverse| inverse);
            return Err((change, ApplyError::Render(error)));
        }
        Ok(inverse)
    }

    fn refresh_preview(&mut self) -> Result<(), Error> {
        self.preview = handle_from_image(render_edits(&self.original, &self.edits)?);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        annotation::AnnotationDraft,
        pixel_effect::{EffectRegion, PixelEffect},
        render::render_image,
        tool::Tool,
        viewport::Viewport,
    };
    use claims::{assert_err, assert_ok, assert_some};
    use iced::{Color, Point, Rectangle, Size, Vector};
    use rstest::rstest;
    use std::{assert_matches, cell::Cell, rc::Rc};
    use tvert_image::CapturedImage;

    fn source() -> Handle {
        let pixels = (0..16)
            .flat_map(|red| [red, 20, 30, 128])
            .collect::<Vec<u8>>();
        Handle::from_rgba(8, 2, pixels)
    }

    fn effect(tool: Tool, x: f32) -> Edit {
        assert_ok!(Edit::try_from(AnnotationDraft {
            tool,
            size: 4,
            stroke_color: Color::BLACK,
            points: vec![Point::new(x, 0.0), Point::new(x + 4.0, 2.0)],
        }))
    }

    fn pixels(handle: &Handle) -> Vec<u8> {
        assert_ok!(render_image(handle, &[])).into_rgba()
    }

    fn bounds(document: &Document) -> Vec<Rectangle> {
        let unit = Size::new(1.0, 1.0);
        let viewport = assert_some!(Viewport::new(unit, Rectangle::with_size(unit)));
        document
            .edits()
            .iter()
            .map(|edit| {
                let Edit::PixelEffect(effect) = edit else {
                    panic!("retained pixel effect changed kind");
                };
                effect.region().canvas_bounds(viewport)
            })
            .collect()
    }

    #[rstest]
    #[case(Tool::Pixelate)]
    #[case(Tool::Blur)]
    fn applying_and_reversing_effects_preserves_the_original(#[case] tool: Tool) {
        let mut document = Document::new(source());
        let original = pixels(&document.original);
        let inverse = assert_ok!(document.apply(Change::Insert(0, effect(tool, 0.0))));

        assert_ne!(pixels(document.preview()), original);
        assert_eq!(pixels(&document.original), original);

        assert_ok!(document.apply(inverse));
        assert_eq!(pixels(document.preview()), original);
        assert_eq!(pixels(&document.original), original);
    }

    fn regions(xs: &[f32]) -> Vec<Rectangle> {
        xs.iter()
            .map(|&x| Rectangle::new(Point::new(x, 0.0), Size::new(4.0, 2.0)))
            .collect()
    }

    fn push(document: &mut Document, edit: Edit) {
        assert_ok!(document.record(Change::Insert(document.edits().len(), edit)));
    }

    fn document(xs: &[f32]) -> Document {
        let mut document = Document::new(source());
        for &x in xs {
            push(&mut document, effect(Tool::Pixelate, x));
        }
        document
    }

    #[test]
    fn undo_and_redo_restore_the_drawing_order() {
        let mut document = document(&[0.0, 2.0]);

        assert_ok!(document.undo());
        assert_eq!(bounds(&document), regions(&[0.0]));
        assert_ok!(document.undo());
        assert_eq!(bounds(&document), regions(&[]));
        assert_ok!(document.redo());
        assert_eq!(bounds(&document), regions(&[0.0]));
        assert_ok!(document.redo());
        assert_eq!(bounds(&document), regions(&[0.0, 2.0]));
        assert!(!document.can_redo());
    }

    #[test]
    fn recording_after_undo_discards_the_redo_branch() {
        let mut document = document(&[0.0, 2.0]);

        assert_ok!(document.undo());
        push(&mut document, effect(Tool::Pixelate, 4.0));

        assert_matches!(assert_err!(document.redo()), ApplyError::Rejected);
        assert_eq!(bounds(&document), regions(&[0.0, 4.0]));
    }

    #[test]
    fn exhausted_history_is_rejected() {
        let mut document = document(&[]);
        assert_matches!(assert_err!(document.undo()), ApplyError::Rejected);
        assert_matches!(assert_err!(document.redo()), ApplyError::Rejected);

        push(&mut document, effect(Tool::Pixelate, 0.0));
        assert_ok!(document.undo());
        assert_matches!(assert_err!(document.undo()), ApplyError::Rejected);
        assert_eq!(bounds(&document), regions(&[]));
        assert!(document.can_redo());

        assert_ok!(document.redo());
        assert_matches!(assert_err!(document.redo()), ApplyError::Rejected);
        assert_eq!(bounds(&document), regions(&[0.0]));
        assert!(document.can_undo());
    }

    #[test]
    fn undoing_a_deletion_restores_its_position() {
        let mut document = document(&[0.0, 2.0, 4.0]);

        assert_ok!(document.record(Change::Remove(1)));
        assert_eq!(bounds(&document), regions(&[0.0, 4.0]));
        assert_ok!(document.undo());
        assert_eq!(bounds(&document), regions(&[0.0, 2.0, 4.0]));
        assert_ok!(document.redo());
        assert_eq!(bounds(&document), regions(&[0.0, 4.0]));
    }

    #[test]
    fn deleting_rerenders_the_rest_from_the_original_in_order() {
        let line = assert_ok!(Edit::try_from(AnnotationDraft {
            tool: Tool::Line,
            size: 2,
            stroke_color: Color::WHITE,
            points: vec![Point::new(0.0, 1.0), Point::new(8.0, 1.0)],
        }));
        let mut document = Document::new(source());
        push(&mut document, effect(Tool::Pixelate, 0.0));
        push(&mut document, line);
        push(&mut document, effect(Tool::Blur, 0.0));
        let edited = pixels(document.preview());
        let expected = assert_ok!(render_edits(
            &source(),
            &[effect(Tool::Pixelate, 0.0), effect(Tool::Blur, 0.0)]
        ))
        .into_rgba();
        assert_ne!(edited, expected);

        assert_ok!(document.record(Change::Remove(1)));
        assert_eq!(pixels(document.preview()), expected);

        assert_ok!(document.undo());
        assert_eq!(pixels(document.preview()), edited);
    }

    #[rstest]
    #[case(Tool::Pixelate)]
    #[case(Tool::Blur)]
    fn a_failed_record_keeps_the_redo_branch(#[case] tool: Tool) {
        let mut document = Document::new(source());
        let original = pixels(document.preview());
        push(&mut document, effect(tool, 0.0));
        let processed = pixels(document.preview());

        assert_ok!(document.undo());
        assert_eq!(pixels(document.preview()), original);

        assert_matches!(
            assert_err!(document.record(Change::Insert(0, effect(tool, 40.0)))),
            ApplyError::Render(_)
        );
        assert!(document.edits().is_empty());
        assert_eq!(pixels(document.preview()), original);

        assert_ok!(document.redo());
        assert_eq!(pixels(document.preview()), processed);
    }

    // Fail after mutating the temporary render, including when a retained
    // effect is re-rendered by deletion or cancellation.
    #[derive(Debug)]
    struct FallibleEffect {
        effect: Box<dyn PixelEffect>,
        fail: Rc<Cell<bool>>,
    }

    impl PixelEffect for FallibleEffect {
        fn region(&self) -> &EffectRegion {
            self.effect.region()
        }

        fn apply(&self, image: &mut CapturedImage) -> Result<(), Error> {
            self.effect.apply(image)?;
            if self.fail.get() {
                return Err(tvert_image::Error::InvalidPixelationBounds.into());
            }
            Ok(())
        }

        fn translated(&self, offset: Vector) -> Box<dyn PixelEffect> {
            Box::new(Self {
                effect: self.effect.translated(offset),
                fail: Rc::clone(&self.fail),
            })
        }
    }

    fn fallible_document() -> (Document, Rc<Cell<bool>>) {
        let fail = Rc::new(Cell::new(false));
        let Edit::PixelEffect(retained_effect) = effect(Tool::Pixelate, 4.0) else {
            panic!("expected a pixel effect");
        };
        let mut document = Document::new(source());
        assert_ok!(document.apply(Change::Insert(0, effect(Tool::Pixelate, 0.0))));
        assert_ok!(document.apply(Change::Insert(
            1,
            Edit::PixelEffect(Box::new(FallibleEffect {
                effect: retained_effect,
                fail: Rc::clone(&fail),
            })),
        )));
        (document, fail)
    }

    #[rstest]
    #[case::insert(Change::Insert(0, effect(Tool::Blur, 2.0)))]
    #[case::remove(Change::Remove(0))]
    #[case::replace(Change::Replace(0, effect(Tool::Blur, 2.0)))]
    fn a_failed_undo_rolls_back_and_keeps_its_entry(#[case] change: Change) {
        let (mut document, fail) = fallible_document();
        let retained = bounds(&document);
        let rendered = pixels(document.preview());
        let original = pixels(&document.original);
        assert_ok!(document.record(change));
        let changed = bounds(&document);
        let preview = document.preview().id();
        let changed_pixels = pixels(document.preview());
        fail.set(true);

        assert_matches!(
            assert_err!(document.undo()),
            ApplyError::Render(Error::Image(tvert_image::Error::InvalidPixelationBounds))
        );
        assert_eq!(bounds(&document), changed);
        assert_eq!(document.preview().id(), preview);
        assert_eq!(pixels(document.preview()), changed_pixels);
        assert_eq!(pixels(&document.original), original);

        fail.set(false);
        assert_ok!(document.undo());
        assert_eq!(bounds(&document), retained);
        assert_eq!(pixels(document.preview()), rendered);
    }

    #[test]
    fn invalid_changes_are_rejected_without_rendering() {
        let (mut document, fail) = fallible_document();
        let preview = document.preview().id();
        let retained = bounds(&document);
        fail.set(true);

        assert_matches!(
            assert_err!(document.apply(Change::Remove(2))),
            ApplyError::Rejected
        );
        assert_eq!(bounds(&document), retained);
        assert_eq!(document.preview().id(), preview);
    }

    #[test]
    fn cancellation_restores_geometry_even_when_rendering_fails() {
        let (mut document, fail) = fallible_document();
        let retained = bounds(&document);
        let rendered = pixels(document.preview());
        let moved = effect(Tool::Pixelate, 1.0);
        let Change::Replace(index, original) =
            assert_ok!(document.apply(Change::Replace(0, moved)))
        else {
            panic!("expected the inverse replacement");
        };
        let preview = document.preview().id();
        let moved_pixels = pixels(document.preview());
        assert_ne!(moved_pixels, rendered);
        fail.set(true);

        assert_matches!(
            assert_err!(document.restore_edit(index, original)),
            Error::Image(tvert_image::Error::InvalidPixelationBounds)
        );
        assert_eq!(bounds(&document), retained);
        assert_eq!(document.preview().id(), preview);
        assert_eq!(pixels(document.preview()), moved_pixels);

        fail.set(false);
        assert_ok!(document.apply(Change::Replace(0, effect(Tool::Pixelate, 0.0))));
        assert_eq!(pixels(document.preview()), rendered);
    }
}
