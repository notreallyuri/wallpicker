// Your Wallhaven collections (the Collections tab; private ones too, with
// the API key): the list, the one shown, and its page of wallpapers.
// Opening one needs your Wallhaven username too (Settings → Wallhaven): the
// API doesn't say whose key it is.
pragma Singleton
import QtQuick
import Quickshell

Singleton {
    id: collections

    /// [{id, label, count, public}]; null while loading.
    property var list: null
    /// Why there's no list ("" when there is one).
    property string listError: ""
    /// The id of the one shown (0 none).
    property int chosen: 0

    property var results: []
    /// {page, last_page, total} of the shown page; null before.
    property var page: null
    property bool loading: false
    /// Why the chosen one's page couldn't be had.
    property string error: ""

    /// A new page is in (the view scrolls back to the top).
    signal landed

    property bool loaded: false
    property int latest: 0

    /// The tab is showing: the list the first time.
    function activate() {
        if (!loaded)
            reload();
    }

    /// The list again (keeping the choice if it's still there).
    function reload() {
        loaded = true;
        list = null;
        listError = "";
        Backend.request("online.collections", {}, got => {
            collections.list = got || [];
            if (!collections.list.some(c => c.id === collections.chosen))
                collections.choose(collections.list.length > 0 ? collections.list[0].id : 0);
        }, e => {
            collections.list = [];
            collections.listError = e;
        });
    }

    // The key or the username may have changed: the list again, if it was
    // shown.
    Connections {
        target: Backend
        function onEvent(name) {
            if (name === "settings-changed" && collections.loaded)
                collections.reload();
        }
    }

    function choose(id) {
        chosen = id;
        results = [];
        page = null;
        error = "";
        if (id)
            goTo(1);
    }

    function goTo(n) {
        const id = ++latest;
        const which = chosen;
        loading = true;
        error = "";
        Backend.request("online.collection", {
            id: which,
            page: n
        }, p => {
            if (id !== collections.latest)
                return;
            const seen = {};
            collections.results = (p.items || []).filter(w => {
                const k = Downloads.key(w);
                if (seen[k])
                    return false;
                seen[k] = true;
                return true;
            });
            collections.page = {
                page: p.page,
                last_page: p.last_page,
                total: p.total
            };
            collections.loading = false;
            collections.landed();
        }, e => {
            if (id !== collections.latest)
                return;
            collections.results = [];
            collections.page = null;
            collections.error = e;
            collections.loading = false;
        });
    }
}
