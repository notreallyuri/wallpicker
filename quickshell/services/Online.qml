// The Online tab's search: the provider, the search text (shared, so a term
// can be tried on each provider), each provider's filters, the results with
// their page, and tag suggestions (Konachan and yande.re match exact tag
// names only). The provider and filters are saved by the backend as they
// change (online.json, which the command line and the plugin use too).
// What the form offers (sorts, sizes, the defaults) comes from core
// (`online.options`).
pragma Singleton
import QtQuick
import Quickshell

Singleton {
    id: online

    /// {providers, defaults, min_sizes, wallhaven: {sorts, ranges,
    /// resolutions, ratios}, moebooru: {sorts}}; null until loaded.
    property var options: null
    property string provider: "wallhaven"
    property string text: ""
    /// provider -> its filters (without search text or paging).
    property var filters: ({})
    /// The saved filters are back: until then nothing is saved or searched.
    property bool restored: false

    property var results: []
    /// {page, last_page, total, seed} of the shown results; null before.
    property var page: null
    property bool loading: false
    /// Konachan's and yande.re's real tags for what's typed.
    property var suggestions: []

    /// A new page of results is in (the view scrolls back to the top).
    signal landed

    readonly property string label: labelOf(provider)
    readonly property bool hasApiKey: !!Settings.data.wallhaven_api_key
    /// konachan.net only has safe posts: Konachan's ratings don't apply.
    readonly property bool ratingsApply: provider !== "konachan" || Settings.data.konachan_site === "com"
    /// Filters set that differ from the defaults (on the Filters button).
    readonly property int activeFilters: countActive(filters[provider], options ? options.defaults[provider] : null)

    // The query behind the shown results: pages re-run it, so editing the
    // form without searching doesn't change what page 2 means.
    property var lastQuery: null
    property string shownProvider: ""
    property string searchedWith: ""
    // Only the newest search's answer is shown.
    property int latest: 0
    // The tab was opened before the filters were back: search once they are.
    property bool wanted: false

    function labelOf(p) {
        const found = options ? options.providers.find(x => x.value === p) : null;
        return found ? found.label : p;
    }

    /// "provider:id", unique across providers.
    function key(w) {
        return w.provider + ":" + w.id;
    }

    function countActive(f, d) {
        if (!f || !d)
            return 0;
        const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
        const set = v => !!v && v.length > 0;
        const on = provider === "wallhaven" ? [f.sorting !== d.sorting, f.sorting === "toplist" && f.top_range !== d.top_range, set(f.atleast), set(f.resolutions), set(f.ratios), !same(f.categories, d.categories), !same(f.purity, d.purity)] : [f.sorting !== d.sorting, set(f.atleast), ratingsApply && !same(f.ratings, d.ratings)];
        return on.filter(x => x).length;
    }

    /// The tab is showing: search the first time, and after the provider
    /// changed while it wasn't.
    function activate() {
        if (!restored)
            wanted = true;
        else if (searchedWith !== provider)
            search();
    }

    function setProvider(p) {
        if (p === provider)
            return;
        provider = p;
        save();
        search();
    }

    /// One of the current provider's filters.
    function setFilter(name, value) {
        const next = Object.assign({}, filters);
        next[provider] = Object.assign({}, next[provider]);
        next[provider][name] = value;
        filters = next;
        save();
    }

    /// The current provider's filters back to the defaults.
    function resetFilters() {
        const next = Object.assign({}, filters);
        next[provider] = JSON.parse(JSON.stringify(options.defaults[provider]));
        filters = next;
        save();
    }

    function save() {
        if (restored)
            saveTimer.restart();
    }

    /// A new search from the form, at page 1.
    function search() {
        const q = Object.assign({}, filters[provider], {
            page: 1
        });
        if (provider === "wallhaven") {
            q.q = text;
            // The backend drops it without an API key anyway.
            q.purity = Object.assign({}, q.purity, {
                nsfw: q.purity.nsfw && hasApiKey
            });
        } else {
            q.tags = text;
        }
        fetch({
            provider: provider,
            query: q
        });
    }

    function goTo(n) {
        if (!lastQuery)
            return;
        const q = Object.assign({}, lastQuery.query, {
            page: n
        });
        // A random order keeps its seed across pages.
        if (lastQuery.provider === "wallhaven")
            q.seed = page ? page.seed : null;
        fetch({
            provider: lastQuery.provider,
            query: q
        });
    }

    /// Search for `q` (a tag, an uploader), newest first, on `on` (the
    /// current provider when not given).
    function searchFor(q, on) {
        if (on)
            provider = on;
        text = q;
        setFilter("sorting", provider === "wallhaven" ? "date_added" : "latest");
        search();
    }

    function pickSuggestion(s) {
        Backend.request("online.apply_suggestion", {
            text: text,
            suggestion: s
        }, t => {
            online.text = t;
            online.search();
        });
    }

    function fetch(q) {
        const id = ++latest;
        searchedWith = q.provider;
        // Another provider's results don't stay looking current meanwhile.
        if (shownProvider !== q.provider) {
            results = [];
            page = null;
        }
        loading = true;
        Backend.request("online.search", {
            query: q
        }, p => {
            if (id !== online.latest)
                return;
            // Keys must be unique; random pages can repeat an image.
            const seen = {};
            online.results = (p.items || []).filter(w => {
                const k = online.key(w);
                if (seen[k])
                    return false;
                seen[k] = true;
                return true;
            });
            online.page = {
                page: p.page,
                last_page: p.last_page,
                total: p.total,
                seed: p.seed
            };
            online.shownProvider = q.provider;
            online.lastQuery = q;
            online.loading = false;
            online.landed();
        }, e => {
            if (id !== online.latest)
                return;
            online.loading = false;
            Toasts.error(online.labelOf(q.provider) + " search failed", e);
        });
    }

    onTextChanged: suggestTimer.restart()
    onProviderChanged: {
        suggestions = [];
        suggestTimer.restart();
    }

    // A moment after typing stops.
    Timer {
        id: suggestTimer
        interval: 350
        property int seq: 0
        onTriggered: {
            const n = ++seq;
            if (online.provider === "wallhaven" || !online.text.trim()) {
                online.suggestions = [];
                return;
            }
            Backend.request("online.tag_suggestions", {
                provider: online.provider,
                text: online.text
            }, got => {
                if (n === suggestTimer.seq)
                    online.suggestions = got || [];
            }, () => {});
        }
    }

    // Changes are saved together, a moment after the last.
    Timer {
        id: saveTimer
        interval: 300
        onTriggered: Backend.request("online.save_filters", {
            filters: Object.assign({
                provider: online.provider
            }, online.filters)
        }, null, e => console.warn("wallpicker: could not save the search filters:", e))
    }

    Component.onCompleted: {
        Backend.request("online.options", {}, o => online.options = o);
        Backend.request("online.filters", {}, f => {
            online.filters = {
                wallhaven: f.wallhaven,
                konachan: f.konachan,
                yandere: f.yandere
            };
            online.provider = f.provider;
            online.restored = true;
            if (online.wanted)
                online.search();
        }, e => {
            Toasts.error("Could not load the saved search filters", e);
            online.restored = true;
        });
    }
}
