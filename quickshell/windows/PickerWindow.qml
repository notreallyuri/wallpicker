// The picker in an ordinary window instead of the floating panel
// (WALLPICKER_WINDOW=1): for compositors without layer-shell, and the
// headless checks (cage has none).
import QtQuick
import Quickshell

FloatingWindow {
    id: window

    signal dismissed

    readonly property alias picker: picker

    title: "wallpicker"
    implicitWidth: 1560
    implicitHeight: 980
    color: "transparent"
    onVisibleChanged: if (!visible)
        window.dismissed()

    Picker {
        id: picker
        anchors.fill: parent
    }

    Shortcut {
        sequence: "Escape"
        onActivated: window.dismissed()
    }
}
