use crate::{
    OutputId,
    wayland::{
        overlay::{SuggestedSurfaceSize, size::SurfaceSize},
        shm::ShmBuffer,
    },
};
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_protocols::wp::viewporter::client::wp_viewport::WpViewport;
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::ZwlrLayerSurfaceV1;

#[derive(Debug, Clone, Copy)]
pub enum OverlayStatus {
    Waiting,
    Configured(SuggestedSurfaceSize),
    UnsupportedSize(SuggestedSurfaceSize),
    Cancelled,
    Closed,
}

/// Owns the overlay's protocol objects and retained display buffer
#[derive(Debug)]
pub struct OverlayState {
    pub output_id: OutputId,
    pub surface: WlSurface,
    pub layer_surface: ZwlrLayerSurfaceV1,
    pub viewport: WpViewport,
    pub status: OverlayStatus,
    pub buffer: Option<ShmBuffer>,
}

impl OverlayState {
    pub const fn output_id(&self) -> OutputId {
        self.output_id
    }

    pub fn set_destination(&self, size: SurfaceSize) {
        self.viewport.set_destination(size.width(), size.height());
    }

    pub fn is_surface(&self, surface: &WlSurface) -> bool {
        &self.surface == surface
    }

    pub const fn cancel(&mut self) {
        if self.buffer.is_none() {
            return;
        }

        if !matches!(self.status, OverlayStatus::Configured(_)) {
            return;
        }

        self.status = OverlayStatus::Cancelled;
    }
}

impl Drop for OverlayState {
    fn drop(&mut self) {
        self.viewport.destroy();
        self.layer_surface.destroy();
        self.surface.destroy();
    }
}
