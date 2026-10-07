// The picker itself: a rounded panel with the wallpaper blurred behind it,
// the top bar and the current tab.
import QtQuick
import QtQuick.Effects
import QtQuick.Layouts
import Quickshell
import Quickshell.Widgets
import qs.bar
import qs.components
import qs.dialogs
import qs.services
import qs.views.local

Item {
    id: picker

    property int tab: 0
    /// The content (for the test hooks' coordinates).
    readonly property alias content: content

    RectangularShadow {
        anchors.fill: panel
        radius: panel.radius
        offset.y: 16
        blur: 48
        spread: -8
        color: Qt.rgba(0, 0, 0, 0.45)
    }

    Rectangle {
        id: panel
        anchors.fill: parent
        radius: 28
        color: Theme.background
        border.width: 1
        border.color: Theme.a(Theme.border, 0.6)

        // The chosen monitor's wallpaper (the first one's, for All), cut to
        // the panel's rounded corners. Only it is clipped that way: clipping
        // the whole panel redraws everything through a shader.
        ClippingRectangle {
            id: scenery
            anchors.fill: parent
            anchors.margins: 1
            radius: parent.radius - 1
            color: "transparent"
            Backdrop {
                anchors.fill: parent
                source: {
                    const name = Monitors.selection === Monitors.all ? (Monitors.list[0] || {}).name : Monitors.selection;
                    const img = Library.images.find(i => i.path === Monitors.current[name]);
                    return img && img.thumb ? "file://" + img.thumb : "";
                }
            }
        }

        ColumnLayout {
            id: content
            anchors.fill: parent
            // Clear of the rounded corners at the bottom.
            anchors.bottomMargin: 12
            spacing: 0
            TopBar {
                Layout.fillWidth: true
                // Its own background, rounded like the panel's top (it
                // stuck out past the corners).
                topLeftRadius: panel.radius
                topRightRadius: panel.radius
                tab: picker.tab
                onTabPicked: index => picker.tab = index
            }
            MonitorDrawer {
                Layout.fillWidth: true
            }
            LocalView {
                // The glass over its grid blurs the wallpaper too.
                backdrop: scenery
                Layout.fillWidth: true
                Layout.fillHeight: true
            }
        }
    }

    // The dialogs, once for everything (services/Dialogs opens them).
    TagsDialog {}
    TrashDialog {}
    DuplicatesDialog {}
    AdjustDialog {}

    ToastStack {
        anchors.right: panel.right
        anchors.bottom: panel.bottom
        anchors.margins: 20
        z: 10
    }

    // Swallow clicks on the panel (a click outside closes it).
    MouseArea {
        anchors.fill: panel
        z: -1
    }
}
