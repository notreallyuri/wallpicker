// The app's Button (ui/src/components/nc/button.rs): h-9, rounded, bold,
// and "chunky": a 3 px bottom shadow that grows a pixel on hover and
// presses down on click. Variants: outline (card), primary, ghost.
import QtQuick
import QtQuick.Layouts
import qs.services

Item {
    id: btn

    property string text
    property string icon
    property string variant: "outline"
    property real minWidth: 0
    property bool active: false
    property alias hovered: mouse.containsMouse
    default property alias extra: row.data
    signal clicked

    readonly property bool primary: variant === "primary" || active
    readonly property bool chunky: variant !== "ghost"
    readonly property color face: primary ? Theme.primary : variant === "ghost" ? (hovered ? Theme.a(Theme.primary, 0.12) : "transparent") : (hovered ? Theme.muted : Theme.a(Theme.card, 0.8))
    readonly property color ink: primary ? Theme.primaryForeground : Theme.foreground
    readonly property real press: mouse.pressed ? 2 : hovered ? -1 : 0

    implicitWidth: Math.max(minWidth, row.implicitWidth + (text ? 28 : 18))
    implicitHeight: 36

    // The chunky edge: a darker copy under the face.
    Rectangle {
        visible: btn.chunky
        width: parent.width
        height: parent.height
        y: 3 + (mouse.pressed ? -1 : btn.hovered ? 0 : 0)
        radius: 14
        color: btn.primary ? Qt.darker(Theme.primary, 1.8) : Theme.a(Theme.foreground, 0.16)
    }
    Rectangle {
        id: face
        width: parent.width
        height: parent.height
        y: btn.press
        radius: 14
        color: btn.face
        border.width: btn.primary || !btn.chunky ? 0 : 1
        border.color: Theme.input
        Behavior on y {
            NumberAnimation {
                duration: 120
                easing.type: Easing.OutCubic
            }
        }
        Behavior on color {
            ColorAnimation {
                duration: 150
            }
        }
        RowLayout {
            id: row
            anchors.centerIn: parent
            spacing: 6
            Icon {
                visible: btn.icon !== ""
                name: btn.icon
                color: btn.ink
            }
            Text {
                visible: btn.text !== ""
                text: btn.text
                color: btn.ink
                font.family: Theme.font
                font.pixelSize: 14
                font.weight: Font.Bold
            }
        }
    }
    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: btn.clicked()
    }
}
