//! renderer.playback (what each monitor plays) and renderer.usage (the
//! daemon's and its decoders' GPU, CPU and memory: Settings → Performance).

use serde_json::Value;

use super::json;
use crate::CmdResult;
use crate::service::renderer;

pub(super) async fn call(name: &str) -> Option<CmdResult<Value>> {
    Some(match name {
        "playback" => json(renderer::playback().await),
        "usage" => json(renderer::usage().await),
        _ => return None,
    })
}
