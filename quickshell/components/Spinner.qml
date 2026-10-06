// A turning arc.
import QtQuick
import qs.services

Canvas {
    id: spinner

    property color color: Theme.primary
    property real size: 22

    implicitWidth: size
    implicitHeight: size
    onColorChanged: requestPaint()
    onPaint: {
        const ctx = getContext("2d");
        ctx.reset();
        ctx.lineWidth = 2.5;
        ctx.lineCap = "round";
        ctx.strokeStyle = color;
        ctx.beginPath();
        ctx.arc(width / 2, height / 2, width / 2 - 2, 0, Math.PI * 1.5);
        ctx.stroke();
    }
    RotationAnimation on rotation {
        from: 0
        to: 360
        duration: 900
        loops: Animation.Infinite
        running: spinner.visible
    }
}
