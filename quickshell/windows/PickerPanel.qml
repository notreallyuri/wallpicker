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

    // Hidden while closed but still loaded (services/Lifetime), and out of
    // the way while a file dialog is open: an overlay sits above every
    // window, the dialog too.
    visible: Lifetime.shown && !Files.asking
    // Shown again: the way in plays again.
    onVisibleChanged: if (visible && !Files.asking) {
        dim.opacity = 0;
        picker.opacity = 0;
        picker.scale = 0.97;
        comeIn.restart();
    }

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
        // The chosen size (Settings → Appearance): a share of the screen's
        // height as the margin above and below, and the old window's shape
        // (1100x760, ~1.45:1) for the width, so each preset suits any screen
        // (Compact on 1080p is about the old window). At least 960x540,
        // never closer than 24 px to an edge.
        readonly property var size: {
            const sw = overlay.width, sh = overlay.height;
            const share = ({
                    large: 0.05,
                    comfortable: 0.10,
                    compact: 0.15
                })[Prefs.panelSize];
            const m = Math.round(sh * (share === undefined ? 0.10 : share));
            const h = Math.max(540, sh - 2 * m);
            const w = Math.max(960, Math.round(h * 1100 / 760));
            return [Math.min(w, sw - 48), Math.min(h, sh - 48)];
        }
        width: size[0]
        height: size[1]
        opacity: 0
        scale: 0.97
    }

    // In: the dim and the panel fade, the panel settles from 97%.
    ParallelAnimation {
        id: comeIn
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

    // Esc closes the top menu or dialog, then the control panel floating
    // over the grid; with none open, the picker.
    Shortcut {
        sequence: "Escape"
        onActivated: if (!Dialogs.closeTop() && !picker.closePanel())
            overlay.dismissed()
    }
}
