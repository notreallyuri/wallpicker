//! Protocol dispatch, registry delegation, and shared-memory buffer releases.

use super::state::State;
use smithay_client_toolkit::dispatch2::Dispatch2;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::reexports::client::protocol::wl_buffer;
use smithay_client_toolkit::reexports::client::{Connection, QueueHandle};
use smithay_client_toolkit::reexports::protocols::wp::alpha_modifier::v1::client::wp_alpha_modifier_surface_v1::WpAlphaModifierSurfaceV1;
use smithay_client_toolkit::reexports::protocols::wp::alpha_modifier::v1::client::wp_alpha_modifier_v1::WpAlphaModifierV1;
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewport::WpViewport;
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewporter::WpViewporter;
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::shm::{Shm, ShmHandler};
use smithay_client_toolkit::{delegate_registry, registry_handlers};

impl ShmHandler for State {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ProvidesRegistryState for State {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }
    registry_handlers![OutputState];
}

/// A buffer's events: its release frees it.
pub(super) struct BufferData;

impl Dispatch2<wl_buffer::WlBuffer, State> for BufferData {
    fn event(
        &self,
        state: &mut State,
        buffer: &wl_buffer::WlBuffer,
        event: wl_buffer::Event,
        _: &Connection,
        _: &QueueHandle<State>,
    ) {
        if let wl_buffer::Event::Release = event {
            state.released(buffer);
        }
    }
}

/// Objects without events.
pub(super) struct Quiet;

impl Dispatch2<WpViewporter, State> for Quiet {
    fn event(
        &self,
        _: &mut State,
        _: &WpViewporter,
        _: <WpViewporter as smithay_client_toolkit::reexports::client::Proxy>::Event,
        _: &Connection,
        _: &QueueHandle<State>,
    ) {
    }
}

impl Dispatch2<WpAlphaModifierV1, State> for Quiet {
    fn event(
        &self,
        _: &mut State,
        _: &WpAlphaModifierV1,
        _: <WpAlphaModifierV1 as smithay_client_toolkit::reexports::client::Proxy>::Event,
        _: &Connection,
        _: &QueueHandle<State>,
    ) {
    }
}

impl Dispatch2<WpAlphaModifierSurfaceV1, State> for Quiet {
    fn event(
        &self,
        _: &mut State,
        _: &WpAlphaModifierSurfaceV1,
        _: <WpAlphaModifierSurfaceV1 as smithay_client_toolkit::reexports::client::Proxy>::Event,
        _: &Connection,
        _: &QueueHandle<State>,
    ) {
    }
}

impl Dispatch2<WpViewport, State> for Quiet {
    fn event(
        &self,
        _: &mut State,
        _: &WpViewport,
        _: <WpViewport as smithay_client_toolkit::reexports::client::Proxy>::Event,
        _: &Connection,
        _: &QueueHandle<State>,
    ) {
    }
}

delegate_registry!(State);
smithay_client_toolkit::delegate_dispatch2!(State);
