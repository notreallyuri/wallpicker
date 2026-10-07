// Browsing the library by folder, as crates/core/src/library.rs does it
// (folder_in is done by the backend: each image comes with root and
// folder). A place is {root, folder}: one of the wallpaper folders (an
// index) and a folder inside it ("" its top, "a/b" further down); null is
// the list of wallpaper folders, a place only when there are several.
.pragma library

function same(a, b) {
    return a === b || (a !== null && b !== null && a.root === b.root && a.folder === b.folder);
}

/// Is `p` this place, or somewhere below it?
function contains(at, p) {
    return at.root === p.root && (at.folder === "" || p.folder === at.folder || p.folder.startsWith(at.folder + "/"));
}

/// Where browsing starts.
function top(roots) {
    return roots <= 1 ? {
        root: 0,
        folder: ""
    } : null;
}

/// Is an image at `p` shown when browsing `at` (at it or below)?
function inside(p, at) {
    return at === null || contains(at, p);
}

/// The subfolders to show at `at` ({name, place, count, cover: the first
/// image's path}) and the images right there, from `images` in their order.
function browse(images, at, rootNames) {
    const folders = [];
    const direct = [];
    for (const img of images) {
        const p = {
            root: img.root,
            folder: img.folder
        };
        let name, place;
        if (at === null) {
            name = rootNames[p.root] || "?";
            place = {
                root: p.root,
                folder: ""
            };
        } else if (!contains(at, p)) {
            continue;
        } else if (p.folder === at.folder) {
            direct.push(img);
            continue;
        } else {
            const rest = at.folder === "" ? p.folder : p.folder.slice(at.folder.length + 1);
            name = rest.split("/")[0];
            place = {
                root: p.root,
                folder: at.folder === "" ? name : at.folder + "/" + name
            };
        }
        const f = folders.find(f => same(f.place, place));
        if (f)
            f.count++;
        else
            folders.push({
                name: name,
                place: place,
                count: 1,
                cover: img
            });
    }
    if (at === null)
        folders.sort((a, b) => a.place.root - b.place.root);
    else
        folders.sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()));
    return {
        folders: folders,
        direct: direct
    };
}

/// One level up: a place, null (the list of wallpaper folders), or
/// undefined when there's nothing above.
function parent(at, roots) {
    if (at === null)
        return undefined;
    if (at.folder === "")
        return roots > 1 ? null : undefined;
    const i = at.folder.lastIndexOf("/");
    return {
        root: at.root,
        folder: i < 0 ? "" : at.folder.slice(0, i)
    };
}

/// The way to `at`, for a breadcrumb: [{label, place}] from the top.
function trail(at, rootNames, allLabel) {
    const steps = [];
    if (rootNames.length > 1)
        steps.push({
            label: allLabel,
            place: null
        });
    if (at === null)
        return steps;
    steps.push({
        label: rootNames[at.root] || "?",
        place: {
            root: at.root,
            folder: ""
        }
    });
    let path = "";
    for (const part of at.folder.split("/").filter(p => p !== "")) {
        path = path === "" ? part : path + "/" + part;
        steps.push({
            label: part,
            place: {
                root: at.root,
                folder: path
            }
        });
    }
    return steps;
}
