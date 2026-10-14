// The picker itself: a rounded panel with the wallpaper blurred behind it,
// the top bar, the current tab and, when it's open, the control panel
// beside it (over it at Compact sizes).
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
import qs.views.presets
import qs.views.collections
import qs.views.settings
import qs.panel

Item {
    id: picker

    /// The content (for the test hooks' coordinates).
    readonly property alias content: content

    /// Close the control panel while it floats over the grid (Esc);
    /// whether it did.
    function closePanel() {
        if (!Prefs.panelOpen || !body.overlay)
            return false;
        Prefs.panelOpen = false;
        return true;
    }

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
            Item {
                id: body
                Layout.fillWidth: true
                Layout.fillHeight: true

                // Too narrow to give the panel its room (Compact): it floats
                // over the grid instead of narrowing it.
                readonly property bool overlay: width < 1100
                readonly property bool open: Prefs.panelOpen
                // While its edge is dragged; saved when it's let go.
                property real dragWidth: -1
                readonly property real panelWidth: Math.max(320, Math.min(560, width - 400, dragWidth >= 0 ? dragWidth : Prefs.panelWidth))
                // The edge toggle's room at the right.
                readonly property real toggleRoom: 24
                // 0 closed, 1 open, in between while it slides: the panel
                // comes in from the right and the grid narrows with it.
                property real shown: open ? 1 : 0
                Behavior on shown {
                    NumberAnimation {
                        duration: Theme.duration
                        easing.type: Easing.OutCubic
                    }
                }
                // Where the panel's left edge is, all the way in (12 px from
                // the right) or out past the edge.
                readonly property real panelLeft: width - (panelWidth + 12) * shown

                // One tab at a time: the others aren't kept (nor their pictures).
                Loader {
                    anchors.left: parent.left
                    anchors.top: parent.top
                    anchors.bottom: parent.bottom
                    width: (body.overlay ? body.width : body.panelLeft) - body.toggleRoom
                    sourceComponent: [localTab, onlineTab, collectionsTab, presetsTab][Views.current]
                }

                // The panel, its handle and its toggle, cut at the picker's
                // edge while the panel slides in and out.
                Item {
                    anchors.fill: parent
                    clip: true
                    ControlPanel {
                        id: controlPanel
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom
                        anchors.topMargin: 12
                        x: body.panelLeft
                        width: body.panelWidth
                        floating: body.overlay
                        visible: body.shown > 0
                        opacity: body.shown
                        onCloseRequested: Prefs.panelOpen = false
                    }

                    // Drag the panel's left edge to resize it (320–560 px).
                    MouseArea {
                        id: resize
                        visible: body.open
                        x: controlPanel.x - 5
                        y: controlPanel.y + 24
                        width: 10
                        height: controlPanel.height - 48
                        hoverEnabled: true
                        cursorShape: Qt.SizeHorCursor
                        property real startX
                        property real startWidth
                        onPressed: mouse => {
                            startX = mapToItem(body, mouse.x, 0).x;
                            startWidth = body.panelWidth;
                            body.dragWidth = startWidth;
                        }
                        onPositionChanged: mouse => {
                            if (pressed)
                                body.dragWidth = Math.max(320, Math.min(560, startWidth + startX - mapToItem(body, mouse.x, 0).x));
                        }
                        onReleased: {
                            Prefs.panelWidth = Math.round(body.panelWidth);
                            body.dragWidth = -1;
                        }
                        Rectangle {
                            anchors.centerIn: parent
                            width: 4
                            height: resize.containsMouse || resize.pressed ? 48 : 32
                            radius: 2
                            color: resize.containsMouse || resize.pressed ? Theme.primary : Theme.a(Theme.foreground, 0.12)
                            Behavior on height {
                                NumberAnimation {
                                    duration: Theme.ms(150)
                                }
                            }
                        }
                    }

                    EdgeToggle {
                        open: body.open
                        x: body.panelLeft - width
                        y: Math.round((body.height - height) / 2)
                        onClicked: Prefs.panelOpen = !Prefs.panelOpen
                    }
                }
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

    Component {
        id: presetsTab
        PresetsView {}
    }

    // Ctrl+1/2/3/4: Local, Online, Collections, Presets (while no popup is open).
    Repeater {
        model: 4
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
    LocalDetailsDialog {}
    TrashDialog {}
    DuplicatesDialog {}
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
