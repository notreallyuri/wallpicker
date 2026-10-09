//! wallpicker's own renderer (the wallpicker backend): `wallpicker daemon`
//! draws each monitor's wallpaper on the background layer (wlr-layer-shell)
//! from shared memory, at the output's own pixels (`wp_viewporter` scales
//! the buffer to the surface; without it, the integer buffer scale). Its own
//! thread and event loop (calloop): idle, it sleeps.
//!
//! Pictures are decoded and fitted on a worker thread (compose.rs). The
//! decoded picture is kept a few seconds, for the next monitor (applying to
//! all of them asks for each in turn). A buffer is let go once the
//! compositor releases it (it has its own copy by then), so nothing is held
//! between changes: a new size or scale reads the file again. A monitor that
//! appears shows what the saved state has for it.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime};

use serde_json::{Value, json};
use smithay_client_toolkit::compositor::{CompositorHandler, CompositorState};
use smithay_client_toolkit::dispatch2::Dispatch2;
use smithay_client_toolkit::output::{OutputHandler, OutputInfo, OutputState};
use smithay_client_toolkit::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay_client_toolkit::reexports::calloop::{EventLoop, LoopHandle, RegistrationToken};
use smithay_client_toolkit::reexports::calloop::channel::{self, Channel, Sender};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::reexports::client::globals::registry_queue_init;
use smithay_client_toolkit::reexports::client::protocol::{wl_buffer, wl_output, wl_shm, wl_surface};
use smithay_client_toolkit::reexports::client::{Connection, QueueHandle};
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewport::WpViewport;
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewporter::WpViewporter;
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{
    Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure,
};
use smithay_client_toolkit::shm::raw::RawPool;
use smithay_client_toolkit::shm::{Shm, ShmHandler};
use smithay_client_toolkit::{delegate_registry, registry_handlers};
use wallpicker_core::compose;
use wallpicker_core::compose::RgbImage;
use wallpicker_core::settings::FitMode;

/// A file to show and its fit.
pub type Picture = (PathBuf, Option<FitMode>);

/// What a monitor shows in the saved state, as the renderer gets it
/// (`hyprpaper::saved_for`).
pub type Saved = Arc<dyn Fn(&str) -> Option<Picture> + Send + Sync>;

/// The last picture decoded (its file, as modified then), for the next
/// monitor showing it.
type Recent = Arc<Mutex<Option<(PathBuf, Option<SystemTime>, Arc<RgbImage>)>>>;

/// How long [`Recent`] keeps it: a decoded 8K picture is 130 MB.
const RECENT_FOR: Duration = Duration::from_secs(3);

/// How a show went: Ok once the monitor has the picture.
type Reply = mpsc::Sender<Result<(), String>>;

enum Msg {
    Show { monitor: String, picture: Picture, reply: Reply },
    Status(mpsc::Sender<Value>),
    Stop,
    /// A worker's frame for screen `id`, made for its `generation`; None
    /// when there was nothing to show (no saved picture).
    Made { id: u32, generation: u64, made: Result<Option<Made>, String> },
}

struct Made {
    picture: Picture,
    /// Its size in pixels.
    size: (u32, u32),
    pixels: Vec<u8>,
}

/// The running renderer. Dropped, it stops.
pub struct Renderer {
    tx: Sender<Msg>,
    thread: Option<JoinHandle<()>>,
}

impl Renderer {
    /// Connect to the Wayland session and start drawing (each monitor what
    /// `saved` says it shows). An error when there's no session, or it
    /// lacks the layer shell.
    pub fn start(saved: Saved) -> Result<Renderer, String> {
        map_big_allocations();
        let (tx, rx) = channel::channel();
        let loop_tx = tx.clone();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("renderer".into())
            .spawn(move || {
                let run = || -> Result<(), String> {
                    let (mut event_loop, mut state) = setup(rx, loop_tx, saved)?;
                    let _ = ready_tx.send(Ok(()));
                    while state.running {
                        event_loop.dispatch(None, &mut state).map_err(|e| e.to_string())?;
                    }
                    Ok(())
                };
                if let Err(e) = run() {
                    // Either it never started (start() says so) or it stopped.
                    if ready_tx.send(Err(e.clone())).is_err() {
                        eprintln!("wallpicker: the renderer stopped: {e}");
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Renderer { tx, thread: Some(thread) }),
            Ok(Err(e)) => Err(e),
            Err(_) => Err("the renderer stopped while starting".into()),
        }
    }

    /// Whether it's still drawing (its connection may have gone).
    pub fn running(&self) -> bool {
        self.thread.as_ref().is_some_and(|t| !t.is_finished())
    }

    /// Show `picture` on `monitor`; the answer comes once it's up.
    pub fn show(&self, monitor: String, picture: Picture) -> Result<mpsc::Receiver<Result<(), String>>, String> {
        let (reply, answer) = mpsc::channel();
        self.tx.send(Msg::Show { monitor, picture, reply }).map_err(|_| "the renderer has stopped".to_string())?;
        Ok(answer)
    }

    /// Each monitor: its name, size, and the file it shows.
    pub fn status(&self) -> Result<Value, String> {
        let (reply, answer) = mpsc::channel();
        self.tx.send(Msg::Status(reply)).map_err(|_| "the renderer has stopped".to_string())?;
        answer.recv_timeout(Duration::from_secs(5)).map_err(|_| "the renderer didn't answer".to_string())
    }
}

impl Drop for Renderer {
    /// Its surfaces go with it: what's underneath shows again.
    fn drop(&mut self) {
        let _ = self.tx.send(Msg::Stop);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// What a screen shows: the saved picture (looked up when drawn), or a file.
#[derive(Clone)]
enum Want {
    Saved,
    File(Picture),
}

/// One output's background surface.
struct Screen {
    /// The output's global name.
    id: u32,
    name: String,
    output: wl_output::WlOutput,
    layer: LayerSurface,
    viewport: Option<WpViewport>,
    /// Its size on the layout (logical pixels), once configured.
    size: Option<(u32, u32)>,
    /// What it shows, drawn again at a new size or scale.
    shown: Want,
    /// A picture on its way: it's what's shown once drawn (a file that
    /// can't be read changes nothing).
    asked: Option<Picture>,
    /// Bumped by each frame asked for: one made for an older is dropped.
    generation: u64,
    /// The size in pixels of what it shows (None: nothing yet).
    drawn: Option<(u32, u32)>,
    /// Shows waiting for the picture to be up.
    waiting: Vec<Reply>,
}

impl Screen {
    fn answer(&mut self, result: Result<(), String>) {
        for reply in self.waiting.drain(..) {
            let _ = reply.send(result.clone());
        }
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        self.answer(Err(format!("{} was disconnected", self.name)));
        if let Some(v) = &self.viewport {
            v.destroy();
        }
    }
}

struct State {
    registry: RegistryState,
    outputs: OutputState,
    compositor: CompositorState,
    layer_shell: LayerShell,
    shm: Shm,
    viewporter: Option<WpViewporter>,
    qh: QueueHandle<State>,
    screens: Vec<Screen>,
    /// Buffers the compositor hasn't released yet, with their memory.
    in_flight: Vec<(wl_buffer::WlBuffer, RawPool)>,
    /// For the workers' frames.
    tx: Sender<Msg>,
    saved: Saved,
    recent: Recent,
    /// For the timer that forgets [`Recent`].
    handle: LoopHandle<'static, State>,
    forget: Option<RegistrationToken>,
    running: bool,
}

fn setup(rx: Channel<Msg>, tx: Sender<Msg>, saved: Saved) -> Result<(EventLoop<'static, State>, State), String> {
    let conn = Connection::connect_to_env().map_err(|e| format!("no Wayland session: {e}"))?;
    let (globals, queue) = registry_queue_init::<State>(&conn).map_err(|e| e.to_string())?;
    let qh = queue.handle();
    let missing = |what: &str| format!("the compositor has no {what}");
    let event_loop = EventLoop::<State>::try_new().map_err(|e| e.to_string())?;
    let state = State {
        registry: RegistryState::new(&globals),
        outputs: OutputState::new(&globals, &qh),
        compositor: CompositorState::bind(&globals, &qh).map_err(|_| missing("wl_compositor"))?,
        layer_shell: LayerShell::bind(&globals, &qh).map_err(|_| missing("layer shell (wlr-layer-shell)"))?,
        shm: Shm::bind(&globals, &qh).map_err(|_| missing("wl_shm"))?,
        viewporter: globals.bind::<WpViewporter, State, Quiet>(&qh, 1..=1, Quiet).ok(),
        qh,
        screens: Vec::new(),
        in_flight: Vec::new(),
        tx,
        saved,
        recent: Arc::default(),
        handle: event_loop.handle(),
        forget: None,
        running: true,
    };
    WaylandSource::new(conn, queue).insert(event_loop.handle()).map_err(|e| e.error.to_string())?;
    event_loop
        .handle()
        .insert_source(rx, |event, _, state: &mut State| match event {
            channel::Event::Msg(msg) => state.handle(msg),
            channel::Event::Closed => state.running = false,
        })
        .map_err(|e| e.error.to_string())?;
    Ok((event_loop, state))
}

impl State {
    fn handle(&mut self, msg: Msg) {
        match msg {
            Msg::Show { monitor, picture, reply } => {
                let Some(i) = self.screens.iter().position(|s| s.name == monitor) else {
                    let _ = reply.send(Err(format!("no monitor named {monitor}")));
                    return;
                };
                let screen = &mut self.screens[i];
                screen.asked = Some(picture);
                screen.waiting.push(reply);
                self.redraw(i, true);
            }
            Msg::Status(reply) => {
                let screens: Vec<Value> = self
                    .screens
                    .iter()
                    .map(|s| {
                        json!({
                            "monitor": s.name,
                            "size": s.size,
                            "pixels": s.drawn,
                            "file": match &s.shown { Want::File((p, _)) => Some(p), Want::Saved => None },
                        })
                    })
                    .collect();
                let _ = reply.send(json!({ "ok": true, "screens": screens }));
            }
            Msg::Stop => self.running = false,
            Msg::Made { id, generation, made } => self.made(id, generation, made),
        }
    }

    /// The pixels to draw for `screen`, and the buffer scale they're at.
    fn target(&self, screen: &Screen) -> Option<((u32, u32), i32)> {
        let (w, h) = screen.size?;
        let info = self.outputs.info(&screen.output);
        match (&screen.viewport, info.as_ref().and_then(mode_pixels)) {
            // The output's own pixels, scaled to the surface by the compositor.
            (Some(_), Some(px)) => Some((px, 1)),
            _ => {
                let scale = info.map_or(1, |i| i.scale_factor.max(1));
                Some(((w * scale as u32, h * scale as u32), scale))
            }
        }
    }

    /// Ask a worker for screen `i`'s frame: when `again`, or when its size
    /// in pixels isn't what it shows.
    fn redraw(&mut self, i: usize, again: bool) {
        let Some((size, _)) = self.target(&self.screens[i]) else {
            return; // Not configured yet: that redraws.
        };
        let screen = &mut self.screens[i];
        if !again && screen.drawn == Some(size) {
            return;
        }
        screen.generation += 1;
        let want = screen.asked.clone().map_or_else(|| screen.shown.clone(), Want::File);
        let (id, generation, name) = (screen.id, screen.generation, screen.name.clone());
        let (saved, recent, tx) = (self.saved.clone(), self.recent.clone(), self.tx.clone());
        std::thread::spawn(move || {
            let made = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| make(&name, want, size, &saved, &recent)))
                .unwrap_or_else(|_| Err("drawing it failed".into()));
            let _ = tx.send(Msg::Made { id, generation, made });
        });
    }

    fn made(&mut self, id: u32, generation: u64, made: Result<Option<Made>, String>) {
        let Some(i) = self.screens.iter().position(|s| s.id == id && s.generation == generation) else {
            return; // Gone, or something newer is on its way.
        };
        self.forget_recent_later();
        let made = match made {
            Ok(Some(made)) => made,
            Ok(None) => return self.screens[i].answer(Ok(())),
            Err(e) => {
                eprintln!("wallpicker: {}: {e}", self.screens[i].name);
                let screen = &mut self.screens[i];
                // What it showed stays; at its size, if that changed meanwhile.
                screen.asked = None;
                screen.answer(Err(e));
                return self.redraw(i, false);
            }
        };
        match self.draw(i, &made) {
            Ok(true) => {
                let screen = &mut self.screens[i];
                screen.asked = None;
                screen.shown = Want::File(made.picture);
                screen.drawn = Some(made.size);
                screen.answer(Ok(()));
            }
            // Made for a size it no longer has: again, at this one.
            Ok(false) => self.redraw(i, true),
            Err(e) => self.screens[i].answer(Err(e)),
        }
    }

    /// The decoded picture dropped [`RECENT_FOR`] after the last frame.
    fn forget_recent_later(&mut self) {
        if let Some(t) = self.forget.take() {
            self.handle.remove(t);
        }
        let timer = Timer::from_duration(RECENT_FOR);
        self.forget = self
            .handle
            .insert_source(timer, |_, _, state| {
                state.recent.lock().unwrap_or_else(|e| e.into_inner()).take();
                state.forget = None;
                TimeoutAction::Drop
            })
            .ok();
    }

    /// Put `made` on screen `i`, from a pool of its own (freed on release).
    /// False when it was made for another size.
    fn draw(&mut self, i: usize, made: &Made) -> Result<bool, String> {
        let ((w, h), (lw, lh), scale) = match (self.target(&self.screens[i]), self.screens[i].size) {
            (Some((px, scale)), Some(size)) if px == made.size => (px, size, scale),
            _ => return Ok(false),
        };
        let stride = w as usize * 4;
        let mut pool = RawPool::new(stride * h as usize, &self.shm).map_err(|e| e.to_string())?;
        pool.mmap().copy_from_slice(&made.pixels);
        let buffer = pool.create_buffer(0, w as i32, h as i32, stride as i32, wl_shm::Format::Xrgb8888, BufferData, &self.qh);
        let screen = &self.screens[i];
        let surface = screen.layer.wl_surface();
        match &screen.viewport {
            Some(v) => v.set_destination(lw as i32, lh as i32),
            None => surface.set_buffer_scale(scale),
        }
        surface.attach(Some(&buffer), 0, 0);
        surface.damage_buffer(0, 0, w as i32, h as i32);
        screen.layer.commit();
        self.in_flight.push((buffer, pool));
        Ok(true)
    }

    fn released(&mut self, buffer: &wl_buffer::WlBuffer) {
        if let Some(i) = self.in_flight.iter().position(|(b, _)| b == buffer) {
            let (buffer, _pool) = self.in_flight.swap_remove(i);
            buffer.destroy();
        }
    }
}

/// Big allocations mapped on their own, so they go back to the system when
/// freed: a frame's tens of MB (decoded, fitted, copied) would otherwise
/// stay with glibc's allocator, in the arena of the worker that made it,
/// for as long as the daemon runs (85 MB after a few changes, against 7).
fn map_big_allocations() {
    const M_MMAP_THRESHOLD: std::ffi::c_int = -3;
    unsafe extern "C" {
        fn mallopt(param: std::ffi::c_int, value: std::ffi::c_int) -> std::ffi::c_int;
    }
    // SAFETY: glibc's; a tuning knob, safe to set at any time.
    unsafe {
        mallopt(M_MMAP_THRESHOLD, 1 << 20);
    }
}

/// The output's current mode in pixels, as it stands (turned: tall).
fn mode_pixels(info: &OutputInfo) -> Option<(u32, u32)> {
    let (w, h) = info.modes.iter().find(|m| m.current)?.dimensions;
    let turned = matches!(
        info.transform,
        wl_output::Transform::_90 | wl_output::Transform::_270 | wl_output::Transform::Flipped90 | wl_output::Transform::Flipped270
    );
    let (w, h) = (u32::try_from(w).ok()?, u32::try_from(h).ok()?);
    Some(if turned { (h, w) } else { (w, h) })
}

/// A worker's job: the picture (`want`, the saved one looked up now) read
/// (or the one just read again) and fitted to `size`.
fn make(name: &str, want: Want, size: (u32, u32), saved: &Saved, recent: &Recent) -> Result<Option<Made>, String> {
    let picture = match want {
        Want::File(p) => p,
        Want::Saved => match saved(name) {
            Some(p) => p,
            None => return Ok(None),
        },
    };
    let src = decoded(&picture.0, recent)?;
    let pixels = compose::xrgb(&compose::compose(&src, size.0, size.1, picture.1));
    Ok(Some(Made { picture, size, pixels }))
}

/// The picture at `path`: the recent one if it's that file, unchanged since.
fn decoded(path: &std::path::Path, recent: &Recent) -> Result<Arc<RgbImage>, String> {
    let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    let lock = || recent.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((p, m, src)) = lock().as_ref()
        && p == path
        && *m == modified
    {
        return Ok(src.clone());
    }
    let src = Arc::new(compose::open(path).map_err(|e| e.to_string())?);
    *lock() = Some((path.to_path_buf(), modified, src.clone()));
    Ok(src)
}

impl OutputHandler for State {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.outputs
    }

    fn new_output(&mut self, _: &Connection, qh: &QueueHandle<Self>, output: wl_output::WlOutput) {
        let Some(info) = self.outputs.info(&output) else { return };
        let Some(name) = info.name.clone() else {
            eprintln!("wallpicker: an output without a name: not drawn on");
            return;
        };
        let surface = self.compositor.create_surface(qh);
        let viewport = self.viewporter.as_ref().map(|v| v.get_viewport(&surface, qh, Quiet));
        let layer = self.layer_shell.create_layer_surface(qh, surface, Layer::Background, Some("wallpicker"), Some(&output));
        layer.set_anchor(Anchor::all());
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer.commit();
        self.screens.push(Screen {
            id: info.id,
            name,
            output,
            layer,
            viewport,
            size: None,
            shown: Want::Saved,
            asked: None,
            generation: 0,
            drawn: None,
            waiting: Vec::new(),
        });
    }

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, output: wl_output::WlOutput) {
        // A new mode, scale or turn: drawn again if its pixels changed.
        if let Some(i) = self.screens.iter().position(|s| s.output == output) {
            self.redraw(i, false);
        }
    }

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, output: wl_output::WlOutput) {
        self.screens.retain(|s| s.output != output);
    }
}

impl LayerShellHandler for State {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, layer: &LayerSurface) {
        self.screens.retain(|s| &s.layer != layer);
    }

    fn configure(&mut self, _: &Connection, _: &QueueHandle<Self>, layer: &LayerSurface, configure: LayerSurfaceConfigure, _: u32) {
        let Some(i) = self.screens.iter().position(|s| &s.layer == layer) else { return };
        let (w, h) = configure.new_size;
        let screen = &mut self.screens[i];
        // Anchored to every edge, it's given the output's size; 0 only if
        // the compositor leaves it to us.
        let logical = self.outputs.info(&screen.output).and_then(|i| i.logical_size);
        let (w, h) = match (w, h, logical) {
            (0, _, Some((lw, lh))) | (_, 0, Some((lw, lh))) => (lw.max(1) as u32, lh.max(1) as u32),
            _ => (w.max(1), h.max(1)),
        };
        screen.size = Some((w, h));
        // What it shows, stretched to the new size until it's drawn again.
        if let (Some(v), Some(_)) = (&screen.viewport, screen.drawn) {
            v.set_destination(w as i32, h as i32);
            screen.layer.commit();
        }
        self.redraw(i, false);
    }
}

impl CompositorHandler for State {
    fn scale_factor_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, surface: &wl_surface::WlSurface, _: i32) {
        if let Some(i) = self.screens.iter().position(|s| s.layer.wl_surface() == surface) {
            self.redraw(i, false);
        }
    }
    fn transform_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: wl_output::Transform) {}
    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {}
    fn surface_enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
    fn surface_leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
}

impl ShmHandler for State {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ProvidesRegistryState for State {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }
    registry_handlers![OutputState];
}

/// A buffer's events: its release frees it.
struct BufferData;

impl Dispatch2<wl_buffer::WlBuffer, State> for BufferData {
    fn event(&self, state: &mut State, buffer: &wl_buffer::WlBuffer, event: wl_buffer::Event, _: &Connection, _: &QueueHandle<State>) {
        if let wl_buffer::Event::Release = event {
            state.released(buffer);
        }
    }
}

/// Objects without events.
struct Quiet;

impl Dispatch2<WpViewporter, State> for Quiet {
    fn event(&self, _: &mut State, _: &WpViewporter, _: <WpViewporter as smithay_client_toolkit::reexports::client::Proxy>::Event, _: &Connection, _: &QueueHandle<State>) {}
}

impl Dispatch2<WpViewport, State> for Quiet {
    fn event(&self, _: &mut State, _: &WpViewport, _: <WpViewport as smithay_client_toolkit::reexports::client::Proxy>::Event, _: &Connection, _: &QueueHandle<State>) {}
}

delegate_registry!(State);
smithay_client_toolkit::delegate_dispatch2!(State);
