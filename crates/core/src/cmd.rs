use std::process::Command;

use crate::error::{Error, Result};

/// Runs external programs. A trait so tests can script `hyprctl` replies.
pub trait Runner {
    /// Apply a prepared wallpaper. Production uses the native daemon; tests inject a desktop.
    fn show_wallpaper(
        &self,
        monitor: &str,
        path: &std::path::Path,
        fit: Option<crate::settings::FitMode>,
        transition: &crate::settings::Transition,
    ) -> Result<()> {
        let _ = (monitor, path, fit, transition);
        Err(Error::Invalid("this runner cannot draw wallpapers".into()))
    }

    /// Discover outputs. Test runners can keep supplying Hyprland JSON.
    fn monitors(&self) -> Result<Vec<crate::monitors::Monitor>> {
        crate::monitors::parse_monitors(&self.run("hyprctl", &["monitors", "-j"])?)
    }

    /// Run `program` with `args` and return its trimmed output.
    /// A non-zero exit status is an error.
    fn run(&self, program: &str, args: &[&str]) -> Result<String>;

    /// Start `program` in the background and leave it running (a daemon).
    fn spawn(&self, program: &str, _args: &[&str]) -> Result<()> {
        Err(Error::Command {
            cmd: program.to_string(),
            msg: "can't start programs here".into(),
        })
    }
}

/// Runs real processes.
pub struct SystemRunner;

impl Runner for SystemRunner {
    fn show_wallpaper(
        &self,
        monitor: &str,
        path: &std::path::Path,
        fit: Option<crate::settings::FitMode>,
        transition: &crate::settings::Transition,
    ) -> Result<()> {
        crate::daemon::show(self, monitor, path, fit, transition)
    }

    fn monitors(&self) -> Result<Vec<crate::monitors::Monitor>> {
        let mut monitors = crate::monitors::wayland::discover()?;
        // Focus and active workspace are not exposed by wl_output. Only ask
        // Hyprland for that optional metadata inside a Hyprland session.
        if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
            && let Ok(extra) = self
                .run("hyprctl", &["monitors", "-j"])
                .and_then(|json| crate::monitors::parse_monitors(&json))
        {
            crate::monitors::enrich(&mut monitors, &extra);
        }
        Ok(monitors)
    }

    fn spawn(&self, program: &str, args: &[&str]) -> Result<()> {
        use std::os::unix::process::CommandExt;
        use std::process::Stdio;
        Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            // Its own process group: it outlives wallpicker (and the app).
            .process_group(0)
            .spawn()
            .map(drop)
            .map_err(|e| Error::Command {
                cmd: program.to_string(),
                msg: e.to_string(),
            })
    }

    fn run(&self, program: &str, args: &[&str]) -> Result<String> {
        let cmd_text = || {
            if args.is_empty() {
                program.to_string()
            } else {
                format!("{program} {}", args.join(" "))
            }
        };

        let out = Command::new(program).args(args).output().map_err(|e| Error::Command {
            cmd: cmd_text(),
            msg: e.to_string(),
        })?;

        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();

        if !out.status.success() {
            let msg = [stderr.as_str(), stdout.as_str()]
                .into_iter()
                .find(|s| !s.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| format!("exited with {}", out.status));
            return Err(Error::Command { cmd: cmd_text(), msg });
        }

        Ok(if stdout.is_empty() { stderr } else { stdout })
    }
}
