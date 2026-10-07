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
import qs.views.online
import qs.views.collections
import qs.views.settings

Item {
    id: picker

    /// The content (for the test hooks' coordinates).
    readonly property alias content: content

    // What Local shows outlives its view (another tab is open meanwhile).
    LocalModel {
        id: localModel
    }

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
                // Settings → Performance (and Appearance): off, the plain color.
                visible: Prefs.backdrop
                anchors.fill: parent
                source: {
                    const name = Monitors.selection === Monitors.all ? (Monitors.list[0] || {}).name : Monitors.selection === Monitors.spanTarget ? Monitors.spanNames[0] : Monitors.selection;
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
                tab: Views.current
                onTabPicked: index => Views.current = index
            }
            MonitorDrawer {
                Layout.fillWidth: true
            }
            // One tab at a time: the others aren't kept (nor their pictures).
            Loader {
                Layout.fillWidth: true
                Layout.fillHeight: true
                sourceComponent: [localTab, onlineTab, collectionsTab][Views.current]
            }
        }
    }

    Component {
        id: localTab
        LocalView {
            local: localModel
            // The glass over its grid blurs the wallpaper too.
            backdrop: scenery
        }
    }
    Component {
        id: onlineTab
        OnlineView {
            backdrop: scenery
        }
    }
    Component {
        id: collectionsTab
        CollectionsView {
            backdrop: scenery
        }
    }

    // Ctrl+1/2/3: Local, Online, Collections (while no menu or dialog is open).
    Repeater {
        model: 3
        Item {
            id: tabKey
            required property int index
            Shortcut {
                enabled: Dialogs.calm
                sequence: "Ctrl+" + (tabKey.index + 1)
                onActivated: Views.current = tabKey.index
            }
        }
    }

    // Alt+1..N: where a click applies, a monitor in the top bar's order (one
    // of the span's picks the span); Alt+N+1: All (two monitors: Alt+1,
    // Alt+2, Alt+3 for All).
    Repeater {
        model: Math.min(9, Monitors.list.length + 1)
        Item {
            id: monitorKey
            required property int index
            Shortcut {
                enabled: Dialogs.calm
                sequence: "Alt+" + (monitorKey.index + 1)
                onActivated: Monitors.select(monitorKey.index < Monitors.list.length ? Monitors.list[monitorKey.index].name : Monitors.all)
            }
        }
    }

    // The dialogs, once for everything (services/Dialogs opens them).
    TagsDialog {}
    TrashDialog {}
    DuplicatesDialog {}
    AdjustDialog {}
    SpanDialog {}
    FiltersDialog {}
    DetailsDialog {}
    SettingsDialog {}

    ToastStack {
        host: panel
    }

    // Swallow clicks on the panel (a click outside closes it).
    MouseArea {
        anchors.fill: panel
        z: -1
    }
}
