// The control panel (TODO.md's UI updates, stage 2): beside the grid,
// everything about the wallpaper where a click applies, following the
// bar's pills: one monitor (MonitorView), All (AllView) or the span
// (SpanView). A floating card like the search card; the picker resizes it
// from its left edge. The footer: Previous there, Favorite and Details of
// its picture, Stop spanning, Reset.
import QtQuick
import QtQuick.Effects
import QtQuick.Layouts
import qs.components
import qs.services

Item {
    id: panel

    signal closeRequested
    /// Over the grid (Compact): opaque, a deeper shadow.
    property bool floating: false

    readonly property string selection: Monitors.selection
    readonly property bool onSpan: selection === Monitors.spanTarget && Monitors.spanned
    readonly property bool onAll: selection === Monitors.all
    /// The view shown (its path, names, busy, adjusted, reset()).
    readonly property var view: body.item
    readonly property var info: Monitors.list.find(m => m.name === selection) || null
    readonly property bool inLibrary: !!view && view.path !== "" && Library.images.some(i => i.path === view.path)

    RectangularShadow {
        anchors.fill: card
        radius: card.radius
        offset.y: 8
        blur: 24
        spread: -4
        color: Qt.rgba(0, 0, 0, panel.floating ? 0.35 : 0.18)
    }

    Rectangle {
        id: card
        anchors.fill: parent
        radius: Theme.radiusXl
        color: panel.floating ? Theme.popover : Theme.a(Theme.card, 0.85)
        // The tiles' hairline.
        border.width: 1
        border.color: Theme.a(Theme.foreground, 0.05)
        clip: true

        ColumnLayout {
            anchors.fill: parent
            spacing: 0

            RowLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 20
                Layout.rightMargin: 10
                Layout.topMargin: 10
                Layout.bottomMargin: 4
                spacing: 8
                Text {
                    text: panel.onSpan ? "Spanned" : panel.onAll ? "All monitors" : panel.selection
                    color: Theme.foreground
                    font.family: Theme.font
                    font.pixelSize: 18
                    font.weight: Font.ExtraBold
                }
                Text {
                    Layout.fillWidth: true
                    elide: Text.ElideRight
                    text: panel.onSpan || panel.onAll ? (panel.view ? panel.view.names.join(" · ") : "") : panel.info ? panel.info.width + "×" + panel.info.height : ""
                    color: Theme.mutedForeground
                    font.family: Theme.font
                    font.pixelSize: 13
                }
                Btn {
                    variant: "ghost"
                    icon: "x"
                    tip: "Close the panel"
                    onClicked: panel.closeRequested()
                }
            }

            Flickable {
                id: flick
                Layout.fillWidth: true
                Layout.fillHeight: true
                contentHeight: body.height + 12
                boundsBehavior: Flickable.StopAtBounds
                clip: true
                // Only while it's shown: the preview asks for a framing copy.
                Loader {
                    id: body
                    active: panel.visible
                    x: 20
                    y: 6
                    width: flick.width - 40
                    height: item ? item.implicitHeight : 0
                    sourceComponent: panel.onSpan ? spanView : panel.onAll ? allView : monitorView
                }
            }

            Rectangle {
                Layout.fillWidth: true
                implicitHeight: 1
                color: Theme.a(Theme.border, 0.6)
            }
            RowLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 10
                Layout.rightMargin: 12
                Layout.topMargin: 8
                Layout.bottomMargin: 10
                spacing: 2
                Btn {
                    variant: "ghost"
                    icon: "history"
                    tip: "Back to the wallpaper before, here"
                    onClicked: Monitors.previousOn(panel.view.names, applied => applied.length > 0 ? "Back on " + applied.join(", ") : "Nothing earlier to go back to")
                }
                Btn {
                    id: favorite
                    readonly property bool on: panel.inLibrary && Library.favorite(panel.view.path)
                    visible: panel.inLibrary
                    variant: "ghost"
                    tip: on ? "Remove from the favorites" : "Add to the favorites"
                    onClicked: Library.setFavorite(panel.view.path, !on)
                    // Filled and yellow when it's one, as on the tiles.
                    Icon {
                        name: "star"
                        filled: favorite.on
                        color: favorite.on ? Theme.star : Theme.foreground
                    }
                }
                Btn {
                    visible: panel.inLibrary
                    variant: "ghost"
                    icon: "info"
                    tip: "Details: where it came from, its artists and tags"
                    onClicked: Dialogs.localDetails(panel.view.path)
                }
                Btn {
                    visible: panel.onSpan
                    variant: "ghost"
                    icon: "unlink"
                    tip: "Stop spanning: each monitor keeps its part"
                    onClicked: Span.separate()
                }
                Item {
                    Layout.fillWidth: true
                }
                Spinner {
                    visible: !!panel.view && panel.view.busy
                    size: 14
                }
                Btn {
                    readonly property bool none: !panel.view || !panel.view.adjusted
                    variant: "ghost"
                    icon: "rotate-ccw"
                    text: "Reset"
                    opacity: none ? 0.5 : 1
                    tip: "Back as it came: no frame, mirroring or fit of its own"
                    onClicked: panel.view.reset()
                }
            }
        }
    }

    Component {
        id: monitorView
        MonitorView {
            monitor: panel.selection
        }
    }
    Component {
        id: allView
        AllView {}
    }
    Component {
        id: spanView
        SpanView {}
    }
}
