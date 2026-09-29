use crate::{Error, OutputId, State, Wayland, wayland::shm::ShmBuffer};
use wayland_client::{
    Dispatch, Proxy, WEnum,
    protocol::wl_shm::{Format, WlShm},
};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::{Event, ZwlrScreencopyFrameV1},
    zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

/// SHM buffer requirements advertised for a screencopy frame
#[derive(Debug, Clone, Copy)]
pub struct ShmRequirements {
    /// Pixel format, including formats unknown to this client
    pub format: WEnum<Format>,
    pub width: u32,
    pub height: u32,
    /// Number of bytes between consecutive rows
    pub stride: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum ScreencopyStatus {
    #[default]
    Pending,
    ReadyToCopy,
    Failed,
}

#[derive(Debug, Default)]
pub struct ScreencopyState {
    frame: Option<ZwlrScreencopyFrameV1>,
    buffer: Option<ShmRequirements>,
    shm_buffer: Option<ShmBuffer>,
    status: ScreencopyStatus,
}

impl Wayland {
    /// Prepares a screencopy frame and allocates its SHM buffer.
    ///
    /// Retains both objects for a subsequent copy request. Calling this again
    /// destroys the previously retained frame and buffer.
    ///
    /// Obtain `output_id` from this instance's [`Self::outputs`] method.
    /// The cursor is excluded from the requested frame.
    ///
    /// # Errors
    /// Returns
    /// - [`Error::MissingGlobal`] if the screencopy manager is unavailable;
    /// - [`Error::UnknownOutput`] if the output is no longer tracked;
    /// - [`Error::ScreencopyFailed`] if the compositor rejects the frame;
    /// - [`Error::ScreencopyShmUnavailable`] if no SHM buffer is advertised;
    /// - [`Error::Dispatch`] if communicating with the compositor fails.
    pub fn prepare_output_capture(
        &mut self,
        output_id: OutputId,
    ) -> Result<ShmRequirements, Error> {
        let Some(manager) = self.state.screencopy_manager.as_ref() else {
            return Err(Error::MissingGlobal {
                interface: ZwlrScreencopyManagerV1::interface().name,
            });
        };

        let Some(output) = self.state.output_states.get(&output_id) else {
            return Err(Error::UnknownOutput(output_id));
        };

        let queue_handle = self.event_queue.handle();
        let frame = manager.capture_output(0, &output.proxy, &queue_handle, ());

        if let Some(previous) = self.state.screencopy.frame.take() {
            previous.destroy();
        }

        self.state.screencopy = ScreencopyState {
            frame: Some(frame),
            ..Default::default()
        };

        loop {
            match self.state.screencopy.status {
                ScreencopyStatus::Pending => {
                    self.event_queue.blocking_dispatch(&mut self.state)?;
                }
                ScreencopyStatus::ReadyToCopy => {
                    let Some(requirements) = self.state.screencopy.buffer else {
                        if let Some(frame) = self.state.screencopy.frame.take() {
                            frame.destroy();
                        }
                        return Err(Error::ScreencopyShmUnavailable);
                    };

                    let Some(shm) = self.state.shm.as_ref() else {
                        return Err(Error::MissingGlobal {
                            interface: WlShm::interface().name,
                        });
                    };

                    let queue_handle = self.event_queue.handle();
                    let buffer =
                        ShmBuffer::new(shm, &self.state.shm_formats, requirements, &queue_handle)?;

                    self.state.screencopy.shm_buffer = Some(buffer);

                    self.event_queue.roundtrip(&mut self.state)?;

                    if self.state.screencopy.status == ScreencopyStatus::Failed {
                        return Err(Error::ScreencopyFailed);
                    }

                    return Ok(requirements);
                }
                ScreencopyStatus::Failed => return Err(Error::ScreencopyFailed),
            }
        }
    }
}

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
            Event::Failed => {
                capture.status = ScreencopyStatus::Failed;

                if let Some(frame) = capture.frame.take() {
                    frame.destroy();
                }
            }
            _ => {}
        }
    }
}
