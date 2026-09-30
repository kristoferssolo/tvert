use crate::{
    State,
    wayland::{screencopy::state::ScreencopyStatus, shm::ShmRequirements},
};
use wayland_client::{Dispatch, Proxy};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::{
    Event, ZwlrScreencopyFrameV1,
};

impl Dispatch<ZwlrScreencopyFrameV1, ()> for State {
    fn event(
        state: &mut Self,
        frame: &ZwlrScreencopyFrameV1,
        event: <ZwlrScreencopyFrameV1 as Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qh: &wayland_client::QueueHandle<Self>,
    ) {
        let capture = &mut state.screencopy;

        if capture.frame.as_ref() != Some(frame) {
            return;
        }

        match event {
            Event::Buffer {
                format,
                width,
                height,
                stride,
            } => {
                capture.buffer = Some(ShmRequirements {
                    format,
                    width,
                    height,
                    stride,
                });

                if frame.version() < 3 {
                    capture.status = ScreencopyStatus::ReadyToCopy;
                }
            }
            Event::BufferDone => {
                capture.status = ScreencopyStatus::ReadyToCopy;
            }
            Event::Flags { flags } => {
                capture.flags = Some(flags);
            }
            Event::Ready { .. } => {
                capture.status = ScreencopyStatus::Complete;

                if let Some(frame) = capture.frame.take() {
                    frame.destroy();
                }
            }
            Event::Failed => {
                capture.status = ScreencopyStatus::Failed;

                if let Some(frame) = capture.frame.take() {
                    frame.destroy();
                }

                drop(capture.shm_buffer.take());
            }
            _ => {}
        }
    }
}
