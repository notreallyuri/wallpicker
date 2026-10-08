// A local wallpaper's details (right-click → Details…): where it came from
// and who made it (the artists, each with More by <artist> on the provider
// it came from; the uploader, More from; the source link; Open on the
// provider's site), the artists and tags (each removable, a field to add
// one), and the tags used elsewhere that match what's typed. A More search
// goes to the Online tab.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

Dialog {
    id: dialog

    readonly property string path: Dialogs.localDetailsFor
    readonly property var current: path !== "" ? Library.tags(path) : []
    readonly property var meta: path !== "" ? Library.metadata(path) : Library.metadata("")
    /// The provider it came from ("" when it wasn't downloaded).
    readonly property string provider: meta.origin ? meta.origin.provider : ""
    readonly property string providerLabel: provider !== "" ? Online.labelOf(provider) : ""
    /// Its page there ("" while asked, or when there's none).
    property string originPage: ""
    readonly property var image: path !== "" ? Library.images.find(i => i.path === path) : null
    // Tags used elsewhere that it doesn't have, matching what's typed.
    readonly property var suggestions: {
        const typed = field.text.trim().toLowerCase();
        const mine = current.map(t => t.toLowerCase());
        const seen = {};
        const all = [];
        for (const p in Library.entries)
            for (const t of (Library.entries[p].tags || [])) {
                const k = t.toLowerCase();
                if (!mine.includes(k) && k.includes(typed) && !seen[k]) {
                    seen[k] = true;
                    all.push(t);
                }
            }
        return all.sort((a, b) => a.toLowerCase().localeCompare(b.toLowerCase())).slice(0, 12);
    }

    /// As core's normalize_tag: spaces collapsed, 1 to 64 characters.
    function normalized(raw) {
        const tag = raw.split(/\s+/).filter(w => w !== "").join(" ");
        return tag.length > 0 && tag.length <= 64 && !/[\u0000-\u001f\u007f]/.test(tag) ? tag : "";
    }
    function add(raw) {
        const tag = normalized(raw);
        if (tag === "")
            return;
        field.text = "";
        Library.addTag(path, tag);
    }
    function addArtist(raw) {
        const name = normalized(raw);
        if (name === "")
            return;
        artistField.text = "";
        if (!meta.artists.some(a => a.name.toLowerCase() === name.toLowerCase()))
            Library.setArtists(path, meta.artists.concat([{
                    name: name
                }]));
    }
    function removeArtist(name) {
        Library.setArtists(path, meta.artists.filter(a => a.name !== name));
    }
    /// More by / from: searched on the Online tab, on the provider it came from.
    function searchFor(q) {
        const on = provider;
        close();
        Online.searchFor(q, on);
        Views.current = Views.online;
    }
    function openUrl(url) {
        Backend.request("online.open_url", {
            url: url
        }, null, e => Toasts.error("Could not open the link", e));
    }

    title: path.split("/").pop()
    description: {
        const parts = [];
        if (image && image.width > 0)
            parts.push(image.width + "×" + image.height);
        if (providerLabel !== "")
            parts.push("From " + providerLabel);
        return parts.join(" · ");
    }
    preferredWidth: 560
    onClosed: Dialogs.localDetailsFor = ""
    onOpened: field.forceFocus()

    onPathChanged: {
        originPage = "";
        if (path === "")
            return;
        const asked = path;
        Backend.request("library.origin_page", {
            path: asked
        }, url => {
            if (dialog.path === asked)
                dialog.originPage = url || "";
        });
    }

    Connections {
        target: Dialogs
        function onLocalDetailsForChanged() {
            if (Dialogs.localDetailsFor !== "")
                dialog.open();
        }
    }

    // The artists: each removable, with More by (where the provider can
    // search for them); one can be added by hand.
    DetailsLabel {
        text: "Artists"
        muted: true
    }
    Text {
        visible: dialog.meta.artists.length === 0
        text: "No artist known."
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 14
    }
    Repeater {
        model: dialog.meta.artists
        RowLayout {
            required property var modelData
            spacing: 8
            RemovableChip {
                icon: "brush"
                text: modelData.name
                removeTip: "Not by " + modelData.name
                onRemoved: dialog.removeArtist(modelData.name)
            }
            Btn {
                visible: !!modelData.query && dialog.provider !== ""
                small: true
                variant: "primary"
                text: "More by " + modelData.name
                tip: "Search " + dialog.providerLabel
                onClicked: dialog.searchFor(modelData.query)
            }
        }
    }
    RowLayout {
        Layout.fillWidth: true
        spacing: 8
        TextField {
            id: artistField
            Layout.fillWidth: true
            placeholder: "Add an artist"
            onAccepted: dialog.addArtist(text)
        }
        Btn {
            variant: "primary"
            text: "Add"
            opacity: dialog.normalized(artistField.text) !== "" ? 1 : 0.5
            onClicked: dialog.addArtist(artistField.text)
        }
    }

    RowLayout {
        visible: dialog.meta.uploader !== null || dialog.meta.source !== null
        spacing: 8
        DetailsLabel {
            visible: dialog.meta.uploader !== null
            text: "Uploaded by"
            muted: true
        }
        DetailsLabel {
            visible: dialog.meta.uploader !== null
            text: dialog.meta.uploader ? dialog.meta.uploader.name : ""
        }
        Btn {
            visible: !!(dialog.meta.uploader && dialog.meta.uploader.query) && dialog.provider !== ""
            small: true
            text: dialog.meta.uploader ? "More from " + dialog.meta.uploader.name : ""
            onClicked: dialog.searchFor(dialog.meta.uploader.query)
        }
        Btn {
            visible: dialog.meta.source !== null
            small: true
            icon: "external-link"
            text: "Open source"
            tip: dialog.meta.source || ""
            onClicked: dialog.openUrl(dialog.meta.source)
        }
    }

    DetailsLabel {
        text: "Tags"
        muted: true
    }
    Flow {
        Layout.fillWidth: true
        spacing: 6
        Text {
            visible: dialog.current.length === 0
            text: "No tags yet."
            color: Theme.mutedForeground
            font.family: Theme.font
            font.pixelSize: 14
        }
        Repeater {
            model: dialog.current
            RemovableChip {
                required property string modelData
                text: modelData
                removeTip: "Remove the tag"
                onRemoved: Library.removeTag(dialog.path, modelData)
            }
        }
    }
    RowLayout {
        Layout.fillWidth: true
        spacing: 8
        TextField {
            id: field
            Layout.fillWidth: true
            placeholder: "Add a tag"
            onAccepted: dialog.add(text)
        }
        Btn {
            variant: "primary"
            text: "Add"
            opacity: dialog.normalized(field.text) !== "" ? 1 : 0.5
            onClicked: dialog.add(field.text)
        }
    }
    Flow {
        visible: dialog.suggestions.length > 0
        Layout.fillWidth: true
        spacing: 6
        Repeater {
            model: dialog.suggestions
            Btn {
                required property string modelData
                implicitHeight: 26
                text: "+ " + modelData
                onClicked: dialog.add(modelData)
            }
        }
    }

    footerLeft: Btn {
        visible: dialog.originPage !== ""
        variant: "ghost"
        icon: "external-link"
        text: "Open on " + dialog.providerLabel
        onClicked: dialog.openUrl(dialog.originPage)
    }
    footer: [
        Btn {
            variant: "ghost"
            text: "Done"
            onClicked: dialog.close()
        }
    ]
}
