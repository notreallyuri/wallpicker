// The local library: every image in the wallpaper folders (with its cached
// thumbnail when there is one, and its place: `root` and `folder`), the
// wallpaper folders, favorites, tags and metadata (artists, uploader,
// source, origin), and the tags' and artists' counts. Rescans when
// the backend says the library changed (a trash, duplicates resolved).
pragma Singleton
import QtQuick
import Quickshell

Singleton {
    id: library

    property var images: []
    /// The wallpaper folders: [{path, name}].
    property var roots: []
    /// path -> {favorite, tags, origin?, source?, uploader?, artists?}
    /// (a person: {name, query?}).
    property var entries: ({})
    property bool loading: true

    /// [{name, count}], by name (counted over the images on disk).
    readonly property var tagCounts: counted(tags)
    readonly property var artistCounts: counted(artists)

    /// What `namesOf(path)` gives, counted over the images (ignoring case,
    /// the first spelling shown), by name.
    function counted(namesOf) {
        const counts = {};
        const names = {};
        for (const img of images)
            for (const t of namesOf(img.path)) {
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
    /// Its artists' names.
    function artists(path) {
        const e = entries[path];
        return e && e.artists ? e.artists.map(a => a.name) : [];
    }
    /// {origin, source, uploader, artists}: what's known, each null or [] when not.
    function metadata(path) {
        const e = entries[path] || {};
        return {
            origin: e.origin || null,
            source: e.source || null,
            uploader: e.uploader || null,
            artists: e.artists || []
        };
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

    /// Make `path`'s artists `list` ([{name, query?}], in order).
    function setArtists(path, list) {
        Backend.request("library.set_artists", {
            path: path,
            artists: list
        }, c => library.entries = c.items || {}, e => Toasts.error("Could not change the artists", e));
    }

    /// To the trash (restorable from the file manager); the backend says the
    /// library changed and it's scanned again.
    function trash(path, done) {
        Backend.request("wallpapers.trash", {
            path: path
        }, () => {
            Toasts.info("Moved " + path.split("/").pop() + " to the trash");
            if (done)
                done(true);
        }, e => {
            Toasts.error("Could not move it to the trash", e);
            if (done)
                done(false);
        });
    }

    function showInFolder(path) {
        Backend.request("wallpapers.show_in_folder", {
            path: path
        }, null, e => Toasts.error("Could not show it in its folder", e));
    }

    /// The duplicate groups (best first in each); `done(groups)`, or
    /// `failed(message)`.
    function duplicates(done, failed) {
        Backend.request("library.duplicates", {}, done, failed);
    }

    /// Keep `keep`, trash `others`; `done()` either way.
    function resolveDuplicates(keep, others, done) {
        Backend.request("library.resolve_duplicates", {
            keep: keep,
            others: others
        }, r => {
            if (r.failed.length === 0)
                Toasts.info("Moved " + r.trashed.length + " to the trash");
            else
                Toasts.error("Some copies could not be moved to the trash", r.failed.map(f => f[0] + ": " + f[1]).join("\n"));
            done();
        }, e => {
            Toasts.error("Could not clear the duplicates", e);
            done();
        });
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
            if (name === "library-changed" || name === "settings-changed")
                library.reload();
            else if (name === "catalog-changed")
                Backend.request("library.catalog", {}, c => library.entries = c && c.items ? c.items : {});
        }
    }
}
