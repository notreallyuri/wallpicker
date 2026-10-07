// Smooth, generous mouse-wheel scrolling for any Flickable (a GridView,
// ListView...). Declare it anywhere with `flickable:`; it lays itself over
// that view (an invisible layer that only takes the wheel: clicks and hover
// go through), so it's first under the pointer: on the view's content, the
// view itself could take the wheel first, in Qt's small steps.
//
// A notch sets a target `step` px further and the view glides there
// (ease-out). Notches add to the target; a fast spin speeds up a little.
// Touchpads scroll 1:1.
//
// Wheel or touchpad is told by the event, not the device: on a compositor
// with pointer gestures (Hyprland), Qt registers only a touchpad device, so
// a mouse's wheel reports itself as a touchpad too. A touchpad's events
// have a scroll phase (begin, update, end); a wheel's have none.
//
// WALLPICKER_DEBUG_WHEEL=1 logs every wheel event and where it sends the
// view (qs log), for tuning against a real mouse.
import QtQuick
import Quickshell

Item {
    id: smooth

    required property Flickable flickable
    /// Pixels per notch (120 units of a wheel's angle), before speed-up.
    property real step: 200
    property int duration: 320

    // Where the glide is heading (null: not gliding), and the run of quick
    // notches for the speed-up.
    property var goal: null
    property real lastNotch: 0
    property real streak: 0

    readonly property bool debug: Quickshell.env("WALLPICKER_DEBUG_WHEEL") === "1"
    property real lastEvent: 0

    readonly property real minY: flickable.originY - flickable.topMargin
    readonly property real maxY: Math.max(minY, flickable.originY + flickable.contentHeight + flickable.bottomMargin - flickable.height)

    parent: flickable
    anchors.fill: parent
    z: 1000

    WheelHandler {
        // Both (see the top). Spelled out: a WheelHandler takes only Mouse
        // by default, so Hyprland's wheel events went past it to the view.
        acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
        target: null
        onWheel: event => {
            if (smooth.debug) {
                const now = Date.now();
                console.info("wheel:", smooth.lastEvent ? "+" + (now - smooth.lastEvent) + "ms" : "first", "device", event.device.type, "phase", event.phase, "angle", event.angleDelta.y, "pixel", event.pixelDelta.y, "inverted", event.inverted, "at", Math.round(smooth.flickable.contentY), "of", Math.round(smooth.minY) + ".." + Math.round(smooth.maxY));
                smooth.lastEvent = now;
            }
            if (event.phase !== Qt.NoScrollPhase || (event.angleDelta.y === 0 && event.pixelDelta.y !== 0))
                smooth.by(-(event.pixelDelta.y !== 0 ? event.pixelDelta.y : event.angleDelta.y / 4));
            else
                smooth.notches(-event.angleDelta.y / 120);
        }
    }

    /// Scroll by `px` at once (a touchpad's movement), stopping any glide.
    function by(px) {
        glide.stop();
        goal = null;
        flickable.cancelFlick();
        flickable.contentY = Math.max(minY, Math.min(maxY, flickable.contentY + px));
    }

    /// Scroll by `n` notches (positive: down).
    function notches(n) {
        if (n === 0)
            return;
        const now = Date.now();
        // Turning back starts over from where it is, at the normal speed.
        const gliding = glide.running && goal !== null;
        const back = gliding && (goal - flickable.contentY) * n < 0;
        // Only a real spin speeds up: notches under 50 ms apart (a mouse
        // measured 60-120 ms in ordinary scrolling, 20-45 ms spinning), up
        // to 2x. A high-resolution wheel sends a notch in pieces: they
        // count as their part of one.
        streak = now - lastNotch < 50 && !back ? streak + Math.abs(n) : 0;
        lastNotch = now;
        const speed = 1 + Math.min(streak, 6) * 0.15;
        const from = gliding && !back ? goal : flickable.contentY;
        goal = Math.max(minY, Math.min(maxY, from + n * step * speed));
        flickable.cancelFlick();
        glide.from = flickable.contentY;
        glide.to = goal;
        glide.restart();
        if (debug)
            console.info("wheel:   notches", n, "streak", streak.toFixed(2), "speed", speed.toFixed(2) + "x", "from", Math.round(from), "goal", Math.round(goal), back ? "(turned back)" : "");
    }

    NumberAnimation {
        id: glide
        target: smooth.flickable
        property: "contentY"
        duration: smooth.duration
        easing.type: Easing.OutCubic
        // Arrived (a restart for the next notch only stops it).
        onFinished: {
            if (smooth.debug)
                console.info("wheel:   arrived at", Math.round(smooth.flickable.contentY));
            smooth.goal = null;
        }
    }
}
