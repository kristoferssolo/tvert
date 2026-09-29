use crate::{Error, OutputId, State, Wayland};
use tracing::debug;
use wayland_client::{Proxy, delegate_noop};
use wayland_protocols::ext::image_capture_source::v1::client::{
    ext_image_capture_source_v1::ExtImageCaptureSourceV1,
    ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
};

impl Wayland {
    /// Creates and retains an image source for an output on this connection.
    ///
    /// Replaces any previously retained source.
    /// Obtain `output_id` from this instance's [`Self::outputs`] method.
    ///
    /// # Errors
    /// Returns
    /// - [`Error::MissingGlobal`] if the output capture source manager was not discovered;
    /// - [`Error::UnknownOutput`] if the output is no longer tracked;
    /// - [`Error::Dispatch`] if communicating with the compositor fails.
    pub fn create_output_source(&mut self, output_id: OutputId) -> Result<(), Error> {
        let Some(manager) = self.state.output_capture_source_manager.as_ref() else {
            return Err(Error::MissingGlobal {
                interface: ExtOutputImageCaptureSourceManagerV1::interface().name,
            });
        };

        let Some(output) = self.state.output_states.get(&output_id) else {
            return Err(Error::UnknownOutput(output_id));
        };

        let queue_handle = self.event_queue.handle();
        let source = manager.create_source(&output.proxy, &queue_handle, ());

        if let Some((_, previous)) = self.state.capture_source.replace((output_id, source)) {
            previous.destroy();
        }
        self.event_queue.roundtrip(&mut self.state)?;

        if !self.state.output_states.contains_key(&output_id) {
            return Err(Error::UnknownOutput(output_id));
        }

        debug!(?output_id, "created output image capture source");

        Ok(())
    }
}

delegate_noop!(State: ignore ExtImageCaptureSourceV1);
