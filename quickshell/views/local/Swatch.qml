// A color in the color picker: a round swatch and its name (stacked, or in
// a row when compact); ringed when chosen, lifting on hover.
import QtQuick
import QtQuick.Layouts
import qs.services

Rectangle {
    id: swatchItem

    property string key
    property string label
    property color swatch
    property bool on: false
    property bool compact: false
    signal picked

    implicitHeight: compact ? 32 : 58
    radius: 12
    color: on ? Theme.a(Theme.primary, 0.15) : (mouse.containsMouse ? Theme.a(Theme.primary, 0.1) : "transparent")
    border.width: on ? 2 : 0
    border.color: Theme.primary

    GridLayout {
        anchors.centerIn: parent
        anchors.horizontalCenterOffset: 0
        y: swatchItem.compact ? 0 : (mouse.containsMouse ? -2 : 0)
        flow: swatchItem.compact ? GridLayout.LeftToRight : GridLayout.TopToBottom
        columnSpacing: 8
        rowSpacing: 4
        Rectangle {
            Layout.alignment: Qt.AlignHCenter
            implicitWidth: swatchItem.compact ? 16 : 28
            implicitHeight: implicitWidth
            radius: implicitWidth / 2
            color: swatchItem.swatch
            border.width: 2
            border.color: Theme.a(Theme.foreground, 0.15)
        }
        Text {
            Layout.alignment: Qt.AlignHCenter
            text: swatchItem.label
            color: Theme.foreground
            font.family: Theme.font
            font.pixelSize: swatchItem.compact ? 12 : 11
            font.weight: Font.Bold
        }
    }
    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: swatchItem.picked()
    }
}
