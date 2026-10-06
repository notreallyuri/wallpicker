// What the Local tab shows: the library filtered (search by name or tag,
// favorites, tags: all of them), favorites first then the chosen order, in
// pages of 60. A new filter starts from the first page.
import QtQuick
import qs.services

QtObject {
    id: model

    readonly property int pageSize: 60
    readonly property var sorts: [["newest", "Newest"], ["oldest", "Oldest"], ["name", "Name"], ["random", "Random order"]]

    property string query: ""
    property bool favoritesOnly: false
    property string sort: "newest"
    property var tagFilter: []
    property int page: 1

    readonly property var shown: {
        const q = query.trim().toLowerCase();
        const want = tagFilter.map(t => t.toLowerCase());
        const list = Library.images.filter(i => {
            const tags = Library.tags(i.path).map(t => t.toLowerCase());
            return (!favoritesOnly || Library.favorite(i.path)) && want.every(t => tags.includes(t)) && (q === "" || i.name.toLowerCase().includes(q) || tags.some(t => t.includes(q)));
        });
        const order = {
            newest: (a, b) => b.modified - a.modified,
            oldest: (a, b) => a.modified - b.modified,
            name: (a, b) => a.name.localeCompare(b.name),
            random: (a, b) => hash(a.path) - hash(b.path)
        }[sort];
        return list.sort((a, b) => (Library.favorite(b.path) - Library.favorite(a.path)) || order(a, b));
    }
    readonly property int lastPage: Math.max(1, Math.ceil(shown.length / pageSize))
    readonly property int currentPage: Math.min(Math.max(1, page), lastPage)
    readonly property var pageItems: shown.slice((currentPage - 1) * pageSize, currentPage * pageSize)

    onQueryChanged: page = 1
    onFavoritesOnlyChanged: page = 1
    onSortChanged: page = 1
    onTagFilterChanged: page = 1

    function toggleTag(name) {
        const t = tagFilter.slice();
        const i = t.indexOf(name);
        if (i >= 0)
            t.splice(i, 1);
        else
            t.push(name);
        tagFilter = t;
    }

    /// One of what's shown, at random, on the chosen monitors.
    function applyRandom() {
        if (shown.length > 0)
            Monitors.apply(shown[Math.floor(Math.random() * shown.length)].path);
    }

    // A stable shuffle for "Random order".
    function hash(s) {
        let h = 0;
        for (let i = 0; i < s.length; i++)
            h = (h * 31 + s.charCodeAt(i)) | 0;
        return h;
    }
}
