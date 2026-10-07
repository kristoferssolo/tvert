mod annotation_canvas;
mod viewport;

use crate::annotation_canvas::AnnotationCanvas;
use iced::widget::canvas::Cache;
use iced::widget::{canvas, column, container, image, stack, text};
use iced::{ContentFit, Element, Fill, Theme, widget::image::Handle, window};
use iced::{Rectangle, Size};
use tvert_image::CapturedImage;

/// An editor window failure
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Iced could not start or run the editor
    #[error("failed to run the editor")]
    Runtime(#[from] iced::Error),
}

/// Opens an editor window containing the captured image.
///
/// Takes ownership of the pixels and runs until the window closes.
/// Call this on the main thread after closing the selection overlay.
///
/// # Errors
///
/// Returns [`Error::Runtime`] if Iced cannot start or run the window.
pub fn run(image: CapturedImage) -> Result<(), Error> {
    let size = image.size();
    let image_size = Size::from((size.width().get(), size.height().get()));
    let preview = Handle::from_rgba(size.width().get(), size.height().get(), image.into_rgba());

    iced::application(
        move || Editor::new(preview.clone(), image_size),
        Editor::update,
        Editor::view,
    )
    .title("Tvert")
    .theme(Theme::Dark)
    .window(window::Settings {
        platform_specific: window::settings::PlatformSpecific {
            application_id: "tvert".into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .window_size((1000.0, 700.0))
    .resizable(true)
    .run()?;
    Ok(())
}

struct Editor {
    preview: Handle,
    image_size: Size,
    rectangles: Vec<Rectangle>,
    annotation_cache: Cache,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    AddRectangle(Rectangle),
}

impl Editor {
    fn new(preview: Handle, image_size: Size) -> Self {
        Self {
            preview,
            image_size,
            rectangles: Vec::new(),
            annotation_cache: Cache::new(),
        }
    }
    fn update(&mut self, message: Message) {
        match message {
            Message::AddRectangle(rectangle) => {
                self.rectangles.push(rectangle);
                self.annotation_cache.clear();
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let picture = image(&self.preview)
            .width(Fill)
            .height(Fill)
            .content_fit(ContentFit::Contain);

        let annotations = canvas(AnnotationCanvas {
            image_size: self.image_size,
            rectangles: &self.rectangles,
            cache: &self.annotation_cache,
        })
        .width(Fill)
        .height(Fill);

        container(
            column![
                text("Rectangle"),
                stack![picture, annotations].width(Fill).height(Fill),
                text("Drag to draw. Right-click or Esc to cancel the current drag.").size(14),
            ]
            .spacing(12),
        )
        .padding(16)
        .width(Fill)
        .height(Fill)
        .into()
    }
}
