// An entry in a Menu: an optional icon, the label, an optional hint at the
// right (a shortcut). Destructive ones are red. Choosing it closes the menu.
import QtQuick
import QtQuick.Layouts
import qs.services

Rectangle {
    id: item

    property string text
    property string icon
    property string hint
    property bool destructive: false
    property bool closes: true
    /// Space for a check mark, so MenuCheck labels line up.
    property bool checkSpace: false
    default property alias trailing: extras.data
    readonly property bool hovered: mouse.containsMouse
    signal triggered

    readonly property color ink: !enabled ? Theme.a(Theme.foreground, 0.4) : destructive ? "#fd4663" : Theme.foreground

    Layout.fillWidth: true
    implicitWidth: row.implicitWidth + 20
    implicitHeight: 34
    radius: 10
    color: hovered && enabled ? (destructive ? Qt.rgba(0.99, 0.27, 0.39, 0.12) : Theme.a(Theme.primary, 0.12)) : "transparent"

    /// Close the menu this is in.
    function closeMenu() {
        for (let p = parent; p; p = p.parent)
            if (p.owner !== undefined) {
                p.owner.close();
                return;
            }
    }

    RowLayout {
        id: row
        anchors.fill: parent
        anchors.leftMargin: 10
        anchors.rightMargin: 10
        spacing: 8
        Item {
            visible: item.checkSpace
            implicitWidth: 16
        }
        Icon {
            visible: item.icon !== ""
            name: item.icon
            color: item.ink
        }
        Text {
            Layout.fillWidth: true
            text: item.text
            color: item.ink
            font.family: Theme.font
            font.pixelSize: 14
            elide: Text.ElideRight
        }
        Row {
            id: extras
            spacing: 6
        }
        Text {
            visible: item.hint !== ""
            text: item.hint
            color: Theme.mutedForeground
            font.family: Theme.font
            font.pixelSize: 12
        }
    }
    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: item.enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
        onClicked: {
            if (!item.enabled)
                return;
            item.triggered();
            if (item.closes)
                item.closeMenu();
        }
    }
}
