// The local library: every image in the wallpaper folders (with its cached
// thumbnail, when there is one), favorites and tags, and the tags' counts.
pragma Singleton
import QtQuick
import Quickshell

Singleton {
    id: library

    property var images: []
    /// path -> {favorite, tags}
    property var entries: ({})
    property bool loading: true

    /// [{name, count}], by name (counted over the images on disk).
    readonly property var tagCounts: {
        const counts = {};
        const names = {};
        for (const img of images)
            for (const t of tags(img.path)) {
                const key = t.toLowerCase();
                counts[key] = (counts[key] || 0) + 1;
                names[key] = names[key] || t;
            }
        return Object.keys(counts).map(k => ({
                    name: names[k],
                    count: counts[k]
                })).sort((a, b) => a.name.localeCompare(b.name, undefined, {
                    sensitivity: "base"
                }));
    }

    function favorite(path) {
        const e = entries[path];
        return !!(e && e.favorite);
    }
    function tags(path) {
        const e = entries[path];
        return e && e.tags ? e.tags : [];
    }

    function reload() {
        Backend.request("library.scan", {}, list => {
            library.images = list || [];
            library.loading = false;
        });
        Backend.request("library.catalog", {}, c => library.entries = c && c.items ? c.items : {});
    }

    /// Shown at once; the saved catalog replaces it.
    function setFavorite(path, on) {
        const next = Object.assign({}, entries);
        next[path] = Object.assign({
            tags: []
        }, next[path], {
            favorite: on
        });
        entries = next;
        Backend.request("library.set_favorite", {
            path: path,
            favorite: on
        }, c => library.entries = c.items || {}, () => library.reload());
    }

    /// `path`'s thumbnail, made if needed; `done(file)`.
    function thumbnail(path, done) {
        Backend.request("library.thumbnail", {
            path: path
        }, done);
    }

    Component.onCompleted: reload()
}
