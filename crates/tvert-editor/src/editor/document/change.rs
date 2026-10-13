use crate::edit::Edit;
use std::mem;

/// A reversible change to the ordered edits.
///
/// The history stacks hold the inverse of each applied change, so undoing
/// a deletion reinserts the edit at its original position.
#[derive(Debug)]
pub enum Change {
    Insert(usize, Edit),
    Remove(usize),
    Replace(usize, Edit),
}

impl Change {
    /// Applies the change and returns its inverse.
    ///
    /// An invalid index returns the rejected change without modifying edits.
    pub(super) fn apply(self, edits: &mut Vec<Edit>) -> Result<Self, Self> {
        match self {
            Self::Insert(index, edit) if index <= edits.len() => {
                edits.insert(index, edit);
                Ok(Self::Remove(index))
            }
            Self::Remove(index) if index < edits.len() => {
                Ok(Self::Insert(index, edits.remove(index)))
            }
            Self::Replace(index, edit) => match edits.get_mut(index) {
                Some(current) => Ok(Self::Replace(index, mem::replace(current, edit))),
                None => Err(Self::Replace(index, edit)),
            },
            change => Err(change),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{annotation::Annotation, figure::LineFigure, style::RED, viewport::Viewport};
    use claims::{assert_err, assert_some};
    use iced::{Point, Rectangle, Size, widget::canvas::Path};
    use rstest::rstest;

    const START: Point = Point::new(2.0, 3.0);
    const END: Point = Point::new(7.0, 11.0);

    fn edit() -> Edit {
        Edit::Annotation(Annotation {
            figure: Box::new(LineFigure::new(START, END)),
            stroke_width: 7,
            stroke_color: RED,
        })
    }

    #[rstest]
    #[case::insert_beyond_end(Change::Insert(2, edit()))]
    #[case::remove_at_end(Change::Remove(1))]
    #[case::replace_at_end(Change::Replace(1, edit()))]
    fn invalid_indices_leave_edits_unchanged(#[case] change: Change) {
        let mut edits = vec![edit()];

        let _ = assert_err!(change.apply(&mut edits));

        let [Edit::Annotation(annotation)] = edits.as_slice() else {
            panic!("the retained annotation changed");
        };
        assert_eq!(annotation.stroke_width, 7);
        assert_eq!(annotation.stroke_color, RED);

        let unit = Size::new(1.0, 1.0);
        let viewport = assert_some!(Viewport::new(unit, Rectangle::with_size(unit)));
        let actual = annotation.figure.path(viewport);
        let expected = Path::line(START, END);
        assert_eq!(
            actual.raw().iter().collect::<Vec<_>>(),
            expected.raw().iter().collect::<Vec<_>>()
        );
    }
}
