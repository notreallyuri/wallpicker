// The picker as a floating panel: a layer-shell overlay over the focused
// monitor (namespace "wallpicker", for Hyprland's layer rules), the panel
// centered on it with a light dim around. It takes the keyboard while open
// (type to search); Esc or a click outside closes it. It fades and scales
// in.
import QtQuick
import Quickshell
import Quickshell.Hyprland
import Quickshell.Wayland

PanelWindow {
    id: overlay

    /// Asked to close (Esc, a click outside).
    signal dismissed

    readonly property alias picker: picker

    screen: Quickshell.screens.find(s => Hyprland.focusedMonitor && s.name === Hyprland.focusedMonitor.name) || Quickshell.screens[0]
    anchors {
        top: true
        bottom: true
        left: true
        right: true
    }
    exclusionMode: ExclusionMode.Ignore
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "wallpicker"
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
    color: "transparent"

    Rectangle {
        id: dim
        anchors.fill: parent
        color: Qt.rgba(0, 0, 0, 0.25)
        opacity: 0
        MouseArea {
            anchors.fill: parent
            onClicked: overlay.dismissed()
        }
    }

    Picker {
        id: picker
        anchors.centerIn: parent
        width: Math.min(1560, overlay.width - 96)
        height: Math.min(980, overlay.height - 96)
        opacity: 0
        scale: 0.97
    }

    // In: the dim and the panel fade, the panel settles from 97%.
    ParallelAnimation {
        running: true
        NumberAnimation {
            target: dim
            property: "opacity"
            to: 1
            duration: 200
        }
        NumberAnimation {
            target: picker
            property: "opacity"
            to: 1
            duration: 200
        }
        NumberAnimation {
            target: picker
            property: "scale"
            to: 1
            duration: 260
            easing.type: Easing.OutCubic
        }
    }

    Shortcut {
        sequence: "Escape"
        onActivated: overlay.dismissed()
    }
}
