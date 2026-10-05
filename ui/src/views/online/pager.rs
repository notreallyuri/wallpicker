//! The pager under the online results.

use leptos::prelude::*;
use wallpicker_core::wallhaven::page_window;

use crate::components::nc::prelude::*;

/// Previous / numbered pages / next. Links get `href="#"` so they stay
/// focusable; the click is handled here instead of navigating.
#[component]
pub(super) fn Pager(
    pages: impl Fn() -> Option<(u64, u64)> + Copy + Send + Sync + 'static,
    loading: RwSignal<bool>,
    go_to: Callback<u64>,
) -> impl IntoView {
    let current = move || pages().map_or(1, |(c, _)| c);
    let last = move || pages().map_or(1, |(_, l)| l);
    // While a page loads, or past either end, a link does nothing.
    let inert = move |off: bool| {
        if off || loading.get() {
            "pointer-events-none opacity-50"
        } else {
            ""
        }
    };
    let click = move |e: leptos::ev::MouseEvent, page: u64| {
        e.prevent_default();
        if !loading.get_untracked() && page != current() && (1..=last()).contains(&page) {
            go_to.run(page);
        }
    };

    view! {
        <Pagination>
            <PaginationContent>
                <PaginationItem>
                    <PaginationPrevious
                        href="#"
                        class=move || inert(current() <= 1)
                        on:click=move |e| click(e, current().saturating_sub(1))
                    />
                </PaginationItem>
                {move || {
                    page_window(current(), last())
                        .into_iter()
                        .map(|p| match p {
                            Some(n) => {
                                view! {
                                    <PaginationItem>
                                        <PaginationLink
                                            href="#"
                                            active=n == current()
                                            class=move || inert(false)
                                            on:click=move |e| click(e, n)
                                        >
                                            {n.to_string()}
                                        </PaginationLink>
                                    </PaginationItem>
                                }
                                    .into_any()
                            }
                            None => {
                                view! {
                                    <PaginationItem>
                                        <PaginationEllipsis />
                                    </PaginationItem>
                                }
                                    .into_any()
                            }
                        })
                        .collect_view()
                }}
                <PaginationItem>
                    <PaginationNext
                        href="#"
                        class=move || inert(current() >= last())
                        on:click=move |e| click(e, current() + 1)
                    />
                </PaginationItem>
            </PaginationContent>
        </Pagination>
    }
}
