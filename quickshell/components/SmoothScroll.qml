// Smooth, generous mouse-wheel scrolling for any Flickable (a GridView,
// ListView...). Declare it anywhere with `flickable:`; it lays itself over
// that view (an invisible layer that only takes the wheel: clicks and hover
// go through), so it's first under the pointer: on the view's content, the
// view itself could take the wheel first, in Qt's small steps.
//
// A notch sets a target `step` px further and the view glides there
// (ease-out). Notches in quick succession speed up (a fast spin goes much
// further) and add to the target. Touchpads keep Qt's own scrolling (1:1,
// with momentum).
import QtQuick

Item {
    id: smooth

    required property Flickable flickable
    /// Pixels per notch (120 units of a wheel's angle), before speed-up.
    property real step: 220
    property int duration: 320

    // Where the glide is heading (null: not gliding), and the run of quick
    // notches for the speed-up.
    property var goal: null
    property real lastNotch: 0
    property int streak: 0

    readonly property real minY: flickable.originY - flickable.topMargin
    readonly property real maxY: Math.max(minY, flickable.originY + flickable.contentHeight + flickable.bottomMargin - flickable.height)

    parent: flickable
    anchors.fill: parent
    z: 1000

    WheelHandler {
        acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
        target: null
        onWheel: event => {
            if (event.device.type === PointerDevice.TouchPad) {
                event.accepted = false;
                return;
            }
            smooth.notches(-event.angleDelta.y / 120);
        }
    }

    /// Scroll by `n` notches (positive: down).
    function notches(n) {
        const now = Date.now();
        // Notches less than 150 ms apart are a spin: up to 3x the step.
        streak = now - lastNotch < 150 ? streak + 1 : 0;
        lastNotch = now;
        const speed = 1 + Math.min(streak, 8) * 0.25;
        const from = glide.running && goal !== null ? goal : flickable.contentY;
        goal = Math.max(minY, Math.min(maxY, from + n * step * speed));
        flickable.cancelFlick();
        glide.from = flickable.contentY;
        glide.to = goal;
        glide.restart();
    }

    NumberAnimation {
        id: glide
        target: smooth.flickable
        property: "contentY"
        duration: smooth.duration
        easing.type: Easing.OutCubic
        // Arrived (a restart for the next notch only stops it).
        onFinished: smooth.goal = null
    }
}
