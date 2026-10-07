//! `wallpicker serve`: the Quickshell UI's backend, one process per open
//! window, speaking JSON lines over stdin and stdout.
//!
//! In: `{"id": 1, "method": "library.scan", "params": {...}}`.
//! Out: `{"id": 1, "result": ...}` or `{"id": 1, "error": "..."}`, as each
//! request finishes (several run at once, so answers can come in any order);
//! and, unasked, `{"event": "...", "data": ...}`.
//!
//! It ends when stdin closes (the UI went away). Methods are in `methods/`,
//! by area; they call `service/`, like the command line does.

mod methods;
mod watch;

use std::io::{BufRead, Write};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use wallpicker_core::store::{Dirs, Store};

use crate::AppState;

/// Where answers and events go: one line each, never interleaved.
#[derive(Clone)]
pub struct Out(Arc<Mutex<std::io::Stdout>>);

impl Out {
    fn line(&self, value: &Value) {
        let mut out = self.0.lock().unwrap_or_else(|p| p.into_inner());
        // The UI gone mid-answer: nothing to do about it.
        let _ = writeln!(out, "{value}");
        let _ = out.flush();
    }

    /// Something the UI didn't ask for (state changed, ...).
    pub fn event(&self, event: &str, data: Value) {
        self.line(&json!({ "event": event, "data": data }));
    }
}

pub fn run() -> i32 {
    let dirs = Dirs::from_env();
    let state = Arc::new(AppState::new(Store::new(dirs.clone())));
    let out = Out(Arc::new(Mutex::new(std::io::stdout())));
    out.event("ready", json!({ "version": env!("CARGO_PKG_VERSION") }));
    watch::start(&dirs, out.clone());
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let (state, out) = (state.clone(), out.clone());
        crate::rt::spawn(async move {
            let reply = answer(&state, &out, &line).await;
            out.line(&reply);
        });
    }
    0
}

async fn answer(state: &AppState, out: &Out, line: &str) -> Value {
    let request: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => return json!({ "id": null, "error": format!("not JSON: {e}") }),
    };
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let Some(method) = request.get("method").and_then(Value::as_str) else {
        return json!({ "id": id, "error": "no method" });
    };
    let params = request.get("params").cloned().unwrap_or(json!({}));
    match methods::call(state, out, method, params).await {
        Ok(result) => json!({ "id": id, "result": result }),
        Err(e) => json!({ "id": id, "error": e }),
    }
}
