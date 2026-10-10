use super::*;
/// Other programs drawing wallpapers besides wallpicker: two at once fight
/// over the screen (whichever drew last is on top). Names to show, like
/// "hyprpaper" or "Noctalia's wallpaper".
/// The wallpaper programs [`conflicts`] looks for, and [`stop_program`] stops.
const DAEMONS: [&str; 6] = [
    "hyprpaper",
    "awww-daemon",
    "swww-daemon",
    "swaybg",
    "mpvpaper",
    "wpaperd",
];

pub fn conflicts(runner: &dyn Runner) -> Vec<String> {
    let mut found: Vec<String> = DAEMONS
        .iter()
        .filter(|d| runner.run("pgrep", &["-x", d]).is_ok())
        .map(|d| d.to_string())
        .collect();
    if crate::noctalia::wallpaper_enabled_on_disk() == Some(true) {
        found.push("Noctalia's wallpaper".into());
    }
    found
}

/// Stop one of the programs [`conflicts`] found: awww's and swww's daemons
/// with their own `kill` (else by name), the others by name. Nothing outside
/// that list; Noctalia's wallpaper is switched off in Noctalia's settings.
pub fn stop_program(runner: &dyn Runner, name: &str) -> Result<()> {
    if !DAEMONS.contains(&name) {
        return Err(Error::Invalid(format!("wallpicker doesn't stop {name}")));
    }
    let own_kill = match name {
        "awww-daemon" => Some("awww"),
        "swww-daemon" => Some("swww"),
        _ => None,
    };
    if let Some(client) = own_kill
        && runner.run(client, &["kill"]).is_ok()
    {
        return Ok(());
    }
    runner.run("pkill", &["-x", name]).map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    #[derive(Default)]
    struct Mock(RefCell<Vec<String>>);
    impl Runner for Mock {
        fn run(&self, program: &str, args: &[&str]) -> Result<String> {
            self.0.borrow_mut().push(format!("{program} {}", args.join(" ")));
            if program == "awww" {
                Err(Error::Invalid("old client".into()))
            } else {
                Ok(String::new())
            }
        }
    }
    #[test]
    fn stopping_competitors_is_allowlisted_and_keeps_legacy_kill_fallback() {
        let mock = Mock::default();
        assert!(stop_program(&mock, "wallpicker").is_err());
        assert!(stop_program(&mock, "Noctalia's wallpaper").is_err());
        assert!(mock.0.borrow().is_empty());
        stop_program(&mock, "awww-daemon").unwrap();
        stop_program(&mock, "hyprpaper").unwrap();
        assert_eq!(
            *mock.0.borrow(),
            ["awww kill", "pkill -x awww-daemon", "pkill -x hyprpaper"]
        );
    }
}
