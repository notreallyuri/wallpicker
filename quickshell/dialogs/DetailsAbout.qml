// The details' text: the artists ("More by"), the uploader ("More from"),
// the source, and the tags (their category on hover), each a search, each
// with a + to keep it in the library; or that they're loading, or why they
// couldn't be. Dozens of tags scroll.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qs.components
import qs.services

ColumnLayout {
    id: about

    /// {details} or {error}; null while loading.
    property var info: null
    /// The tag names kept: added to the library with Download or Apply.
    property var chosen: []
    signal search(string query)
    signal openUrl(string url)
    signal toggle(string name)
    signal clearChosen

    readonly property var d: info && info.details ? info.details : null
    readonly property var artists: d ? d.tags.filter(t => t.is_artist) : []
    readonly property var tags: d ? d.tags.filter(t => !t.is_artist) : []

    spacing: 12

    RowLayout {
        visible: about.info === null
        spacing: 8
        Spinner {
            size: 16
        }
        Text {
            text: "Loading tags…"
            color: Theme.mutedForeground
            font.family: Theme.font
            font.pixelSize: 14
        }
    }
    Text {
        visible: about.info !== null && about.d === null
        Layout.fillWidth: true
        wrapMode: Text.Wrap
        text: about.info && about.info.error ? "Could not load the details: " + about.info.error : ""
        color: Theme.mutedForeground
        font.family: Theme.font
        font.pixelSize: 14
    }

    Repeater {
        model: about.artists
        RowLayout {
            required property var modelData
            spacing: 8
            DetailsLabel {
                text: "Artist"
                muted: true
            }
            DetailsLabel {
                text: modelData.name
            }
            Btn {
                small: true
                variant: "primary"
                text: "More by " + modelData.name
                onClicked: about.search(modelData.query)
            }
        }
    }
    RowLayout {
        visible: about.d !== null && (about.d.uploader !== null || about.d.source !== "")
        spacing: 8
        DetailsLabel {
            visible: !!(about.d && about.d.uploader)
            text: "Uploaded by"
            muted: true
        }
        DetailsLabel {
            visible: !!(about.d && about.d.uploader)
            text: about.d && about.d.uploader ? about.d.uploader.name : ""
        }
        Btn {
            visible: !!(about.d && about.d.uploader && about.d.uploader.query)
            small: true
            text: about.d && about.d.uploader ? "More from " + about.d.uploader.name : ""
            onClicked: about.search(about.d.uploader.query)
        }
        Btn {
            visible: !!(about.d && about.d.source)
            small: true
            icon: "external-link"
            text: "Open source"
            tip: about.d ? about.d.source : ""
            onClicked: about.openUrl(about.d.source)
        }
    }

    RowLayout {
        visible: about.d !== null && about.tags.length > 0
        Layout.fillWidth: true
        spacing: 8
        Text {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            text: {
                const n = about.chosen.length;
                if (n === 0)
                    return "+ on a tag adds it to your library too, with Download or Apply.";
                return n + (n === 1 ? " tag" : " tags") + " will be added to your library.";
            }
            color: Theme.mutedForeground
            font.family: Theme.font
            font.pixelSize: 13
        }
        Btn {
            visible: about.chosen.length > 0
            small: true
            variant: "ghost"
            text: "Clear"
            onClicked: about.clearChosen()
        }
    }

    // The tags, scrolling past a few rows.
    Flickable {
        visible: about.d !== null
        Layout.fillWidth: true
        Layout.preferredHeight: Math.min(tagFlow.implicitHeight, 160)
        contentHeight: tagFlow.implicitHeight
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar {}
        Flow {
            id: tagFlow
            width: parent.width - 12
            spacing: 6
            DetailsLabel {
                visible: about.tags.length === 0
                text: "No tags."
                muted: true
            }
            Repeater {
                model: about.tags
                DetailsTag {
                    required property var modelData
                    text: modelData.name
                    tip: modelData.category
                    kept: about.chosen.includes(modelData.name)
                    onSearch: about.search(modelData.query)
                    onToggled: about.toggle(modelData.name)
                }
            }
        }
    }
}
