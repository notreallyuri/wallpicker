//! Source ownership and presentation decisions used by the qualified Wayland
//! host and headless tests. All times are elapsed from one monotonic epoch;
//! callers inject time, so tests need neither sleeps nor a compositor.
mod pacing;
mod runtime;
mod session;
mod timeline;

pub use pacing::{Decision, HIDDEN_AFTER, OutputPacer, Target, Ticket, Visibility, visibility};
pub use runtime::{Host, Runtime, Signals};
pub use session::{PauseReason, SourceSession};
pub use timeline::{Advance, MAX_CATCH_UP, Timeline};

#[cfg(test)]
mod tests;
