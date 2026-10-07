// Online wallpapers and the download folder: which are there already
// (`saved`), which are on their way (`working`, with their progress), and
// fetching one, then applying it if asked. Keys are "provider:id". Shared
// by the Online and Collections tabs and the details dialog.
pragma Singleton
import QtQuick
import Quickshell

Singleton {
    id: downloads

    /// key -> true: in the download folder (applying won't download again).
    property var saved: ({})
    /// key -> {done, total} (bytes; total 0 when the server doesn't say).
    property var working: ({})

    function key(w) {
        return w.provider + ":" + w.id;
    }
    function isSaved(w) {
        return !!saved[key(w)];
    }
    function isWorking(w) {
        return !!working[key(w)];
    }
    /// 0-1, or -1 while it isn't known.
    function progress(w) {
        const p = working[key(w)];
        return p && p.total > 0 ? p.done / p.total : -1;
    }

    /// Download `w`, then apply it on `monitors` (names; "span": across all
    /// of them), or just say where it was saved when that's null. Once at a
    /// time per wallpaper.
    function fetch(w, monitors) {
        const k = key(w);
        if (working[k])
            return;
        setWorking(k, {
            done: 0,
            total: 0
        });
        Backend.request("online.download", {
            wallpaper: w
        }, path => {
            setWorking(k, null);
            const next = Object.assign({}, downloads.saved);
            next[k] = true;
            downloads.saved = next;
            if (monitors === "span")
                Monitors.span(path);
            else if (monitors)
                Monitors.applyTo(path, monitors);
            else
                Toasts.info("Saved " + path.split("/").pop(), path);
        }, e => {
            setWorking(k, null);
            Toasts.error("Download failed", e);
        });
    }

    function setWorking(k, p) {
        const next = Object.assign({}, working);
        if (p)
            next[k] = p;
        else
            delete next[k];
        working = next;
    }

    function reload() {
        Backend.request("online.downloaded", {}, keys => {
            const next = {};
            for (const k of keys || [])
                next[k] = true;
            downloads.saved = next;
        }, e => console.warn("wallpicker: could not list the downloaded wallpapers:", e));
    }

    Component.onCompleted: reload()

    Connections {
        target: Backend
        function onEvent(name, data) {
            if (name === "download-progress" && downloads.working[data.key])
                downloads.setWorking(data.key, {
                    done: data.done,
                    total: data.total || 0
                });
            else if (name === "library-changed" || name === "settings-changed")
                downloads.reload();
        }
    }
}
