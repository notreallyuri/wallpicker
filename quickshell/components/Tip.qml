// A tooltip (the app's Tip, nocomponents' tooltip): a dark rounded bubble
// with a short explanation, under (or over) what it's on, after a moment's
// hover; never the system kind. Only drawn: it takes no input at all (a
// tip over a button kept the button from being hovered and clicked), so
// what it's on says when the pointer is over it and when it's pressed:
//   Tip { text: "Settings"; over: mouse.containsMouse; pressed: mouse.pressed }
// Empty text shows nothing. Btn has it as `tip`; MonitorPill too.
import QtQuick
import QtQuick.Controls
import QtQuick.Window
import qs.services

Item {
    id: tip

    property string text
    /// "bottom" or "top".
    property string side: "bottom"
    /// The pointer is over what it's on.
    property bool over: false
    /// What it's on is pressed (a click, a drag): it closes.
    property bool pressed: false

    // Its size, to place the bubble by; nothing to take input.
    anchors.fill: parent

    onPressedChanged: if (pressed) {
        showing.stop();
        bubble.close();
    }
    Timer {
        id: showing
        interval: 400
        onTriggered: bubble.open()
    }
    Timer {
        id: hiding
        interval: 100
        onTriggered: bubble.close()
    }
    onOverChanged: {
        if (tip.over && tip.text !== "") {
            hiding.stop();
            if (!bubble.shown)
                showing.restart();
        } else {
            showing.stop();
            hiding.restart();
        }
    }

    // The bubble: a plain item in the window's overlay (above everything,
    // dialogs included), taking no input, so a click under it reaches what's
    // there (a Popup would swallow it). Placed when shown.
    Item {
        id: bubble

        property bool shown: false

        function open() {
            // Looked up now, not bound once: the window can be replaced.
            const layer = tip.Overlay.overlay || tip.Window.contentItem;
            if (!layer)
                return;
            parent = layer;
            const at = tip.mapToItem(layer, 0, 0);
            x = Math.max(8, Math.min(layer.width - width - 8, at.x + (tip.width - width) / 2));
            const below = at.y + tip.height + 6;
            const above = at.y - height - 6;
            // Its side, unless that's out of the window.
            y = tip.side === "top" ? (above >= 8 ? above : below) : (below + height <= layer.height - 8 ? below : above);
            shown = true;
        }
        function close() {
            shown = false;
        }

        z: 1000000
        visible: opacity > 0
        opacity: shown ? 1 : 0
        // Grows in (zoom-in-95).
        scale: shown ? 1 : 0.95
        width: Math.min(label.implicitWidth, 288)
        height: label.implicitHeight
        Behavior on opacity {
            NumberAnimation {
                duration: 100
            }
        }
        Behavior on scale {
            NumberAnimation {
                duration: 100
            }
        }

        Rectangle {
            anchors.fill: parent
            radius: 12
            color: Theme.foreground
        }
        Text {
            id: label
            width: parent.width
            leftPadding: 10
            rightPadding: 10
            topPadding: 4
            bottomPadding: 4
            text: tip.text
            // Long names break anywhere rather than overflow.
            wrapMode: Text.WrapAtWordBoundaryOrAnywhere
            color: Theme.background
            font.family: Theme.font
            font.pixelSize: 12
            font.weight: Font.DemiBold
        }
    }
}
