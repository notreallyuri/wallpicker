// The window: the blurred wallpaper behind, the top bar, the current tab.
import QtQuick
import QtQuick.Layouts
import Quickshell
import qs.bar
import qs.components
import qs.services
import qs.views.local

FloatingWindow {
    id: window

    property int tab: 0

    title: "wallpicker"
    implicitWidth: 1400
    implicitHeight: 900
    color: Theme.background

    // The chosen monitor's wallpaper (the first one's, for All).
    Backdrop {
        anchors.fill: parent
        source: {
            const name = Monitors.selection === Monitors.all ? (Monitors.list[0] || {}).name : Monitors.selection;
            const img = Library.images.find(i => i.path === Monitors.current[name]);
            return img && img.thumb ? "file://" + img.thumb : "";
        }
    }

    ColumnLayout {
        id: content
        anchors.fill: parent
        spacing: 0
        TopBar {
            Layout.fillWidth: true
            tab: window.tab
            onTabPicked: index => window.tab = index
        }
        LocalView {
            Layout.fillWidth: true
            Layout.fillHeight: true
        }
    }

    // Headless checks only.
    Loader {
        active: Quickshell.env("WALLPICKER_TEST") === "1"
        sourceComponent: TestHooks {
            root: content
        }
    }
}
