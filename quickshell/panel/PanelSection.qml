// A section of the control panel: a hairline, a header (its title, what
// it's set to, a chevron) and its contents. Click the header to fold it;
// folded, the header still says what it's set to.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

ColumnLayout {
    id: section

    property string title
    /// What it's set to ("Zoom 140%"), at the header's right.
    property string hint
    property bool open: true
    default property alias content: body.data

    spacing: 8

    Rectangle {
        Layout.fillWidth: true
        implicitHeight: 1
        color: Theme.a(Theme.border, 0.6)
    }

    Item {
        Layout.fillWidth: true
        implicitHeight: header.implicitHeight + 8
        RowLayout {
            id: header
            anchors.verticalCenter: parent.verticalCenter
            width: parent.width
            spacing: 8
            Text {
                Layout.fillWidth: true
                text: section.title
                color: Theme.foreground
                font.family: Theme.font
                font.pixelSize: 12
                font.weight: Font.Bold
            }
            Text {
                visible: section.hint !== ""
                text: section.hint
                color: Theme.mutedForeground
                font.family: Theme.font
                font.pixelSize: 12
            }
            Icon {
                name: "chevron-down"
                size: 14
                color: Theme.mutedForeground
                rotation: section.open ? 0 : -90
                Behavior on rotation {
                    NumberAnimation {
                        duration: Theme.duration
                        easing.type: Easing.OutCubic
                    }
                }
            }
        }
        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: section.open = !section.open
        }
    }

    ColumnLayout {
        id: body
        visible: section.open
        Layout.fillWidth: true
        Layout.bottomMargin: 6
        spacing: 8
    }
}
