// Smooth mouse-wheel scrolling for any Flickable (a GridView, ListView...):
// put it inside the view. A wheel notch sets a target `step` px further and
// the view glides there (ease-out); notches in a row add to the target, so a
// fast spin goes further without stutter. Touchpads send exact pixels and
// keep Qt's own scrolling (1:1, with momentum).
import QtQuick

WheelHandler {
    id: smooth

    /// The view to scroll. (Declared inside a Flickable, a handler lands on
    /// its contentItem: set this.)
    property Flickable flickable
    /// Pixels per notch (120 units of a wheel's angle).
    property real step: 120
    property int duration: 260

    // Where the glide is heading (null: not gliding).
    property var goal: null

    readonly property real top: flickable ? flickable.originY - flickable.topMargin : 0
    readonly property real bottom: flickable ? Math.max(top, flickable.originY + flickable.contentHeight + flickable.bottomMargin - flickable.height) : 0

    acceptedDevices: PointerDevice.Mouse
    target: null
    // Ours, not the view's: it would jump a step at once.
    onWheel: event => {
        if (event.pixelDelta.y !== 0) {
            event.accepted = false;
            return;
        }
        smooth.notches(-event.angleDelta.y / 120);
    }

    /// Scroll by `n` notches (positive: down).
    function notches(n) {
        const from = glide.running && goal !== null ? goal : flickable.contentY;
        goal = Math.max(top, Math.min(bottom, from + n * step));
        flickable.cancelFlick();
        glide.from = flickable.contentY;
        glide.to = goal;
        glide.restart();
    }

    property NumberAnimation glide: NumberAnimation {
        target: smooth.flickable
        property: "contentY"
        duration: smooth.duration
        easing.type: Easing.OutCubic
        // Arrived (a restart for the next notch only stops it).
        onFinished: smooth.goal = null
    }
}
