//! Settings → Workspaces: a wallpaper per Hyprland workspace. The switches
//! are saved with the form; the list (each workspace and its wallpaper) is
//! the app's own and changes at once: as you apply wallpapers (with
//! "remember" on), and from its buttons here.

use leptos::prelude::*;
use leptos::task::spawn_local;
use nocomponents::utils::types::Orientation;

use crate::api::{self, WorkspaceRow};
use crate::components::nc::prelude::*;
use crate::ctx::Ctx;

#[component]
pub(crate) fn WorkspacesPane(enabled: RwSignal<bool>, remember: RwSignal<bool>) -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let rows = RwSignal::new(None::<Vec<WorkspaceRow>>);
    let load = move || {
        spawn_local(async move {
            match api::workspace_rows().await {
                Ok(r) => {
                    rows.try_set(Some(r));
                }
                Err(e) => ctx.error_with("Could not list the workspaces", &e),
            }
        });
    };
    load();
    // Workspaces open and close, and switches change what's shown: kept up
    // to date while this section is open.
    let alive = RwSignal::new(true);
    on_cleanup(move || alive.set(false));
    let _ = set_interval_with_handle(
        move || {
            if alive.get_untracked() {
                load();
            }
        },
        std::time::Duration::from_secs(3),
    )
    .map(|h| on_cleanup(move || h.clear()));
    let assign = move |ws: String, path: Option<String>| {
        spawn_local(async move {
            match api::set_workspace_wallpaper(&ws, path.as_deref()).await {
                Ok(()) => load(),
                Err(e) => ctx.error_with("Could not change the workspace's wallpaper", &e),
            }
        });
    };

    view! {
        <Field orientation=Orientation::Horizontal>
            <Switch checked=enabled />
            <FieldLabel>"A wallpaper per workspace"</FieldLabel>
        </Field>
        <p class="-mt-2 text-sm text-muted-foreground">
            "When a monitor switches to a workspace that has a wallpaper, it goes on that monitor. "
            "Workspaces without one leave the monitor as it is. Switching doesn't count as a pick: "
            "Previous still goes back to the one you chose before. Save to apply."
        </p>
        <Field orientation=Orientation::Horizontal>
            <Switch checked=remember />
            <FieldLabel>"Remember what I put on each workspace"</FieldLabel>
        </Field>
        <p class="-mt-2 text-sm text-muted-foreground">
            "A wallpaper you apply (or go back to) becomes the wallpaper of the workspace its monitor is "
            "showing. Random and the slideshow don't change them."
        </p>
        <div class="flex flex-col gap-2">
            <span class="text-sm font-bold">"Workspaces"</span>
            {move || match rows.get() {
                None => view! { <Spinner class="size-4" /> }.into_any(),
                Some(r) if r.is_empty() => {
                    view! { <p class="text-sm text-muted-foreground">"No workspaces (is Hyprland running?)."</p> }.into_any()
                }
                Some(r) => {
                    r.into_iter()
                        .map(|row| view! { <WorkspaceItem row=row assign=assign /> })
                        .collect_view()
                        .into_any()
                }
            }}
            <p class="text-xs text-muted-foreground">
                "Also from a terminal: " <code>"wallpicker workspaces"</code>
                ". wallpicker follows the workspaces while it runs (the tray is enough); without it, run "
                <code>"wallpicker workspaces run"</code> "."
            </p>
        </div>
    }
}

/// One workspace: its wallpaper's thumbnail, where it is, and buttons.
#[component]
fn WorkspaceItem(row: WorkspaceRow, assign: impl Fn(String, Option<String>) + Copy + Send + Sync + 'static) -> impl IntoView {
    let ctx = expect_context::<Ctx>();
    let thumb = RwSignal::new(None::<String>);
    if let Some(p) = row.wallpaper.clone() {
        spawn_local(async move {
            if let Ok(t) = api::thumbnail(&p).await {
                thumb.try_set(Some(api::asset_url(&t)));
            }
        });
    }
    let name = StoredValue::new(row.name.clone());
    // What its monitor shows now: "Use it" when that's not its wallpaper.
    let shown = {
        let monitor = row.monitor.clone();
        let wallpaper = row.wallpaper.clone();
        Memo::new(move |_| {
            let m = monitor.as_ref()?;
            let p = ctx.state.with(|s| s.current.get(m).map(|p| p.to_string_lossy().into_owned()))?;
            (Some(&p) != wallpaper.as_ref()).then_some((m.clone(), p))
        })
    };
    let file = row.wallpaper.as_deref().and_then(|p| p.rsplit('/').next()).map(str::to_string);
    let where_ = match (&row.monitor, row.open) {
        (Some(m), _) => format!("on {m}"),
        (None, true) => "open".to_string(),
        (None, false) => "not open now".to_string(),
    };
    view! {
        <div class="flex items-center gap-3 rounded-2xl border-2 border-border/60 p-2">
            <div class="flex size-9 shrink-0 items-center justify-center rounded-xl bg-primary/15 font-black text-primary">
                {row.name.chars().take(3).collect::<String>()}
            </div>
            <div class="h-12 w-20 shrink-0 overflow-hidden rounded-lg bg-muted">
                {move || thumb.get().map(|t| view! { <img src=t alt="" class="size-full object-cover" /> })}
            </div>
            <div class="flex min-w-0 flex-1 flex-col">
                <span class="truncate text-sm font-bold">
                    {file.unwrap_or_else(|| "No wallpaper".to_string())}
                </span>
                <span class="truncate text-xs text-muted-foreground">{format!("Workspace {} · {where_}", row.name)}</span>
            </div>
            {move || shown.get().map(|(m, p)| view! {
                <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| assign(name.get_value(), Some(p.clone()))>
                    {format!("Use {m}'s")}
                </Button>
            })}
            {row.wallpaper.is_some().then(|| view! {
                <Button size=ButtonSize::Sm variant=ButtonVariant::Ghost on_click=move |_| assign(name.get_value(), None)>
                    "Clear"
                </Button>
            })}
        </div>
    }
}
