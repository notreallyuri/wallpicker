use serde::{Deserialize, Serialize};

use crate::error::Result;

/// One output as reported by `hyprctl monitors -j`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}
