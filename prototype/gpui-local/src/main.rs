//! The Local tab, in GPUI: a prototype to compare with the WebKit app
//! (memory, scrolling, looks) before deciding on a port. Same core crate:
//! the library scan, thumbnails, favorites and tags, and applying, as the
//! app does them. Not a replacement: no settings, Online, monitors, menus.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::component::button::*;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::*;
use gpui_kit::*;
use wallpicker_core::catalog::Catalog;
use wallpicker_core::cmd::SystemRunner;
use wallpicker_core::hyprpaper;
use wallpicker_core::library::{self, LocalImage};
use wallpicker_core::store::{Dirs, Store};
use wallpicker_core::thumbs;

/// Blossom, dark (the app's default theme): globals.css's oklch values.
const BACKGROUND: u32 = 0x18141e;
const FOREGROUND: u32 = 0xf2ecf4;
const CARD: u32 = 0x221c29;
const PRIMARY: u32 = 0xf1a1ce;
const PRIMARY_FOREGROUND: u32 = 0x351929;
const MUTED: u32 = 0x2c2535;
const MUTED_FOREGROUND: u32 = 0xb4a9bd;
/// Tailwind's yellow-400 (a favorite's star).
const STAR: u32 = 0xfacc15;
/// `color` with an alpha (0-255), like Tailwind's `bg-card/80`.
fn with_alpha(color: u32, alpha: u8) -> Rgba {
    rgba((color << 8) | u32::from(alpha))
}

/// The narrowest a tile gets before a column goes (the app's grid: minmax(220px, 1fr)).
const TILE_MIN: f32 = 220.0;
const GAP: f32 = 16.0;
/// Decoded thumbnails kept; the rest are freed (the WebKit app keeps all).
const IMAGES_KEPT: usize = 150;

struct Local {
    store: Store,
    images: Vec<LocalImage>,
    /// Each image's thumbnail, once made (same cache as the app).
    thumbs: HashMap<PathBuf, PathBuf>,
    catalog: Catalog,
    /// What each monitor shows (for the badges).
    current: HashMap<PathBuf, Vec<String>>,
    /// Indexes into `images` that pass the search, favorites first.
    shown: Vec<usize>,
    query: String,
    favorites_only: bool,
    search: Entity<InputState>,
    image_cache: Entity<LruImages>,
    status: SharedString,
    columns: usize,
    scroll: UniformListScrollHandle,
    /// Where a wheel scroll is gliding to (offset y, negative going down).
    glide_to: Option<f32>,
    /// The loop moving it there, while one runs.
    gliding: Option<Task<()>>,
}

impl Local {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let store = Store::new(Dirs::from_env());
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Filter by name or tag"));
        cx.subscribe_in(&search, window, |this, state, event: &InputEvent, _, cx| {
            if let InputEvent::Change = event {
                this.query = state.read(cx).value().to_string();
                this.filter();
                cx.notify();
            }
        })
        .detach();

        // The library and its thumbnails, off the UI thread.
        let scan_store = store.clone();
        cx.spawn(async move |this, cx| {
            let (images, catalog, current) = cx
                .background_spawn(async move {
                    let settings = scan_store.load_settings().unwrap_or_default();
                    let mut current: HashMap<PathBuf, Vec<String>> = HashMap::new();
                    for (m, p) in scan_store.load_state().unwrap_or_default().current {
                        current.entry(p).or_default().push(m);
                    }
                    (library::scan(&settings.folders, settings.recursive), scan_store.load_catalog().unwrap_or_default(), current)
                })
                .await;
            let paths: Vec<PathBuf> = images.iter().map(|i| i.path.clone()).collect();
            let _ = this.update(cx, |this, cx| {
                this.images = images;
                this.catalog = catalog;
                this.current = current;
                this.filter();
                cx.notify();
            });
            // Made (or found) in batches, shown as they come.
            let thumb_store = scan_store_clone(&this, cx);
            for chunk in paths.chunks(24) {
                let chunk = chunk.to_vec();
                let store = thumb_store.clone();
                let made: Vec<(PathBuf, PathBuf)> = cx
                    .background_spawn(async move {
                        let width = store.load_settings().map(|s| s.thumb_width).unwrap_or(360);
                        chunk
                            .into_iter()
                            .filter_map(|p| thumbs::ensure_thumb(&store.dirs().cache, &p, width).ok().map(|t| (p, t)))
                            .collect()
                    })
                    .await;
                let _ = this.update(cx, |this, cx| {
                    this.thumbs.extend(made);
                    cx.notify();
                });
            }
        })
        .detach();

        // For measuring only: WALLPICKER_AUTOSCROLL=1 scrolls through the
        // whole library, a row at a time, then back up.
        if std::env::var("WALLPICKER_AUTOSCROLL").as_deref() == Ok("wheel") {
            // Wheel notches through the real handler, as a mouse sends them.
            cx.spawn_in(window, async move |this, cx| {
                cx.background_executor().timer(std::time::Duration::from_secs(4)).await;
                for i in 0..60 {
                    let down = i < 40;
                    let event = ScrollWheelEvent {
                        delta: ScrollDelta::Lines(point(0., if down { -3. } else { 3. })),
                        ..Default::default()
                    };
                    let _ = this.update_in(cx, |this, window, cx| this.wheel(&event, window, cx));
                    cx.background_executor().timer(std::time::Duration::from_millis(150)).await;
                }
                eprintln!("wheel test: done");
            })
            .detach();
        } else if std::env::var_os("WALLPICKER_AUTOSCROLL").is_some() {
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(std::time::Duration::from_secs(4)).await;
                for pass in 0..2 {
                    let rows = this.read_with(cx, |this, _| this.shown.len().div_ceil(this.columns)).unwrap_or(0);
                    for row in 0..rows {
                        let row = if pass == 0 { row } else { rows - 1 - row };
                        let _ = this.update(cx, |this, cx| {
                            this.scroll.scroll_to_item(row, ScrollStrategy::Top);
                            cx.notify();
                        });
                        cx.background_executor().timer(std::time::Duration::from_millis(400)).await;
                    }
                }
            })
            .detach();
        }

        Local {
            store,
            images: Vec::new(),
            thumbs: HashMap::new(),
            catalog: Catalog::default(),
            current: HashMap::new(),
            shown: Vec::new(),
            query: String::new(),
            favorites_only: false,
            search,
            image_cache: cx.new(|_| LruImages::new(IMAGES_KEPT)),
            status: "".into(),
            columns: 4,
            scroll: UniformListScrollHandle::new(),
            glide_to: None,
            gliding: None,
        }
    }

    /// What passes the search and the favorites switch, favorites first,
    /// then the newest (as the app sorts by default).
    fn filter(&mut self) {
        let q = self.query.trim().to_lowercase();
        let c = &self.catalog;
        let mut shown: Vec<usize> = (0..self.images.len())
            .filter(|&i| {
                let img = &self.images[i];
                (!self.favorites_only || c.is_favorite(&img.path))
                    && (q.is_empty()
                        || img.name.to_lowercase().contains(&q)
                        || c.tags(&img.path).iter().any(|t| t.to_lowercase().contains(&q)))
            })
            .collect();
        shown.sort_by(|&a, &b| {
            let (a, b) = (&self.images[a], &self.images[b]);
            c.is_favorite(&b.path).cmp(&c.is_favorite(&a.path)).then(b.modified.cmp(&a.modified))
        });
        self.shown = shown;
    }

    /// Put it on the focused monitor, like a click in the app.
    fn apply(&mut self, index: usize, cx: &mut Context<Self>) {
        let path = self.images[index].path.clone();
        let store = self.store.clone();
        self.status = format!("Applying {}…", self.images[index].name).into();
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let backend = store.load_settings().map(|s| s.wallpaper_backend).unwrap_or_default();
                    let monitors = hyprpaper::list_monitors(&SystemRunner)?;
                    let focused = monitors.iter().find(|m| m.focused).or(monitors.first()).map(|m| m.name.clone());
                    let Some(m) = focused else { return Err(wallpicker_core::Error::Invalid("no monitor".into())) };
                    hyprpaper::apply_and_remember_with(&SystemRunner, &store, backend, &[m.clone()], &path)?;
                    Ok::<_, wallpicker_core::Error>((m, path))
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.status = match result {
                    Ok((m, path)) => {
                        for shown in this.current.values_mut() {
                            shown.retain(|x| *x != m);
                        }
                        this.current.entry(path).or_default().push(m.clone());
                        format!("Applied to {m}").into()
                    }
                    Err(e) => format!("Could not apply: {e}").into(),
                };
                cx.notify();
            });
        })
        .detach();
    }

    /// Star or unstar, through the same catalog the app uses.
    fn toggle_favorite(&mut self, index: usize, cx: &mut Context<Self>) {
        let path = self.images[index].path.clone();
        let on = !self.catalog.is_favorite(&path);
        match self.store.update_catalog(|c| c.set_favorite(&path, on)) {
            Ok(c) => self.catalog = c,
            Err(e) => self.status = format!("Could not save: {e}").into(),
        }
        self.filter();
        cx.notify();
    }

    /// The WebKit app's tile (views/local/tile.rs), as close as GPUI goes:
    /// a rounded card with the picture inset, the monitors showing it, the
    /// name and size, the tags, and the star. No hover transitions (GPUI
    /// switches styles at once) and no zoom on the picture.
    fn tile(&self, index: usize, tile_w: f32, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let info = &self.images[index];
        let favorite = self.catalog.is_favorite(&info.path);
        let tags = self.catalog.tags(&info.path).to_vec();
        let shown_on = self.current.get(&info.path).cloned().unwrap_or_default();
        let thumb = self.thumbs.get(&info.path).cloned();
        let name: SharedString = info.name.clone().into();
        let size = (info.width > 0).then(|| format!("{}×{}", info.width, info.height));
        // p-1.5 around the picture; the picture is 16:10.
        let inner_w = tile_w - 2.0 * 6.0 - 2.0 * 2.0;
        let picture_h = inner_w * 10.0 / 16.0;
        let group: SharedString = format!("tile-{index}").into();

        div()
            .id(("tile", index))
            .group(group.clone())
            .relative()
            .w(px(tile_w))
            .flex_none()
            .p(px(6.))
            .rounded(px(24.))
            .bg(with_alpha(CARD, 0xcc))
            .border_2()
            .border_color(with_alpha(FOREGROUND, 0x0d))
            .shadow(vec![BoxShadow {
                color: with_alpha(0x000000, 0x1a).into(),
                offset: point(px(0.), px(4.)),
                blur_radius: px(6.),
                spread_radius: px(-1.),
                inset: false,
            }])
            .cursor_pointer()
            // Lifts, rings and glows on hover (the app eases it; GPUI snaps).
            .hover(|s| {
                s.top(px(-4.)).border_color(with_alpha(PRIMARY, 0xb3)).shadow(vec![BoxShadow {
                    color: with_alpha(PRIMARY, 0x33).into(),
                    offset: point(px(0.), px(10.)),
                    blur_radius: px(15.),
                    spread_radius: px(-3.),
                    inset: false,
                }])
            })
            .on_click(cx.listener(move |this, _, _, cx| this.apply(index, cx)))
            .child(
                // An exact size: a picture that isn't 16:10 would otherwise
                // grow to its own height over the name.
                div()
                    .relative()
                    .w(px(inner_w))
                    .h(px(picture_h))
                    .overflow_hidden()
                    .rounded(px(17.6))
                    .bg(rgb(MUTED))
                    .children(thumb.map(|t| {
                        img(t)
                            .absolute()
                            .top_0()
                            .left_0()
                            .w(px(inner_w))
                            .h(px(picture_h))
                            .rounded(px(17.6))
                            .object_fit(ObjectFit::Cover)
                    }))
                    .child(h_flex().absolute().top_2().left_2().gap_1().children(shown_on.into_iter().map(|m| {
                        div()
                            .px_2()
                            .rounded(px(8.))
                            .bg(rgb(PRIMARY))
                            .text_color(rgb(PRIMARY_FOREGROUND))
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(m)
                    }))),
            )
            .child(
                v_flex()
                    .h(px(50.))
                    .px_3()
                    .py_2()
                    .gap_1()
                    .text_xs()
                    .child(
                        h_flex()
                            .gap_2()
                            .justify_between()
                            .items_baseline()
                            .child(div().min_w_0().truncate().text_color(rgb(MUTED_FOREGROUND)).child(name))
                            .children(size.map(|s| {
                                div().flex_none().text_size(px(10.)).text_color(with_alpha(MUTED_FOREGROUND, 0xb3)).child(s)
                            })),
                    )
                    .child(h_flex().gap_1().overflow_hidden().children(tags.into_iter().map(|t| {
                        div()
                            .flex_none()
                            .px_2()
                            .py(px(2.))
                            .rounded_full()
                            .bg(with_alpha(PRIMARY, 0x26))
                            .text_size(px(10.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(PRIMARY))
                            .child(t)
                    }))),
            )
            .child({
                // The star: yellow on a favorite, else shown on hover.
                let star = div()
                    .id(("star", index))
                    .absolute()
                    .top(px(14.))
                    .right(px(14.))
                    .size(px(28.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .bg(with_alpha(BACKGROUND, 0xcc))
                    .hover(|s| s.bg(rgb(BACKGROUND)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.toggle_favorite(index, cx);
                    }));
                if favorite {
                    star.text_color(rgb(STAR)).child("★")
                } else {
                    star.text_color(with_alpha(FOREGROUND, 0xcc))
                        .opacity(0.)
                        .group_hover(group, |s| s.opacity(1.))
                        .child("☆")
                }
            })
    }
}

impl Local {
    /// A wheel notch moves a fixed step at once in GPUI; here it sets where
    /// to go and a short loop glides there (`glide`), about 120 steps a
    /// second. A touchpad's pixel deltas are already smooth: they apply as
    /// they come. (Not `request_animation_frame`: from an input event there's
    /// no view being drawn, and GPUI panics.)
    fn wheel(&mut self, event: &ScrollWheelEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let handle = self.scroll.0.borrow().base_handle.clone();
        let max = f32::from(handle.max_offset().y);
        let now = f32::from(handle.offset().y);
        match event.delta {
            ScrollDelta::Lines(lines) => {
                let from = self.glide_to.unwrap_or(now);
                self.glide_to = Some((from + lines.y * 140.0).clamp(-max, 0.0));
                if self.gliding.is_none() {
                    self.gliding = Some(cx.spawn(async move |this, cx| {
                        loop {
                            cx.background_executor().timer(std::time::Duration::from_millis(8)).await;
                            let more = this.update(cx, |this, cx| {
                                let more = this.glide();
                                cx.notify();
                                if !more {
                                    this.gliding = None;
                                }
                                more
                            });
                            if !matches!(more, Ok(true)) {
                                break;
                            }
                        }
                    }));
                }
            }
            ScrollDelta::Pixels(delta) => {
                self.glide_to = None;
                handle.set_offset(point(px(0.), px((now + f32::from(delta.y)).clamp(-max, 0.0))));
            }
        }
        cx.notify();
    }

    /// One step of a glide: a fifth of the way left, so it eases out.
    /// Whether there's more to go.
    fn glide(&mut self) -> bool {
        let Some(to) = self.glide_to else { return false };
        let handle = self.scroll.0.borrow().base_handle.clone();
        let now = f32::from(handle.offset().y);
        let next = if (to - now).abs() < 0.5 { to } else { now + (to - now) * 0.2 };
        handle.set_offset(point(px(0.), px(next)));
        if next == to {
            self.glide_to = None;
        }
        self.glide_to.is_some()
    }
}

/// The store again, for the thumbnail task (it only has `this`).
fn scan_store_clone(this: &WeakEntity<Local>, cx: &mut AsyncApp) -> Store {
    this.read_with(cx, |this, _| this.store.clone()).unwrap_or_else(|_| Store::new(Dirs::from_env()))
}

impl Render for Local {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let width = f32::from(window.viewport_size().width) - 2.0 * 24.0;
        self.columns = (((width + GAP) / (TILE_MIN + GAP)).floor() as usize).max(1);
        let columns = self.columns;
        let tile_w = (width - GAP * (columns as f32 - 1.0)) / columns as f32;
        let rows = self.shown.len().div_ceil(columns);
        let favorites_only = self.favorites_only;
        let count = format!("{} images", self.shown.len());

        let favorites = Button::new("favorites").label("★ Favorites").rounded(px(14.));
        let favorites = if favorites_only { favorites.primary() } else { favorites.outline() }
            .on_click(cx.listener(|this, _, _, cx| {
                this.favorites_only = !this.favorites_only;
                this.filter();
                cx.notify();
            }));

        v_flex()
            .size_full()
            .bg(rgb(BACKGROUND))
            .text_color(rgb(FOREGROUND))
            .font_family("Nunito")
            .child(
                // The floating search card.
                h_flex()
                    .m_4()
                    .px_4()
                    .py_3()
                    .gap_3()
                    .rounded(px(22.))
                    .bg(rgb(CARD))
                    .border_2()
                    .border_color(with_alpha(FOREGROUND, 0x17))
                    .shadow_lg()
                    .child(div().text_lg().font_weight(FontWeight::EXTRA_BOLD).text_color(rgb(PRIMARY)).child("wallpicker"))
                    .child(div().w(px(320.)).child(Input::new(&self.search).rounded(px(14.))))
                    .child(favorites)
                    .child(div().text_sm().text_color(rgb(MUTED_FOREGROUND)).child(count))
                    .child(div().flex_1())
                    .child(div().text_sm().text_color(rgb(MUTED_FOREGROUND)).child(self.status.clone())),
            )
            .child(
                image_cache(self.image_cache.clone())
                    .relative()
                    .flex_1()
                    .min_h_0()
                    // Catches the wheel before the list does (capture phase).
                    .child({
                        let view = cx.entity();
                        canvas(
                            |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
                            move |_, hitbox, window, _| {
                                window.on_mouse_event(move |event: &ScrollWheelEvent, phase, window, cx| {
                                    if phase == DispatchPhase::Capture && hitbox.is_hovered(window) {
                                        cx.stop_propagation();
                                        view.update(cx, |this, cx| this.wheel(event, window, cx));
                                    }
                                });
                            },
                        )
                        .absolute()
                        .size_full()
                    })
                    .child(
                    uniform_list(
                        "grid",
                        rows,
                        cx.processor(move |this, range: std::ops::Range<usize>, _window, cx| {
                            range
                                .map(|row| {
                                    let tiles: Vec<_> = (row * columns..((row + 1) * columns).min(this.shown.len()))
                                        .map(|i| this.shown[i])
                                        .map(|index| this.tile(index, tile_w, cx).into_any_element())
                                        .collect();
                                    h_flex().px_6().pb(px(GAP)).gap(px(GAP)).children(tiles)
                                })
                                .collect()
                        }),
                    )
                    .track_scroll(&self.scroll)
                    .size_full(),
                ),
            )
    }
}

/// Keeps the `max` most recently drawn images decoded; older ones are freed
/// (from the GPU too). GPUI's default keeps every image ever drawn.
struct LruImages {
    max: usize,
    order: VecDeque<u64>,
    items: HashMap<u64, ImageCacheItem>,
}

impl LruImages {
    fn new(max: usize) -> Self {
        LruImages { max, order: VecDeque::new(), items: HashMap::new() }
    }
}

impl ImageCache for LruImages {
    fn load(&mut self, resource: &Resource, window: &mut Window, cx: &mut App) -> Option<Result<Arc<RenderImage>, ImageCacheError>> {
        let key = hash(resource);
        if self.items.contains_key(&key) {
            self.order.retain(|k| *k != key);
        } else {
            while self.order.len() >= self.max {
                let Some(old) = self.order.pop_back() else { break };
                if let Some(item) = self.items.remove(&old)
                    && let Some(Ok(image)) = item.get()
                {
                    cx.drop_image(image, Some(window));
                }
            }
            self.items.insert(key, ImageCacheItem::new(resource, cx));
        }
        self.order.push_front(key);
        self.items.get(&key).and_then(|item| item.use_image(window))
    }
}

fn main() {
    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        // The app's Nunito is one variable font; GPUI only uses its default
        // (light) weight, so fixed weights cut from it (fonts/).
        let _ = cx.text_system().add_fonts(vec![
            std::borrow::Cow::Borrowed(include_bytes!("../fonts/Nunito-400.ttf")),
            std::borrow::Cow::Borrowed(include_bytes!("../fonts/Nunito-600.ttf")),
            std::borrow::Cow::Borrowed(include_bytes!("../fonts/Nunito-700.ttf")),
            std::borrow::Cow::Borrowed(include_bytes!("../fonts/Nunito-800.ttf")),
        ]);
        Theme::change(ThemeMode::Dark, None, cx);
        {
            let theme = Theme::global_mut(cx);
            theme.font_family = "Nunito".into();
            theme.colors.primary = rgb(PRIMARY).into();
            theme.colors.primary_foreground = rgb(BACKGROUND).into();
            theme.colors.background = rgb(BACKGROUND).into();
            theme.colors.border = with_alpha(FOREGROUND, 0x17).into();
            theme.colors.input = with_alpha(FOREGROUND, 0x17).into();
            theme.colors.ring = rgb(PRIMARY).into();
        }
        let bounds = Bounds::centered(None, size(px(1280.), px(860.)), cx);
        gpui_kit::open_window(
            WindowOptions {
                titlebar: Some(TitlebarOptions { title: Some("wallpicker · GPUI prototype".into()), ..Default::default() }),
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                app_id: Some("wallpicker-gpui".into()),
                ..Default::default()
            },
            cx,
            |window, cx| cx.new(|cx| Local::new(window, cx)),
        )
        .expect("could not open the window");
    });
}
