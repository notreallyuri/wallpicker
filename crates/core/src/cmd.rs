use std::process::Command;

use crate::error::{Error, Result};

/// Runs external programs. A trait so tests can script `hyprctl` replies.
pub trait Runner {
    /// Run `program` with `args` and return its trimmed output.
    /// A non-zero exit status is an error.
    fn run(&self, program: &str, args: &[&str]) -> Result<String>;

    /// Where hyprpaper's IPC socket should be, used to tell "hyprpaper isn't
    /// running" apart from other failures. `None` skips that check.
    fn hyprpaper_socket(&self) -> Option<std::path::PathBuf> {
        None
    }
}

/// Runs real processes.
pub struct SystemRunner;

impl Runner for SystemRunner {
    fn hyprpaper_socket(&self) -> Option<std::path::PathBuf> {
        crate::hyprpaper::socket_path()
    }

    fn run(&self, program: &str, args: &[&str]) -> Result<String> {
        let cmd_text = || {
            if args.is_empty() {
                program.to_string()
            } else {
                format!("{program} {}", args.join(" "))
            }
        };

        let out = Command::new(program)
            .args(args)
            .output()
            .map_err(|e| Error::Command {
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
            return Err(Error::Command {
                cmd: cmd_text(),
                msg,
            });
        }

        Ok(if stdout.is_empty() { stderr } else { stdout })
    }
}
