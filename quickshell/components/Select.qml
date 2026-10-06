// A button showing the chosen option; a click lists them in a popover
// (Local's sort). `options`: [[value, label], ...].
import QtQuick
import QtQuick.Layouts
import qs.services

Btn {
    id: select

    property var options: []
    property string value
    signal picked(string value)

    readonly property string label: {
        const o = options.find(o => o[0] === value);
        return o ? o[1] : "";
    }

    text: label
    onClicked: list.open()

    Icon {
        name: "chevron-down"
        color: Theme.mutedForeground
    }

    Popover {
        id: list
        // Under the button (not the row inside it that holds the icon).
        parent: select
        y: select.height + 6
        width: Math.max(select.width, 180)

        contentItem: ColumnLayout {
            spacing: 2
            Repeater {
                model: select.options
                Rectangle {
                    id: option
                    required property var modelData
                    Layout.fillWidth: true
                    implicitHeight: 34
                    radius: 10
                    color: optionMouse.containsMouse ? Theme.a(Theme.primary, 0.12) : "transparent"
                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 10
                        anchors.rightMargin: 10
                        Text {
                            Layout.fillWidth: true
                            text: option.modelData[1]
                            color: Theme.foreground
                            font.family: Theme.font
                            font.pixelSize: 14
                            font.weight: option.modelData[0] === select.value ? Font.Bold : Font.Normal
                        }
                        Icon {
                            visible: option.modelData[0] === select.value
                            name: "check"
                            color: Theme.primary
                        }
                    }
                    MouseArea {
                        id: optionMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            select.picked(option.modelData[0]);
                            list.close();
                        }
                    }
                }
            }
        }
    }
}
