// The picker in an ordinary window instead of the floating panel
// (WALLPICKER_WINDOW=1): for compositors without layer-shell, and the
// headless checks (cage has none).
import QtQuick
import Quickshell
import qs.services

FloatingWindow {
    id: window

    signal dismissed

    readonly property alias picker: picker

    title: "wallpicker"
    implicitWidth: 1560
    implicitHeight: 980
    color: "transparent"
    // Hidden while closed but still loaded (services/Lifetime).
    visible: Lifetime.shown
    onVisibleChanged: if (!visible)
        window.dismissed()

    Picker {
        id: picker
        anchors.fill: parent
    }

    // Esc closes the top menu or dialog; with none open, the picker.
    Shortcut {
        sequence: "Escape"
        onActivated: if (!Dialogs.closeTop())
            window.dismissed()
    }
}
