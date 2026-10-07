// The picker as a floating panel: a layer-shell overlay over the focused
// monitor (namespace "wallpicker", for Hyprland's layer rules), the panel
// centered on it with a light dim around. It takes the keyboard while open
// (type to search); Esc or a click outside closes it. It fades and scales
// in.
import QtQuick
import Quickshell
import qs.services
import Quickshell.Hyprland
import Quickshell.Wayland

PanelWindow {
    id: overlay

    /// Asked to close (Esc, a click outside).
    signal dismissed

    readonly property alias picker: picker

    // Out of the way while a file dialog is open: an overlay sits above
    // every window, the dialog too.
    visible: !Files.asking

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
        // The chosen size (Settings → Appearance): an even margin all round,
        // a share of the screen's height, so each preset suits any screen
        // (a 720p one too). At least 960x540, at most 2200 wide (an ultrawide
        // screen would make it a strip), never closer than 24 px to an edge.
        readonly property var size: {
            const sw = overlay.width, sh = overlay.height;
            const share = ({
                    large: 0.05,
                    comfortable: 0.10,
                    compact: 0.15
                })[Prefs.panelSize];
            const m = Math.round(sh * (share === undefined ? 0.10 : share));
            const w = Math.max(960, Math.min(2200, sw - 2 * m));
            const h = Math.max(540, sh - 2 * m);
            return [Math.min(w, sw - 48), Math.min(h, sh - 48)];
        }
        width: size[0]
        height: size[1]
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
            duration: Theme.ms(200)
        }
        NumberAnimation {
            target: picker
            property: "opacity"
            to: 1
            duration: Theme.ms(200)
        }
        NumberAnimation {
            target: picker
            property: "scale"
            to: 1
            duration: Theme.ms(260)
            easing.type: Easing.OutCubic
        }
    }

    // Esc closes the top menu or dialog; with none open, the picker.
    Shortcut {
        sequence: "Escape"
        onActivated: if (!Dialogs.closeTop())
            overlay.dismissed()
    }
}
