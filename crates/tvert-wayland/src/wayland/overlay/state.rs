use crate::{
    Error, LogicalRect, OutputId,
    state::State,
    wayland::overlay::{OverlayBuffer, SuggestedSurfaceSize, size::SurfaceSize},
};
use wayland_client::protocol::{wl_callback::WlCallback, wl_surface::WlSurface};
use wayland_protocols::wp::viewporter::client::wp_viewport::WpViewport;
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::ZwlrLayerSurfaceV1;

#[derive(Debug, Clone, Copy)]
pub enum OverlayStatus {
    Waiting,
    Configured(SurfaceSize),
    UnsupportedSize(SuggestedSurfaceSize),
    Cancelled,
    Closed,
}

impl OverlayStatus {
    pub(super) const fn size(self) -> Result<Option<SurfaceSize>, Error> {
        match self {
            Self::Configured(size) => Ok(Some(size)),
            Self::UnsupportedSize(size) => Err(Error::UnsupportedOverlaySize {
                width: size.width,
                height: size.height,
            }),
            Self::Waiting | Self::Cancelled | Self::Closed => Ok(None),
        }
    }
}

/// Owns the overlay's protocol objects and retained display buffer
#[derive(Debug)]
pub struct OverlayState {
    pub surface: WlSurface,
    pub layer_surface: ZwlrLayerSurfaceV1,
    pub viewport: WpViewport,
    pub status: OverlayStatus,
    pub buffer: Option<OverlayBuffer>,
    pub frame_callback: Option<WlCallback>,
}

impl OverlayState {
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
}

impl State {
    /// Returns a selector surface's configured size and logical desktop area.
    ///
    /// Outputs without logical-output metadata use [`SurfaceSize::at_origin`].
    /// Returns `None` if the overlay is unknown or not configured.
    pub fn overlay_area(&self, output_id: OutputId) -> Option<(SurfaceSize, LogicalRect)> {
        let OverlayStatus::Configured(size) = self.overlays.get(&output_id)?.status else {
            return None;
        };
        let area = self
            .outputs
            .get(&output_id)
            .and_then(|output| output.logical)
            .unwrap_or_else(|| size.at_origin());
        Some((size, area))
    }
}

impl Drop for OverlayState {
    fn drop(&mut self) {
        self.viewport.destroy();
        self.layer_surface.destroy();
        self.surface.destroy();
    }
}
