// The page numbers a pager shows (wallhaven::page_window): 1, current ±1,
// last; a gap of one page filled in, longer gaps 0 (shown as "…").
.pragma library

function pages(current, last) {
    const wanted = [...new Set([1, current - 1, current, current + 1, last].filter(p => p >= 1 && p <= last))].sort((a, b) => a - b);
    const out = [];
    let prev = 0;
    for (const p of wanted) {
        if (prev && p - prev === 2)
            out.push(prev + 1);
        else if (prev && p - prev > 2)
            out.push(0);
        out.push(p);
        prev = p;
    }
    return out;
}
