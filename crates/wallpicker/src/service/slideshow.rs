//! The slideshow from Settings: when it changes next, and Change now.

use crate::cli::slideshow::{self, Status};
use crate::{AppState, CmdResult, err, rt};

pub async fn status(state: &AppState) -> CmdResult<Status> {
    let store = state.store.clone();
    rt::spawn_blocking(move || slideshow::status_of(&store)).await.map_err(err)?
}

/// A change right away (the timer starts over).
pub async fn now(state: &AppState) -> CmdResult<()> {
    let store = state.store.clone();
    match rt::spawn_blocking(move || slideshow::tick_with(&store, true)).await.map_err(err)? {
        Ok(true) => Ok(()),
        Ok(false) => Err("Some monitors could not be changed.".into()),
        Err(e) => Err(e),
    }
}
