use serde::{Deserialize, Serialize};

use crate::error::Result;

pub(crate) mod wayland;

/// Connected outputs, ordered left-to-right and then top-to-bottom.
pub fn list(runner: &dyn crate::cmd::Runner) -> Result<Vec<Monitor>> {
    runner.monitors()
}

/// Add compositor-specific metadata without replacing Wayland geometry.
pub(crate) fn enrich(monitors: &mut [Monitor], extra: &[Monitor]) {
    for monitor in monitors {
        if let Some(found) = extra.iter().find(|m| m.name == monitor.name) {
            monitor.focused = found.focused;
            monitor.active_workspace = found.active_workspace.clone();
        }
    }
}

/// One connected Wayland output, with optional compositor-specific metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Monitor {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
    #[serde(default)]
    pub focused: bool,
    #[serde(default)]
    pub disabled: bool,
    /// Its scale: `width`×`height` are pixels, `x`/`y` and the layout are in
    /// logical pixels (pixels / scale).
    #[serde(default = "one")]
    pub scale: f64,
    /// Wayland's transform: 1, 3, 5 and 7 turn it a quarter (width and
    /// height swap on screen).
    #[serde(default)]
    pub transform: i32,
    /// Its panel's size in millimetres, as it reports (0: unknown).
    #[serde(default, rename = "physicalWidth")]
    pub physical_width: u32,
    #[serde(default, rename = "physicalHeight")]
    pub physical_height: u32,
    /// The workspace it shows ("2"), when Hyprland says.
    #[serde(default, rename = "activeWorkspace", skip_serializing_if = "Option::is_none")]
    pub active_workspace: Option<WorkspaceRef>,
}

/// A workspace as Hyprland names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceRef {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub name: String,
}

fn one() -> f64 {
    1.0
}

/// Parse the JSON printed by `hyprctl monitors -j`.
///
/// Disabled outputs are dropped and the rest are ordered left-to-right, then
/// top-to-bottom, which matches how people think about "monitor 1, monitor 2".
pub fn parse_monitors(json: &str) -> Result<Vec<Monitor>> {
    let mut monitors: Vec<Monitor> = serde_json::from_str(json)?;
    monitors.retain(|m| !m.disabled);
    monitors.sort_by_key(|m| (m.x, m.y));
    Ok(monitors)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Trimmed from real `hyprctl monitors -j` output (extra fields are ignored).
    const SAMPLE: &str = r#"[
      {"id":1,"name":"DP-2","description":"Second","make":"X","model":"Y",
       "width":1920,"height":1080,"refreshRate":60.0,"x":1920,"y":0,
       "focused":false,"disabled":false,"scale":1.0},
      {"id":0,"name":"DP-1","description":"First","width":1920,"height":1080,
       "x":0,"y":0,"focused":true,"disabled":false},
      {"id":2,"name":"HDMI-A-1","width":1280,"height":720,"x":3840,"y":0,"disabled":true}
    ]"#;

    #[test]
    fn parses_sorts_and_drops_disabled() {
        let monitors = parse_monitors(SAMPLE).unwrap();
        let names: Vec<_> = monitors.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["DP-1", "DP-2"]);
        assert!(monitors[0].focused);
        assert_eq!(monitors[1].x, 1920);
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_monitors("not json").is_err());
    }

    #[test]
    fn discovery_uses_the_output_provider_without_running_hyprctl() {
        struct Native;
        impl crate::cmd::Runner for Native {
            fn run(&self, _: &str, _: &[&str]) -> Result<String> {
                panic!("native output discovery must not require hyprctl")
            }
            fn monitors(&self) -> Result<Vec<Monitor>> {
                parse_monitors(SAMPLE)
            }
        }
        assert_eq!(list(&Native).unwrap().len(), 2);
    }

    #[test]
    fn enrichment_matches_names_and_keeps_wayland_geometry() {
        let mut native = parse_monitors(SAMPLE).unwrap();
        for monitor in &mut native {
            monitor.focused = false;
        }
        let mut extra = native.clone();
        extra.reverse();
        extra[0].focused = true;
        extra[0].width = 999;
        extra[0].x = -999;
        extra[0].active_workspace = Some(WorkspaceRef { id: 3, name: "3".into() });
        enrich(&mut native, &extra);
        assert!(!native[0].focused);
        assert!(native[1].focused);
        assert_eq!(native[1].width, 1920);
        assert_eq!(native[1].x, 1920);
        assert_eq!(native[1].active_workspace.as_ref().unwrap().name, "3");
        assert!(native[0].active_workspace.is_none());
    }

    #[test]
    fn negative_output_positions_sort_before_positive_positions() {
        let monitors = parse_monitors(
            r#"[
            {"name":"right","x":0,"y":0},
            {"name":"lower-left","x":-1920,"y":1080},
            {"name":"upper-left","x":-1920,"y":-1080}
        ]"#,
        )
        .unwrap();
        assert_eq!(monitors.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), ["upper-left", "lower-left", "right"]);
    }
}
