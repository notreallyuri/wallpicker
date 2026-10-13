//! Version negotiation and one JSON batch/reply over the existing daemon socket.
use super::{Batch, Report, VERSION};
use crate::{Error, Result, cmd::Runner};
use serde_json::{Value, json};
use std::time::Duration;

pub fn send(runner: &dyn Runner, batch: &Batch) -> Result<Report> {
    send_with(runner, batch, &crate::daemon::ask, &std::thread::sleep)
}

pub(super) fn send_with(
    runner: &dyn Runner,
    batch: &Batch,
    ask: &dyn Fn(&Value) -> Result<Value>,
    sleep: &dyn Fn(Duration),
) -> Result<Report> {
    batch.validate()?;
    if batch.outputs.is_empty() {
        return Ok(Report::default());
    }
    let request = json!({ "cmd": "render", "batch": batch });
    if serde_json::to_vec(&request)?.len() >= 64 * 1024 {
        return Err(Error::Invalid("render batch exceeds daemon request size".into()));
    }
    let query = json!({"cmd":"capabilities"});
    let capabilities = match ask(&query) {
        Err(Error::DaemonNotRunning) => {
            let exe = std::env::current_exe()?;
            runner.spawn(&exe.to_string_lossy(), &["daemon"])?;
            let mut ready = None;
            for _ in 0..30 {
                match ask(&query) {
                    Err(Error::DaemonNotRunning) => sleep(Duration::from_millis(100)),
                    result => {
                        ready = Some(result);
                        break;
                    }
                }
            }
            ready.ok_or_else(|| Error::Invalid("started wallpicker daemon, but it did not become ready".into()))?
        }
        result => result,
    };
    let capabilities = match capabilities {
        Err(Error::Invalid(message)) if message.contains("unknown request") => return Err(restart()),
        result => result?,
    };
    if capabilities.get("render_version").and_then(Value::as_u64) != Some(u64::from(VERSION)) {
        return Err(restart());
    }
    let report: Report = serde_json::from_value(ask(&request)?)?;
    report.validate_for(batch)?;
    Ok(report)
}

fn restart() -> Error {
    Error::Invalid("the running wallpicker daemon uses a different render protocol; restart it after updating".into())
}
