// What the Local tab shows (ui/src/views/local/mod.rs): the library
// filtered (search by name, tag, artist or uploader, favorites, tags with
// All or Any, any of some artists, a color), favorites first then the chosen order (with a color: the most of
// it first), in pages of 60. In the Folders layout with no filter on, the
// current folder's subfolders lead the first page, then its own images. A
// new filter, sort, layout or folder starts from the first page.
import QtQuick
import qs.services
import "Folders.js" as Folders

QtObject {
    id: model

    readonly property int pageSize: 60
    readonly property var sorts: [["newest", "Newest"], ["oldest", "Oldest"], ["name", "Name"], ["random", "Random order"]]

    property string query: ""
    property bool favoritesOnly: false
    property var tagFilter: []
    property var artistFilter: []
    // Remembered between opens.
    property string sort: Prefs.localSort
    property string layout: Prefs.localLayout
    property bool tagsMatchAll: Prefs.tagsMatchAll
    /// A hue or tone ("" none), and the paths with it, the most first.
    property string color: ""
    property var colorOrder: []
    property bool colorLoading: false
    /// Where the Folders layout is (see Folders.js).
    property var place: Folders.top(Library.roots.length)
    property int page: 1
    /// The tile under the pointer (Delete asks to trash it).
    property string hovered: ""
    /// Random order: a number per path, drawn again each time it's picked.
    property var shuffle: ({})
    /// What's being looked at: the same key with a new list (a favorite
    /// moved, a tag changed) keeps the grid's scroll position.
    readonly property string viewKey: [currentPage, query, favoritesOnly, JSON.stringify(tagFilter), JSON.stringify(artistFilter), sort, layout, tagsMatchAll, color, JSON.stringify(place)].join("|")

    readonly property bool folders: layout === "folders"
    readonly property bool filtering: favoritesOnly || query.trim() !== "" || tagFilter.length > 0 || artistFilter.length > 0 || color !== ""
    readonly property var rootNames: Library.roots.map(r => r.name)

    // The library in order: favorites first, then the sort.
    readonly property var sorted: {
        const order = {
            newest: (a, b) => b.modified - a.modified,
            oldest: (a, b) => a.modified - b.modified,
            name: (a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()),
            random: (a, b) => (shuffle[a.path] || 0) - (shuffle[b.path] || 0)
        }[sort] || ((a, b) => b.modified - a.modified);
        return Library.images.slice().sort((a, b) => (Library.favorite(b.path) - Library.favorite(a.path)) || order(a, b) || (a.path < b.path ? -1 : 1));
    }

    // Browsing (Folders, no filter): this folder's subfolders and images.
    readonly property var browsed: folders && !filtering ? Folders.browse(sorted, place, rootNames) : null

    readonly property var shown: {
        if (browsed)
            return browsed.direct;
        const q = query.trim().toLowerCase();
        const want = tagFilter.map(t => t.toLowerCase());
        const by = artistFilter.map(a => a.toLowerCase());
        const ranked = {};
        colorOrder.forEach((p, i) => ranked[p] = i);
        const list = sorted.filter(i => {
            if (folders && !Folders.inside(i, place))
                return false;
            if (favoritesOnly && !Library.favorite(i.path))
                return false;
            const tags = Library.tags(i.path).map(t => t.toLowerCase());
            if (want.length > 0 && !(tagsMatchAll ? want.every(t => tags.includes(t)) : want.some(t => tags.includes(t))))
                return false;
            const meta = Library.metadata(i.path);
            const artists = meta.artists.map(a => a.name.toLowerCase());
            if (by.length > 0 && !by.some(a => artists.includes(a)))
                return false;
            if (q !== "" && !i.name.toLowerCase().includes(q) && !tags.some(t => t.includes(q)) && !artists.some(a => a.includes(q)) && !(meta.uploader && meta.uploader.name.toLowerCase().includes(q)))
                return false;
            // Until the colors are read, nothing matches one.
            return color === "" || ranked[i.path] !== undefined;
        });
        return color === "" ? list : list.sort((a, b) => ranked[a.path] - ranked[b.path]);
    }
    readonly property var subfolders: browsed ? browsed.folders : []

    readonly property int lastPage: Math.max(1, Math.ceil(shown.length / pageSize))
    readonly property int currentPage: Math.min(Math.max(1, page), lastPage)
    /// This page: folders (first page only) as {kind: "folder", ...}, then
    /// images as {kind: "image", img}.
    readonly property var pageItems: Library.loading ? skeletons : (currentPage === 1 ? subfolders.map(f => ({
                    kind: "folder",
                    folder: f
                })) : []).concat(shown.slice((currentPage - 1) * pageSize, currentPage * pageSize).map(i => ({
                kind: "image",
                img: i
            })))

    // While scanning: placeholder tiles.
    readonly property var skeletons: Array.from({
        length: 16
    }, () => ({
                kind: "skeleton"
            }))

    onQueryChanged: page = 1
    onFavoritesOnlyChanged: page = 1
    onTagFilterChanged: page = 1
    onArtistFilterChanged: page = 1
    onTagsMatchAllChanged: {
        page = 1;
        Prefs.tagsMatchAll = tagsMatchAll;
    }
    onPlaceChanged: page = 1
    onLayoutChanged: {
        page = 1;
        Prefs.localLayout = layout;
    }
    onSortChanged: {
        page = 1;
        Prefs.localSort = sort;
        if (sort === "random")
            reshuffle();
    }
    onColorChanged: {
        page = 1;
        colorOrder = [];
        if (color === "")
            return;
        colorLoading = true;
        const asked = color;
        Library.withColor(asked, paths => {
            if (model.color !== asked)
                return;
            model.colorOrder = paths;
            model.colorLoading = false;
        });
    }

    // A tag (or artist) no longer in use (its last one removed) would filter
    // everything out while no longer shown: dropped.
    readonly property var liveTags: Library.tagCounts.map(t => t.name)
    onLiveTagsChanged: {
        const kept = tagFilter.filter(t => liveTags.includes(t));
        if (kept.length !== tagFilter.length)
            tagFilter = kept;
    }
    readonly property var liveArtists: Library.artistCounts.map(a => a.name)
    onLiveArtistsChanged: {
        const kept = artistFilter.filter(a => liveArtists.includes(a));
        if (kept.length !== artistFilter.length)
            artistFilter = kept;
    }

    // The folders changed, or this place is gone: back to the top.
    readonly property int rootCount: Library.roots.length
    onRootCountChanged: place = Folders.top(rootCount)

    function reshuffle() {
        const s = {};
        for (const i of Library.images)
            s[i.path] = Math.random();
        shuffle = s;
    }

    /// Everything shown again: no search, Favorites, color, tags or artists.
    function clearFilters() {
        query = "";
        favoritesOnly = false;
        color = "";
        tagFilter = [];
        artistFilter = [];
    }

    function toggleTag(name) {
        tagFilter = toggled(tagFilter, name);
    }
    function toggleArtist(name) {
        artistFilter = toggled(artistFilter, name);
    }
    function toggled(list, name) {
        return list.includes(name) ? list.filter(n => n !== name) : list.concat([name]);
    }

    function openFolder(to) {
        place = to;
    }

    /// One level up, if there is one.
    function up() {
        const to = Folders.parent(place, Library.roots.length);
        if (to !== undefined)
            place = to;
    }
    readonly property bool canGoUp: Folders.parent(place, Library.roots.length) !== undefined

    /// A random one of those shown, not one already on the chosen monitors
    /// when there's another.
    function applyRandom() {
        const onNow = Monitors.targets.map(m => Monitors.current[m]);
        const fresh = shown.filter(i => !onNow.includes(i.path));
        const pool = fresh.length > 0 ? fresh : shown;
        if (pool.length > 0)
            Monitors.apply(pool[Math.floor(Math.random() * pool.length)].path);
    }
}
