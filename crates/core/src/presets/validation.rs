//! Reject invalid saved data rather than silently replacing it with defaults.

use super::{Collection, FORMAT, Preset};
use crate::{Error, Result};
use std::collections::BTreeSet;

pub(super) fn name(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 100 || value.chars().any(char::is_control) {
        return Err(Error::Invalid("preset name must contain 1–100 characters without control characters".into()));
    }
    Ok(value.to_owned())
}

pub(super) fn unique_name(items: &[Preset], value: &str, except: Option<&str>) -> Result<()> {
    if items.iter().any(|p| Some(p.id.as_str()) != except && p.name.to_lowercase() == value.to_lowercase()) {
        return Err(Error::Invalid(format!("a preset named {value:?} already exists")));
    }
    Ok(())
}

pub(super) fn collection(saved: &Collection) -> Result<()> {
    if saved.version != FORMAT {
        return Err(Error::Invalid(format!("unsupported preset format version {}", saved.version)));
    }
    let mut ids = BTreeSet::new();
    for item in &saved.items {
        let id = item.id.strip_prefix('p').and_then(|s| s.parse::<u64>().ok());
        if !id.is_some_and(|n| n > 0 && n < saved.next_id && format!("p{n}") == item.id) || !ids.insert(&item.id) {
            return Err(Error::Invalid("invalid or duplicate preset ID".into()));
        }
        unique_name(&saved.items, &item.name, Some(&item.id))?;
        preset(item)?;
    }
    if saved.next_id == 0 {
        return Err(Error::Invalid("invalid next preset ID".into()));
    }
    Ok(())
}

pub(super) fn preset(preset: &Preset) -> Result<()> {
    let bad = || Error::Invalid(format!("invalid wallpaper setup in preset {:?}", preset.name));
    if name(&preset.name)? != preset.name || preset.monitors.is_empty() {
        return Err(bad());
    }
    for (monitor, wallpaper) in &preset.monitors {
        if monitor.trim().is_empty() || wallpaper.wallpaper.as_os_str().is_empty() {
            return Err(bad());
        }
        if let Some(frame) = wallpaper.frame
            && (!(100..=400).contains(&frame.zoom) || frame.x > 1000 || frame.y > 1000 || wallpaper.region.is_some())
        {
            return Err(bad());
        }
        if let Some(region) = wallpaper.region
            && (region.width == 0
                || region.height == 0
                || region.x.checked_add(region.width).is_none()
                || region.y.checked_add(region.height).is_none())
        {
            return Err(bad());
        }
    }
    if let Some(span) = &preset.span {
        let names: BTreeSet<_> = span.monitors.iter().collect();
        if names.len() < 2
            || names.len() != span.monitors.len()
            || span.look != span.look.sanitized()
            || span.layout.len() != names.len()
        {
            return Err(bad());
        }
        for monitor in names {
            let Some(wallpaper) = preset.monitors.get(monitor) else {
                return Err(bad());
            };
            let Some(point) = span.layout.get(monitor) else {
                return Err(bad());
            };
            if wallpaper.wallpaper != span.wallpaper
                || wallpaper.frame.is_some()
                || wallpaper.region.is_some()
                || wallpaper.flip != span.look.flip
                || !point.x.is_finite()
                || !point.y.is_finite()
            {
                return Err(bad());
            }
        }
    }
    Ok(())
}
