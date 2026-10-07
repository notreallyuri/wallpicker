// The span's card in the drawer: its monitors as arranged on the canvas,
// one picture across them; Adjust (the Span dialog: zoom, position,
// mirroring, which monitors and where), Separate (each monitor on its own
// again, keeping its part) and Previous (on the span's monitors). Picked,
// a click on a tile spans that picture across them.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services
import "../components/Span.js" as SpanLayout

DrawerCard {
    id: card

    readonly property string shown: Monitors.spanState ? Monitors.spanState.path : ""
    readonly property var image: Library.images.find(i => i.path === shown)
    // The library's thumbnail, else one made on demand.
    property string thumb: ""
    onShownChanged: loadThumb()
    Component.onCompleted: loadThumb()
    function loadThumb() {
        const img = Library.images.find(i => i.path === shown);
        thumb = img && img.thumb ? img.thumb : "";
        if (shown !== "" && thumb === "") {
            const wanted = shown;
            Library.thumbnail(shown, f => {
                if (card.shown === wanted)
                    card.thumb = f;
            });
        }
    }

    implicitWidth: Math.max(288, Math.min(440, layout.width + 24))
    picked: Monitors.selection === Monitors.spanTarget
    onClicked: Monitors.select(Monitors.spanTarget)

    // The monitors as arranged, one picture across them (h-32).
    Item {
        Layout.fillWidth: true
        implicitHeight: 128
        readonly property var fitted: SpanLayout.fit(Monitors.spanPlaces, 416, 112, 6)
        SpanView {
            id: layout
            anchors.centerIn: parent
            width: parent.fitted.width
            height: parent.fitted.height
            places: Monitors.spanPlaces
            spanned: Monitors.spanNames
            look: Monitors.spanState ? Monitors.spanState.look : SpanLayout.initial()
            source: card.thumb ? "file://" + card.thumb : ""
            naturalWidth: card.image ? card.image.width : 0
            naturalHeight: card.image ? card.image.height : 0
            mapping: parent.fitted
            showNames: false
            ring: 2
        }
    }
    RowLayout {
        Layout.fillWidth: true
        spacing: 8
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0
            RowLayout {
                spacing: 6
                Text {
                    Layout.alignment: Qt.AlignBaseline
                    text: "Spanned"
                    color: Theme.foreground
                    font.family: Theme.font
                    font.pixelSize: 14
                    font.weight: Font.ExtraBold
                }
                Text {
                    Layout.alignment: Qt.AlignBaseline
                    text: Monitors.spanNames.join(" + ")
                    color: Theme.mutedForeground
                    font.family: Theme.font
                    font.pixelSize: 12
                    font.weight: Font.DemiBold
                }
            }
            Text {
                Layout.fillWidth: true
                text: card.shown.split("/").pop()
                elide: Text.ElideRight
                color: Theme.mutedForeground
                font.family: Theme.font
                font.pixelSize: 12
            }
        }
        Btn {
            variant: "ghost"
            icon: "sliders-horizontal"
            implicitHeight: 32
            tip: "Adjust the span: zoom, position, mirror; which monitors and where"
            onClicked: Dialogs.adjustSpan()
        }
        Btn {
            variant: "ghost"
            icon: "unlink"
            implicitHeight: 32
            tip: "Separate: each monitor on its own again, keeping its part"
            onClicked: Span.separate()
        }
        Btn {
            variant: "ghost"
            icon: "rotate-ccw"
            implicitHeight: 32
            tip: "Back to the wallpapers before, on " + Monitors.spanNames.join(" and ")
            onClicked: Monitors.previousOn(Monitors.spanNames, applied => applied.length > 0 ? "Back on " + applied.join(", ") : "Nothing earlier to go back to")
        }
    }
}
