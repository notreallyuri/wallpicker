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

/// Blossom, dark (the app's default theme).
const BACKGROUND: u32 = 0x1d1622;
const CARD: u32 = 0x2a2030;
const BORDER: u32 = 0x3a2d42;
const PRIMARY: u32 = 0xf0a6c8;
const MUTED: u32 = 0xa89bb0;
const STAR: u32 = 0xfbbf24;

/// The narrowest a tile gets before a column goes.
const TILE_MIN: f32 = 260.0;
const GAP: f32 = 16.0;
/// Decoded thumbnails kept; the rest are freed (the WebKit app keeps all).
const IMAGES_KEPT: usize = 150;

struct Local {
    store: Store,
    images: Vec<LocalImage>,
    /// Each image's thumbnail, once made (same cache as the app).
    thumbs: HashMap<PathBuf, PathBuf>,
    catalog: Catalog,
    /// Indexes into `images` that pass the search, favorites first.
    shown: Vec<usize>,
    query: String,
    favorites_only: bool,
    search: Entity<InputState>,
    image_cache: Entity<LruImages>,
    status: SharedString,
    columns: usize,
    scroll: UniformListScrollHandle,
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
            let (images, catalog) = cx
                .background_spawn(async move {
                    let settings = scan_store.load_settings().unwrap_or_default();
                    (library::scan(&settings.folders, settings.recursive), scan_store.load_catalog().unwrap_or_default())
                })
                .await;
            let paths: Vec<PathBuf> = images.iter().map(|i| i.path.clone()).collect();
            let _ = this.update(cx, |this, cx| {
                this.images = images;
                this.catalog = catalog;
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
        if std::env::var_os("WALLPICKER_AUTOSCROLL").is_some() {
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
            shown: Vec::new(),
            query: String::new(),
            favorites_only: false,
            search,
            image_cache: cx.new(|_| LruImages::new(IMAGES_KEPT)),
            status: "".into(),
            columns: 4,
            scroll: UniformListScrollHandle::new(),
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
                    let Some(m) = focused else { return Ok("no monitor".to_string()) };
                    hyprpaper::apply_and_remember_with(&SystemRunner, &store, backend, &[m.clone()], &path)?;
                    Ok::<_, wallpicker_core::Error>(format!("Applied to {m}"))
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.status = match result {
                    Ok(s) => s.into(),
                    Err(e) => format!("Could not apply: {e}").into(),
                };
                cx.notify();
            });
        })
        .detach();
    }

    fn tile(&self, index: usize, tile_w: f32, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let img_info = &self.images[index];
        let favorite = self.catalog.is_favorite(&img_info.path);
        let tag = self.catalog.tags(&img_info.path).first().cloned();
        let thumb = self.thumbs.get(&img_info.path).cloned();
        let name: SharedString = img_info.name.clone().into();
        let picture_h = tile_w * 9.0 / 16.0;
        div()
            .id(("tile", index))
            .w(px(tile_w))
            .flex_none()
            .rounded(px(18.))
            .bg(rgb(CARD))
            .border_2()
            .border_color(rgb(BORDER))
            .overflow_hidden()
            .cursor_pointer()
            .shadow_md()
            .hover(|s| s.border_color(rgb(PRIMARY)).shadow_lg())
            .on_click(cx.listener(move |this, _, _, cx| this.apply(index, cx)))
            .child(
                div()
                    .relative()
                    .h(px(picture_h))
                    .w_full()
                    .bg(rgb(BORDER))
                    .children(thumb.map(|t| img(t).size_full().object_fit(ObjectFit::Cover)))
                    .children(favorite.then(|| {
                        div()
                            .absolute()
                            .top_2()
                            .right_2()
                            .px_2()
                            .rounded_full()
                            .bg(rgba(0x00000088))
                            .text_color(rgb(STAR))
                            .child("★")
                    })),
            )
            .child(
                h_flex()
                    .px_3()
                    .py_2()
                    .gap_2()
                    .justify_between()
                    .child(div().text_sm().font_weight(FontWeight::BOLD).truncate().child(name))
                    .children(tag.map(|t| {
                        div()
                            .flex_none()
                            .px_2()
                            .rounded_full()
                            .bg(rgba(0xf0a6c833))
                            .text_xs()
                            .text_color(rgb(PRIMARY))
                            .child(t)
                    })),
            )
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
            .text_color(rgb(0xf4eef6))
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
                    .border_color(rgb(BORDER))
                    .shadow_lg()
                    .child(div().text_lg().font_weight(FontWeight::EXTRA_BOLD).text_color(rgb(PRIMARY)).child("wallpicker"))
                    .child(div().w(px(320.)).child(Input::new(&self.search).rounded(px(14.))))
                    .child(favorites)
                    .child(div().text_sm().text_color(rgb(MUTED)).child(count))
                    .child(div().flex_1())
                    .child(div().text_sm().text_color(rgb(MUTED)).child(self.status.clone())),
            )
            .child(
                image_cache(self.image_cache.clone()).flex_1().min_h_0().child(
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
        let _ = cx
            .text_system()
            .add_fonts(vec![std::borrow::Cow::Borrowed(include_bytes!("../../../ui/assets/fonts/Nunito.ttf"))]);
        Theme::change(ThemeMode::Dark, None, cx);
        {
            let theme = Theme::global_mut(cx);
            theme.font_family = "Nunito".into();
            theme.colors.primary = rgb(PRIMARY).into();
            theme.colors.primary_foreground = rgb(BACKGROUND).into();
            theme.colors.background = rgb(BACKGROUND).into();
            theme.colors.border = rgb(BORDER).into();
            theme.colors.input = rgb(BORDER).into();
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
