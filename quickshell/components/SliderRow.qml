// A labelled slider (the Adjust dialog's Zoom, Left – right, Up – down): the
// label at the left (w-24, bold), a rounded track filled in the primary
// color up to a round handle. `moved(value)` as it's dragged.
import QtQuick
import QtQuick.Layouts
import QtQuick.Templates as T
import qs.services

RowLayout {
    id: row

    property string label
    property real from: 0
    property real to: 100
    property real stepSize: 5
    property real value: 0
    signal moved(real value)

    spacing: 12

    ColumnLayout {
        Layout.preferredWidth: 96
        Layout.minimumWidth: 96
        Layout.maximumWidth: 96
        spacing: 2

        Text {
            Layout.fillWidth: true
            text: row.label
            color: Theme.foreground
            font.family: Theme.font
            font.pixelSize: 14
            font.weight: Font.Bold
        }
        Text {
            Layout.fillWidth: true
            text: Math.round(slider.value)
            color: Theme.mutedForeground
            font.family: Theme.font
            font.pixelSize: 12
        }
    }

    T.Slider {
        id: slider
        Layout.fillWidth: true
        Layout.minimumWidth: 18
        implicitWidth: 200
        implicitHeight: 20
        from: row.from
        to: row.to
        stepSize: row.stepSize
        snapMode: T.Slider.SnapAlways
        value: row.value
        onMoved: row.moved(value)

        background: Rectangle {
            x: slider.leftPadding
            y: slider.topPadding + (slider.availableHeight - height) / 2
            width: slider.availableWidth
            height: 8
            radius: 4
            color: Theme.muted
            Rectangle {
                width: slider.visualPosition * parent.width
                height: parent.height
                radius: parent.radius
                color: Theme.primary
            }
        }
        handle: Rectangle {
            x: slider.leftPadding + slider.visualPosition * (slider.availableWidth - width)
            y: slider.topPadding + (slider.availableHeight - height) / 2
            width: 18
            height: 18
            radius: 9
            color: Theme.background
            border.width: 2
            border.color: Theme.primary
            scale: slider.pressed ? 1.15 : 1
            Behavior on scale {
                NumberAnimation {
                    duration: Theme.ms(120)
                }
            }
        }
        HoverHandler {
            cursorShape: Qt.PointingHandCursor
        }
    }
}
