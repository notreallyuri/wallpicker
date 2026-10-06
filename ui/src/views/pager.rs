//! The pager under Local's and Online's results: previous, numbered pages,
//! next, and a box to jump to any page.

use leptos::prelude::*;
use wallpicker_core::wallhaven::page_window;

use crate::components::nc::prelude::*;

/// Previous / numbered pages / next, then "Jump to" (a page number and
/// Enter; kept to the pages there are). Links get `href="#"` so they stay
/// focusable; the click is handled here instead of navigating.
#[component]
pub(crate) fn Pager(
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
    let jump = RwSignal::new(String::new());
    // Digits only (a paste or a drop can bring anything else). Not
    // type=number: its spinner arrows don't fit the look.
    Effect::new(move |_| {
        let typed = jump.get();
        let digits: String = typed.chars().filter(char::is_ascii_digit).take(6).collect();
        if digits != typed {
            jump.set(digits);
        }
    });
    let jump_to = move || {
        let Ok(n) = jump.get_untracked().trim().parse::<u64>() else { return };
        let page = n.clamp(1, last());
        jump.set(String::new());
        if !loading.get_untracked() && page != current() {
            go_to.run(page);
        }
    };

    view! {
        <Pagination class="flex-wrap items-center gap-x-3 gap-y-2">
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
            // Many pages online: the numbered ones only reach a few.
            <label class="flex items-center gap-2 text-xs text-muted-foreground">
                "Jump to"
                <Input
                    model=jump
                    class="h-8 w-16 text-center"
                    attr:inputmode="numeric"
                    attr:placeholder="page"
                    attr:aria-label=move || format!("Page, 1 to {}", last())
                    on:keydown=move |e: leptos::ev::KeyboardEvent| {
                        let key = e.key();
                        if key == "Enter" {
                            e.prevent_default();
                            jump_to();
                        } else if key.chars().count() == 1
                            && !key.chars().all(|c| c.is_ascii_digit())
                            && !e.ctrl_key()
                            && !e.meta_key()
                            && !e.alt_key()
                        {
                            // A letter or a sign never shows up, not even
                            // for a moment (shortcuts like Ctrl+V still work).
                            e.prevent_default();
                        }
                    }
                />
                {move || format!("of {}", last())}
            </label>
        </Pagination>
    }
}
