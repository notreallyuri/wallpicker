//! Read-only output snapshots; no renderer or layer-shell support required.

use super::Monitor;
use crate::error::{Error, Result};
use smithay_client_toolkit::output::{OutputHandler, OutputInfo, OutputState};
use smithay_client_toolkit::reexports::client::protocol::wl_output;
use smithay_client_toolkit::reexports::client::{Connection, QueueHandle, globals::registry_queue_init};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::{delegate_registry, registry_handlers};

struct Snapshot {
    registry: RegistryState,
    outputs: OutputState,
}

pub(crate) fn discover() -> Result<Vec<Monitor>> {
    let failure = |e: String| Error::Invalid(format!("Wayland monitor discovery: {e}"));
    let conn = Connection::connect_to_env().map_err(|e| failure(e.to_string()))?;
    let (globals, mut queue) = registry_queue_init::<Snapshot>(&conn).map_err(|e| failure(e.to_string()))?;
    let qh = queue.handle();
    let mut snapshot = Snapshot { registry: RegistryState::new(&globals), outputs: OutputState::new(&globals, &qh) };
    // Bind outputs, then receive their properties and xdg-output geometry.
    queue.roundtrip(&mut snapshot).map_err(|e| failure(e.to_string()))?;
    let mut monitors = Vec::new();
    for output in snapshot.outputs.outputs() {
        let Some(info) = snapshot.outputs.info(&output) else {
            continue;
        };
        if let Some(monitor) = convert(info)? {
            monitors.push(monitor);
        }
    }
    monitors.sort_by_key(|m| (m.x, m.y));
    Ok(monitors)
}

fn convert(info: OutputInfo) -> Result<Option<Monitor>> {
    // Outputs without a current mode are not enabled for displaying images.
    let Some(mode) = info.modes.iter().find(|m| m.current) else {
        return Ok(None);
    };
    let (w, h) = mode.dimensions;
    if w <= 0 || h <= 0 {
        return Ok(None);
    }
    let name = info.name.ok_or_else(|| {
        Error::Invalid("Wayland monitor discovery: output names require wl_output v4 or xdg-output v2".into())
    })?;
    let transform = match info.transform {
        wl_output::Transform::Normal => 0,
        wl_output::Transform::_90 => 1,
        wl_output::Transform::_180 => 2,
        wl_output::Transform::_270 => 3,
        wl_output::Transform::Flipped => 4,
        wl_output::Transform::Flipped90 => 5,
        wl_output::Transform::Flipped180 => 6,
        wl_output::Transform::Flipped270 => 7,
        _ => 0,
    };
    let pixels = if transform % 2 == 1 { (h, w) } else { (w, h) };
    let scale = scale(pixels, info.logical_size, info.scale_factor);
    let (x, y) = info.logical_position.unwrap_or(info.location);
    Ok(Some(Monitor {
        name,
        description: info.description.unwrap_or_else(|| format!("{} {}", info.make, info.model).trim().into()),
        width: w as u32,
        height: h as u32,
        x,
        y,
        focused: false,
        disabled: false,
        scale,
        transform,
        physical_width: info.physical_size.0.max(0) as u32,
        physical_height: info.physical_size.1.max(0) as u32,
        active_workspace: None,
    }))
}

fn scale(pixels: (i32, i32), logical: Option<(i32, i32)>, integer: i32) -> f64 {
    match logical {
        // xdg-output reports logical dimensions, including fractional scaling.
        // Integer logical sizes can introduce a small rounding error.
        Some((w, h)) if w > 0 && h > 0 => (f64::from(pixels.0) / f64::from(w)).min(f64::from(pixels.1) / f64::from(h)),
        _ => f64::from(integer.max(1)),
    }
}

impl OutputHandler for Snapshot {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.outputs
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl ProvidesRegistryState for Snapshot {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }
    registry_handlers![OutputState];
}

delegate_registry!(Snapshot);
smithay_client_toolkit::delegate_dispatch2!(Snapshot);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fractional_and_rotated_output_scale() {
        assert_eq!(scale((3840, 2160), Some((2560, 1440)), 2), 1.5);
        assert_eq!(scale((2160, 3840), Some((1440, 2560)), 2), 1.5);
        assert_eq!(scale((1920, 1080), None, 2), 2.0);
        assert_eq!(scale((1920, 1080), Some((0, 0)), 0), 1.0);
    }

    #[test]
    #[ignore = "requires a running Wayland session"]
    fn live_monitor_snapshot() {
        let monitors = discover().unwrap();
        assert!(!monitors.is_empty());
        for monitor in monitors {
            assert!(!monitor.name.is_empty());
            assert!(monitor.width > 0 && monitor.height > 0 && monitor.scale > 0.0);
            eprintln!("{monitor:?}");
        }
    }
}
