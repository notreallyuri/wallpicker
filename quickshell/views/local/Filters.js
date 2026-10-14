// Local's tag and artist filters, as tags.rs orders them: the chosen ones
// first, then by name or the most used first (Settings → General).
.pragma library

/// `counts` ([{name, count}]) in order: `chosen` (names) first, then by
/// name ("name") or the most used first ("count"). Every comparison
/// decides (the name last): the engine's sort isn't stable, so ties would
/// come out in any order.
function ordered(counts, chosen, order) {
    return counts.slice().sort((a, b) => (chosen.includes(b.name) - chosen.includes(a.name)) || (order === "count" ? b.count - a.count : 0) || a.name.localeCompare(b.name, undefined, {
            sensitivity: "base"
        }) || (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));
}
