// The app's Input: a rounded field, an optional icon, a placeholder, and the
// primary ring on focus.
import QtQuick
import qs.services

Rectangle {
    id: field

    property alias text: input.text
    property string placeholder
    property string icon
    property alias validator: input.validator
    property alias echoMode: input.echoMode
    property int align: TextInput.AlignLeft
    /// What's typed can't be used (a red ring).
    property bool invalid: false
    signal accepted

    function forceFocus() {
        input.forceActiveFocus();
    }

    implicitWidth: 240
    implicitHeight: 36
    radius: height / 2
    color: Theme.a(Theme.input, 0.3)
    border.width: input.activeFocus || invalid ? 2 : 1
    border.color: invalid ? "#fd4663" : input.activeFocus ? Theme.a(Theme.primary, 0.7) : Theme.input
    Behavior on border.color {
        ColorAnimation {
            duration: 150
        }
    }

    Icon {
        id: lead
        visible: field.icon !== ""
        x: 10
        anchors.verticalCenter: parent.verticalCenter
        name: field.icon
        color: Theme.mutedForeground
    }
    TextInput {
        id: input
        x: lead.visible ? 34 : 12
        width: parent.width - x - 12
        anchors.verticalCenter: parent.verticalCenter
        horizontalAlignment: field.align
        color: Theme.foreground
        selectionColor: Theme.a(Theme.primary, 0.4)
        selectedTextColor: Theme.foreground
        font.family: Theme.font
        font.pixelSize: 14
        clip: true
        onAccepted: field.accepted()
        Text {
            visible: !input.text && !(input.activeFocus && field.align === TextInput.AlignHCenter)
            width: parent.width
            horizontalAlignment: field.align
            text: field.placeholder
            color: Theme.mutedForeground
            font: input.font
        }
    }
}
