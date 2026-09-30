use crate::{
    Error, OutputId, Wayland,
    wayland::{
        screencopy::state::{ScreencopyState, ScreencopyStatus},
        shm::ShmBuffer,
    },
};
use tvert_image::CapturedImage;
use wayland_client::{Proxy, WEnum, protocol::wl_shm::WlShm};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::Flags, zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

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
    ///
    /// Returns
    /// - [`Error::MissingGlobal`] if the screencopy manager is unavailable;
    /// - [`Error::UnknownOutput`] if the output is no longer tracked;
    /// - [`Error::ScreencopyFailed`] if the compositor rejects the frame;
    /// - [`Error::ScreencopyShmUnavailable`] if no SHM buffer is advertised;
    /// - [`Error::Dispatch`] if communicating with the compositor fails;
    /// - [`Error::UnsupportedShmFormat`] or [`Error::InvalidShmLayout`] if the advertised SHM requirements cannot be used;
    /// - [`Error::ShmIo`] if creating or sizing the backing file fails;
    /// - [`Error::InvalidScreencopyState`] if preparation encounters an inconsistent capture state.
    pub fn prepare_output_capture(&mut self, output_id: OutputId) -> Result<(), Error> {
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
            output_id: Some(output_id),
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

                    return Ok(());
                }
                ScreencopyStatus::Failed => return Err(Error::ScreencopyFailed),
                ScreencopyStatus::Copying | ScreencopyStatus::Complete => {
                    return Err(Error::InvalidScreencopyState);
                }
            }
        }
    }

    /// Captures an output and returns an owned RGBA8 image.
    ///
    /// Rows are ordered top-to-bottom without padding, and alpha is straight. The image remains
    /// valid after another capture or after this instance is dropped.
    ///
    /// Obtain `output_id` from this instance's [`Self::outputs`] method. The cursor is excluded.
    ///
    /// # Errors
    ///
    /// Returns
    /// - [`Error::MissingGlobal`] if a required protocol is unavailable, or [`Error::UnknownOutput`] if the output is no longer tracked;
    /// - [`Error::ScreencopyShmUnavailable`], [`Error::UnsupportedShmFormat`], or [`Error::InvalidShmLayout`] if the advertised buffer requirements cannot be used;
    /// - [`Error::ImageAllocation`] if allocating image storage fails.
    /// - [`Error::UnsupportedScreencopyFlags`] if the frame flags cannot be interpreted.
    /// - [`Error::ScreencopyFailed`] if the compositor rejects or aborts capture, or [`Error::Dispatch`] if communication fails;
    /// - [`Error::InvalidScreencopyState`] if the retained capture state is inconsistent.
    pub fn capture_output(&mut self, output_id: OutputId) -> Result<CapturedImage, Error> {
        self.prepare_output_capture(output_id)?;

        let capture = &mut self.state.screencopy;

        if capture.status != ScreencopyStatus::ReadyToCopy {
            return Err(Error::InvalidScreencopyState);
        }

        let Some(frame) = capture.frame.as_ref() else {
            return Err(Error::InvalidScreencopyState);
        };

        let Some(buffer) = capture.shm_buffer.as_ref() else {
            return Err(Error::InvalidScreencopyState);
        };

        frame.copy(buffer.proxy());
        capture.status = ScreencopyStatus::Copying;

        loop {
            match self.state.screencopy.status {
                ScreencopyStatus::Copying => {
                    self.event_queue.blocking_dispatch(&mut self.state)?;
                }
                ScreencopyStatus::Complete => {
                    let flags = match self.state.screencopy.flags {
                        Some(WEnum::Value(flags)) => flags,
                        Some(WEnum::Unknown(bits)) => {
                            return Err(Error::UnsupportedScreencopyFlags(bits));
                        }
                        None => return Err(Error::InvalidScreencopyState),
                    };
                    let y_inverted = flags.contains(Flags::YInvert);

                    let Some(buffer) = self.state.screencopy.shm_buffer.take() else {
                        return Err(Error::InvalidScreencopyState);
                    };

                    return buffer.read_image(y_inverted);
                }
                ScreencopyStatus::Failed => {
                    return Err(Error::ScreencopyFailed);
                }
                ScreencopyStatus::ReadyToCopy | ScreencopyStatus::Pending => {
                    return Err(Error::InvalidScreencopyState);
                }
            }
        }
    }
}
