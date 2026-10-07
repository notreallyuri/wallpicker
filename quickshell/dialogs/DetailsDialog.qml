// An online wallpaper's details (ui/src/views/online/details.rs): its size
// and category, likes, views and file size; a bigger picture; the artists,
// uploader, source and tags, each of which searches for more; Open on the
// provider's site, Download, Apply. Opened by services/Dialogs.showDetails.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

Dialog {
    id: dialog

    readonly property var w: Dialogs.details
    readonly property string key: w ? Downloads.key(w) : ""
    readonly property bool saved: w !== null && Downloads.isSaved(w)
    readonly property bool working: w !== null && Downloads.isWorking(w)
    /// {key, details} or {key, error}; null while loading.
    property var info: null

    title: w ? w.width + "×" + w.height + (w.category ? " · " + w.category : "") : ""
    description: {
        if (!w)
            return "";
        const stats = [w.provider === "konachan" || w.provider === "yandere" ? "Score " + w.likes : "♥ " + w.likes];
        if (w.views !== null && w.views !== undefined)
            stats.push(w.views + " views");
        if (w.file_size > 0)
            stats.push((w.file_size / 1000000).toFixed(1) + " MB");
        if (saved)
            stats.push("Downloaded");
        return stats.join(" · ");
    }
    preferredWidth: 720

    onWChanged: {
        if (!w)
            return;
        load();
        open();
    }
    onClosed: Dialogs.details = null

    function load() {
        // From `w`: in its change handler, `key` can still be the last one.
        const k = Downloads.key(w);
        // Moebooru results carry theirs.
        if (w.details) {
            info = {
                key: k,
                details: w.details
            };
            return;
        }
        // Reopening the same one reuses them (Wallhaven allows 45 requests
        // a minute).
        if (info && info.key === k && info.details)
            return;
        info = null;
        Backend.request("online.details", {
            wallpaper: w
        }, d => {
            if (dialog.key === k)
                dialog.info = {
                    key: k,
                    details: d
                };
        }, e => {
            if (dialog.key === k)
                dialog.info = {
                    key: k,
                    error: e
                };
        });
    }

    /// More like this (a tag, an artist, an uploader): searched on the
    /// Online tab, which replaces the results behind the dialog.
    function searchFor(q) {
        const provider = w.provider;
        close();
        Online.searchFor(q, provider);
        Views.current = Views.online;
    }

    function openUrl(url) {
        Backend.request("online.open_url", {
            url: url
        }, null, e => Toasts.error("Could not open the link", e));
    }

    DetailsPreview {
        Layout.fillWidth: true
        // Within 45% of the window, as the app's max-h-[45vh].
        Layout.preferredHeight: Math.min(width * 10 / 16, (dialog.parent ? dialog.parent.height : 900) * 0.45)
        w: dialog.w
    }
    DetailsAbout {
        Layout.fillWidth: true
        info: dialog.info && dialog.info.key === dialog.key ? dialog.info : null
        photo: !!dialog.w && dialog.w.provider === "unsplash"
        onSearch: q => dialog.searchFor(q)
        onOpenUrl: url => dialog.openUrl(url)
    }

    footerLeft: Btn {
        visible: dialog.w !== null && /^https?:\/\//.test(dialog.w.page_url)
        variant: "ghost"
        icon: "external-link"
        text: dialog.w ? "Open on " + Online.labelOf(dialog.w.provider) : ""
        onClicked: dialog.openUrl(dialog.w.page_url)
    }
    footer: [
        Btn {
            visible: !dialog.saved
            icon: "download"
            text: "Download"
            opacity: dialog.working ? 0.5 : 1
            onClicked: if (!dialog.working)
                Downloads.fetch(dialog.w, null)
        },
        Btn {
            variant: "primary"
            text: "Apply"
            opacity: dialog.working ? 0.5 : 1
            onClicked: if (!dialog.working) {
                Downloads.fetch(dialog.w, Monitors.targets);
                dialog.close();
            }
        }
    ]
}
