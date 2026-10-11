use crate::{
    color::{format_hex, parse_hex},
    editor::Editor,
};
use iced::Color;
use iced_color_triangle::Selection;

impl Editor {
    pub(super) fn select_stroke_color(&mut self, color: Color) {
        let mut selection = Selection::from(color);
        if selection.saturation() <= f32::EPSILON {
            selection = selection.with_hue(self.stroke_selection.hue());
        }
        self.select_stroke_selection(selection);
    }

    pub(super) fn select_stroke_selection(&mut self, selection: Selection) {
        self.stroke_selection = selection;
        self.color_hex = format_hex(selection.into());
    }

    pub(super) fn edit_color_hex(&mut self, value: String) {
        if let Some(color) = parse_hex(&value) {
            self.select_stroke_color(color);
        }
        self.color_hex = value;
    }

    pub(super) fn close_color_palette(&mut self) {
        self.color_palette_open = false;
        self.color_hex = format_hex(self.stroke_selection.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Settings, editor::Message, style::RED};
    use iced::{Size, widget::image::Handle};
    use std::rc::Rc;

    fn editor() -> Editor {
        Editor::new(
            Handle::from_rgba(1, 1, vec![0, 0, 0, 255]),
            Size::new(1.0, 1.0),
            Settings {
                stroke_color: RED,
                stroke_width: 4,
                pixelation_block_size: 16,
                blur_sigma: 4,
            },
            Rc::new(|_, _| Ok(())),
        )
    }

    #[test]
    fn selection_changes_apply_immediately_and_survive_dismissal() {
        let mut editor = editor();
        let selection = editor.stroke_selection.with_hue(210.0).with_lightness(0.0);

        let _ = editor.update(Message::ToggleColorPalette);
        let _ = editor.update(Message::SelectStrokeSelection(selection));
        assert_eq!(editor.stroke_selection, selection);
        assert_eq!(editor.color_hex, "#000000");
        assert!(editor.color_palette_open);

        let _ = editor.update(Message::CloseColorPalette);
        let _ = editor.update(Message::ToggleColorPalette);
        assert_eq!(editor.stroke_selection, selection);

        let green = Color::from_rgb8(64, 200, 96);
        let _ = editor.update(Message::SelectStrokeColor(green));
        assert_eq!(
            Color::from(editor.stroke_selection).into_rgba8(),
            green.into_rgba8()
        );
        assert_eq!(editor.color_hex, "#40C860");
        assert!(editor.color_palette_open);
    }

    #[test]
    fn hex_edits_preserve_partial_text_and_keep_the_last_valid_color() {
        let mut editor = editor();
        let original = editor.stroke_selection;
        let _ = editor.update(Message::ToggleColorPalette);

        for value in ["", "#12", "ééé", "#GG0000"] {
            let _ = editor.update(Message::EditColorHex(value.into()));
            assert_eq!(editor.color_hex, value);
            assert_eq!(editor.stroke_selection, original);
        }

        let _ = editor.update(Message::EditColorHex(" 40c86080 ".into()));
        assert_eq!(editor.color_hex, " 40c86080 ");
        assert_eq!(
            Color::from(editor.stroke_selection).into_rgba8(),
            [64, 200, 96, 128]
        );
        assert!(editor.color_palette_open);

        let _ = editor.update(Message::EditColorHex("#40".into()));
        let _ = editor.update(Message::CloseColorPalette);
        assert_eq!(editor.color_hex, "#40C86080");
        assert_eq!(
            Color::from(editor.stroke_selection).into_rgba8(),
            [64, 200, 96, 128]
        );
        assert!(!editor.color_palette_open);
    }
}
