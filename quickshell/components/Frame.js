// A monitor's frame on a picture (core's settings::Frame): the largest area
// in the monitor's proportions that fits in the picture (as Cover shows),
// made `zoom`% smaller around a center (`x`, `y`, ‰ of the picture's width
// and height), cut to the monitor's resolution (`width`×`height`).
.pragma library

/// The whole Cover area, centered, for a `w`×`h` monitor.
function initial(w, h) {
    return {
        zoom: 100,
        x: 500,
        y: 500,
        width: w,
        height: h
    };
}

/// The area cut from a `w`×`h` picture: {left, top, width, height} in its
/// pixels, kept inside it (as Frame::rect).
function rect(f, w, h) {
    w = Math.max(w, 1);
    h = Math.max(h, 1);
    const aspect = Math.max(f.width, 1) / Math.max(f.height, 1);
    let cw = w / h > aspect ? h * aspect : w;
    let ch = w / h > aspect ? h : w / aspect;
    const zoom = Math.min(Math.max(f.zoom, 100), 400) / 100;
    cw /= zoom;
    ch /= zoom;
    const cx = Math.min(f.x, 1000) / 1000 * w;
    const cy = Math.min(f.y, 1000) / 1000 * h;
    return {
        left: Math.round(Math.min(Math.max(cx - cw / 2, 0), w - cw)),
        top: Math.round(Math.min(Math.max(cy - ch / 2, 0), h - ch)),
        width: Math.max(Math.round(cw), 1),
        height: Math.max(Math.round(ch), 1)
    };
}

/// `f` moved by a drag of (dx, dy) on a screen `screenWidth` wide showing
/// it, on a `w`×`h` picture: the picture follows the pointer, so the frame
/// moves the other way (and the mirrored way when it's mirrored). Its
/// center is put back where the frame can have it (inside the picture).
function dragged(f, w, h, screenWidth, dx, dy, flip) {
    let r = rect(f, w, h);
    const s = screenWidth / Math.max(r.width, 1);
    if (flip.h)
        dx = -dx;
    if (flip.v)
        dy = -dy;
    const cx = r.left + r.width / 2 - dx / s;
    const cy = r.top + r.height / 2 - dy / s;
    const moved = Object.assign({}, f, {
        x: Math.round(Math.min(Math.max(cx / w * 1000, 0), 1000)),
        y: Math.round(Math.min(Math.max(cy / h * 1000, 0), 1000))
    });
    r = rect(moved, w, h);
    return Object.assign(moved, {
        x: Math.round((r.left + r.width / 2) / w * 1000),
        y: Math.round((r.top + r.height / 2) / h * 1000)
    });
}

/// `f` zoomed by `factor` (100 to 400%).
function zoomed(f, factor) {
    return Object.assign({}, f, {
        zoom: Math.trunc(Math.min(Math.max(f.zoom * factor, 100), 400))
    });
}
