// The bar across the top: the name, the tabs, where a click applies (the
// monitor pills), Previous and Settings. bg-background/55 over the
// backdrop, a hairline under it.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

Rectangle {
    id: bar

    property int tab: 0
    signal tabPicked(int index)

    implicitHeight: 56
    color: Theme.a(Theme.background, 0.55)

    Rectangle {
        anchors.bottom: parent.bottom
        width: parent.width
        height: 1
        color: Theme.a(Theme.border, 0.6)
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 20
        anchors.rightMargin: 20
        spacing: 16

        GradientText {
            text: "wallpicker"
        }
        Tabs {
            current: bar.tab
            onPicked: index => bar.tabPicked(index)
        }
        Item {
            Layout.fillWidth: true
        }
        MonitorPills {}
        // Back to the wallpaper before, on the chosen monitors.
        Btn {
            variant: "ghost"
            icon: "history"
            tip: "Back to the wallpaper before, on the selected monitors"
            onClicked: Monitors.previous()
        }
        Btn {
            variant: "ghost"
            icon: "settings"
            tip: "Settings"
            onClicked: Dialogs.openSettings()
        }
    }
}
