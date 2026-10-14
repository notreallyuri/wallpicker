// A switch on a row (the app's horizontal Field): the label and an
// optional muted note at the left, the switch at the right. The label
// toggles it too. In a Group, its rows have a hairline between them.
import QtQuick
import QtQuick.Layouts
import qs.services

ColumnLayout {
    id: field

    property string label
    property string description
    property bool checked: false
    signal toggled
    readonly property bool grouped: !!parent && parent.grouped === true
    readonly property bool first: grouped && parent.visibleChildren.length > 0 && parent.visibleChildren[0] === field

    spacing: 6

    Rectangle {
        visible: field.grouped && !field.first
        Layout.fillWidth: true
        Layout.bottomMargin: 4
        implicitHeight: 1
        color: Theme.a(Theme.foreground, 0.06)
    }
    Item {
        visible: field.grouped
        implicitHeight: field.first ? 8 : 2
    }
    RowLayout {
        Layout.fillWidth: true
        spacing: 16
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2
            Text {
                Layout.fillWidth: true
                text: field.label
                wrapMode: Text.Wrap
                color: Theme.foreground
                font.family: Theme.font
                font.pixelSize: 14
                font.weight: Font.Bold
                MouseArea {
                    anchors.fill: parent
                    enabled: field.enabled
                    cursorShape: Qt.PointingHandCursor
                    onClicked: field.toggled()
                }
            }
            Text {
                visible: field.description !== ""
                Layout.fillWidth: true
                text: field.description
                wrapMode: Text.Wrap
                color: Theme.mutedForeground
                font.family: Theme.font
                font.pixelSize: 12
            }
        }
        Toggle {
            Layout.alignment: Qt.AlignVCenter
            checked: field.checked
            enabled: field.enabled
            onToggled: field.toggled()
        }
    }
    Item {
        visible: field.grouped
        implicitHeight: 8
    }
}
