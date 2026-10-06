// The local library: every image in the wallpaper folders (with its cached
// thumbnail when there is one, and its place: `root` and `folder`), the
// wallpaper folders, favorites and tags, and the tags' counts. Rescans when
// the backend says the library changed (a trash, duplicates resolved).
pragma Singleton
import QtQuick
import Quickshell

Singleton {
    id: library

    property var images: []
    /// The wallpaper folders: [{path, name}].
    property var roots: []
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
        Backend.request("library.scan", {}, lib => {
            library.images = lib.images || [];
            library.roots = lib.roots || [];
            library.loading = false;
        }, e => {
            library.loading = false;
            Toasts.error("Scanning the folders failed", e);
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

    function addTag(path, tag, done) {
        Backend.request("library.add_tag", {
            path: path,
            tag: tag
        }, c => {
            library.entries = c.items || {};
            if (done)
                done();
        }, e => Toasts.error("Could not add the tag", e));
    }

    function removeTag(path, tag) {
        Backend.request("library.remove_tag", {
            path: path,
            tag: tag
        }, c => library.entries = c.items || {}, e => Toasts.error("Could not remove the tag", e));
    }

    /// Paths with `color`, the most of it first; `done(paths)`.
    function withColor(color, done) {
        Backend.request("library.with_color", {
            color: color
        }, found => done(found.map(f => f[0])), e => {
            Toasts.error("Could not read the wallpapers' colors", e);
            done([]);
        });
    }

    Component.onCompleted: reload()

    Connections {
        target: Backend
        function onEvent(name) {
            if (name === "library-changed")
                library.reload();
        }
    }
}
