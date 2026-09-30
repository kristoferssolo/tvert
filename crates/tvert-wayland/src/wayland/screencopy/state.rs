use crate::{
    OutputId,
    wayland::shm::{ShmBuffer, ShmRequirements},
};
use wayland_client::WEnum;
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::{
    Flags, ZwlrScreencopyFrameV1,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ScreencopyStatus {
    #[default]
    Pending,
    ReadyToCopy,
    Copying,
    Complete,
    Failed,
}

#[derive(Debug, Default)]
pub struct ScreencopyState {
    pub output_id: Option<OutputId>,
    pub frame: Option<ZwlrScreencopyFrameV1>,
    pub buffer: Option<ShmRequirements>,
    pub shm_buffer: Option<ShmBuffer>,
    pub status: ScreencopyStatus,
    pub flags: Option<WEnum<Flags>>,
}

impl ScreencopyState {
    /// Cancels the retained capture if it belongs to the removed output
    pub fn cancel_for_output(&mut self, output_id: OutputId) {
        if self.output_id != Some(output_id) {
            return;
        }

        self.output_id = None;
        self.status = ScreencopyStatus::Failed;

        if let Some(frame) = self.frame.take() {
            frame.destroy();
        }

        drop(self.shm_buffer.take());
    }
}
