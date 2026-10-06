use crate::{
    OutputId,
    wayland::overlay::{
        OverlayBuffer, SelectionState, SuggestedSurfaceSize, SurfacePosition, SurfaceRect,
        size::SurfaceSize,
    },
};
use wayland_client::protocol::{wl_callback::WlCallback, wl_surface::WlSurface};
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
    pub buffer: Option<OverlayBuffer>,
    pub pointer_position: Option<SurfacePosition>,
    pub selection: SelectionState,
    pub frame_callback: Option<WlCallback>,
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

    /// Cancels an interactive overlay
    pub const fn cancel(&mut self) {
        if !self.is_interactive() {
            return;
        }

        self.status = OverlayStatus::Cancelled;
    }

    pub const fn is_interactive(&self) -> bool {
        self.buffer.is_some() && matches!(self.status, OverlayStatus::Configured(_))
    }

    /// Returns the selection rectangle within the configured surface bounds.
    ///
    /// Returns `None` if the overlay is not interactive or the selection is empty.
    pub fn selection_rectangle(&self) -> Option<SurfaceRect> {
        if !self.is_interactive() {
            return None;
        }

        let OverlayStatus::Configured(suggested) = self.status else {
            return None;
        };

        let bounds = SurfaceSize::try_from(suggested).ok()?;
        self.selection.rectangle(bounds)
    }
}

impl Drop for OverlayState {
    fn drop(&mut self) {
        self.viewport.destroy();
        self.layer_surface.destroy();
        self.surface.destroy();
    }
}
