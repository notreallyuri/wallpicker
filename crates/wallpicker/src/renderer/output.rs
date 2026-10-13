//! Output discovery, removal, and physical pixel dimensions.

use super::image::Want;
use super::protocol::Quiet;
use super::screen::Screen;
use super::state::State;
use smithay_client_toolkit::output::{OutputHandler, OutputInfo, OutputState};
use smithay_client_toolkit::reexports::client::protocol::wl_output;
use smithay_client_toolkit::reexports::client::{Connection, QueueHandle};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer};

impl OutputHandler for State {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.outputs
    }

    fn new_output(&mut self, _: &Connection, qh: &QueueHandle<Self>, output: wl_output::WlOutput) {
        let Some(info) = self.outputs.info(&output) else {
            return;
        };
        let Some(name) = info.name.clone() else {
            eprintln!("wallpicker: an output without a name: not drawn on");
            return;
        };
        let surface = self.compositor.create_surface(qh);
        let viewport = self.viewporter.as_ref().map(|v| v.get_viewport(&surface, qh, Quiet));
        let layer =
            self.layer_shell
                .create_layer_surface(qh, surface, Layer::Background, Some("wallpicker"), Some(&output));
        layer.set_anchor(Anchor::all());
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer.commit();
        self.screens.push(Screen {
            animation: None,
            incoming: None,
            id: info.id,
            name,
            output,
            layer,
            viewport,
            size: None,
            shown: Want::Saved,
            asked: None,
            overlay: None,
            frame_pending: false,
            generation: 0,
            drawn: None,
            render_error: None,
            waiting: Vec::new(),
        });
    }

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, output: wl_output::WlOutput) {
        // A new mode, scale or turn: drawn again if its pixels changed.
        if let Some(i) = self.screens.iter().position(|s| s.output == output) {
            self.redraw(i, false);
            self.refresh_animation_transition(i);
        }
    }

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, output: wl_output::WlOutput) {
        if let Some(i) = self.screens.iter().position(|s| s.output == output) {
            self.disconnect_animation(i);
            self.screens.remove(i);
        }
    }
}

/// The output's current mode in pixels, as it stands (turned: tall).
pub(super) fn mode_pixels(info: &OutputInfo) -> Option<(u32, u32)> {
    let (w, h) = info.modes.iter().find(|m| m.current)?.dimensions;
    let turned = matches!(
        info.transform,
        wl_output::Transform::_90
            | wl_output::Transform::_270
            | wl_output::Transform::Flipped90
            | wl_output::Transform::Flipped270
    );
    let (w, h) = (u32::try_from(w).ok()?, u32::try_from(h).ok()?);
    Some(if turned { (h, w) } else { (w, h) })
}
