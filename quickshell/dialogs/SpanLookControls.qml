// The Span dialog's framing controls: zoom and position sliders, mirroring.
import QtQuick
import QtQuick.Layouts
import qs.components
import qs.services

ColumnLayout {
    id: controls

    /// {zoom, x, y, flip: {h, v}}
    property var look
    /// A slider moved: (field, value).
    signal slid(string field, int value)
    /// Mirror on an axis ("h" or "v").
    signal mirrored(string axis)

    spacing: 8

    SliderRow {
        Layout.fillWidth: true
        label: "Zoom"
        from: 100
        to: 400
        value: controls.look.zoom
        onMoved: v => controls.slid("zoom", Math.round(v))
    }
    SliderRow {
        Layout.fillWidth: true
        label: "Left – right"
        to: 1000
        value: controls.look.x
        onMoved: v => controls.slid("x", Math.round(v))
    }
    SliderRow {
        Layout.fillWidth: true
        label: "Up – down"
        to: 1000
        value: controls.look.y
        onMoved: v => controls.slid("y", Math.round(v))
    }
    RowLayout {
        Layout.topMargin: 4
        spacing: 8
        Text {
            Layout.rightMargin: 4
            text: "Mirror"
            color: Theme.foreground
            font.family: Theme.font
            font.pixelSize: 14
            font.weight: Font.Bold
        }
        Btn {
            icon: "flip-horizontal"
            text: "Left to right"
            implicitHeight: 32
            active: controls.look.flip.h
            onClicked: controls.mirrored("h")
        }
        Btn {
            icon: "flip-vertical"
            text: "Top to bottom"
            implicitHeight: 32
            active: controls.look.flip.v
            onClicked: controls.mirrored("v")
        }
    }
}
