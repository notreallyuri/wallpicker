use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("image error: {0}")]
    Image(String),

    #[error("command `{cmd}` failed: {msg}")]
    Command { cmd: String, msg: String },

    #[error("hyprpaper: {0}")]
    Hyprpaper(String),

    #[error("hyprpaper is not running. Start it, and add it to your Hyprland autostart.")]
    HyprpaperNotRunning,

    #[error("wallpicker daemon isn't running")]
    DaemonNotRunning,

    #[error("{0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, Error>;
