// A span on wallpicker's canvas (core's span.rs): monitors placed in
// millimetres ({name, x, y, w, h}, their top left and physical size), the
// picture over the ones spanned as their look says (zoom and center like a
// monitor's frame over them all, mirrored as a whole). Also fitting the
// canvas into a box, and snapping a dragged monitor to the others.
.pragma library
.import "Frame.js" as Frame

/// The box around `places`: {x, y, w, h} (mm).
function bounds(places) {
    if (!places || places.length === 0)
        return {
            x: 0,
            y: 0,
            w: 1,
            h: 1
        };
    const x = Math.min(...places.map(p => p.x));
    const y = Math.min(...places.map(p => p.y));
    return {
        x: x,
        y: y,
        w: Math.max(1, Math.max(...places.map(p => p.x + p.w)) - x),
        h: Math.max(1, Math.max(...places.map(p => p.y + p.h)) - y)
    };
}

/// A look as it comes: Cover over the monitors, not mirrored.
function initial() {
    return {
        zoom: 100,
        x: 500,
        y: 500,
        flip: {
            h: false,
            v: false
        }
    };
}

/// The look as a frame over `spanned`'s box (tenths of mm, for its
/// proportions), for Frame.js.
function frameOf(look, spanned) {
    const b = bounds(spanned);
    return {
        zoom: look.zoom,
        x: look.x,
        y: look.y,
        width: Math.max(1, Math.round(b.w * 10)),
        height: Math.max(1, Math.round(b.h * 10))
    };
}

/// Where a `nw`×`nh` picture goes, unmirrored, for `look` over `spanned`
/// drawn at `k` pixels per mm: {x, y, width, height} in pixels from the
/// spanned box's top left.
function picture(look, spanned, nw, nh, k) {
    const b = bounds(spanned);
    if (!(nw > 0 && nh > 0))
        return {
            x: 0,
            y: 0,
            width: b.w * k,
            height: b.h * k
        };
    const r = Frame.rect(frameOf(look, spanned), nw, nh);
    const s = b.w * k / r.width;
    return {
        x: -r.left * s,
        y: -r.top * s,
        width: nw * s,
        height: nh * s
    };
}

/// `places` fitted in `maxW`×`maxH` pixels with `pad` around: {k (pixels
/// per mm), x, y (the mm at the view's top left), width, height}.
function fit(places, maxW, maxH, pad) {
    const b = bounds(places);
    const k = Math.min((maxW - 2 * pad) / b.w, (maxH - 2 * pad) / b.h);
    return {
        k: k,
        x: b.x - pad / k,
        y: b.y - pad / k,
        width: b.w * k + 2 * pad,
        height: b.h * k + 2 * pad
    };
}

/// `p` (being dragged) snapped to `others`: against their edges `bezel` mm
/// apart, or lined up with their edges and centers, when within `reach`
/// mm; each axis on its own, so it can sit anywhere along an edge.
function snap(p, others, bezel, reach) {
    const xs = [], ys = [];
    for (const o of others) {
        xs.push(o.x + o.w + bezel, o.x - p.w - bezel, o.x, o.x + o.w - p.w, o.x + (o.w - p.w) / 2);
        ys.push(o.y + o.h + bezel, o.y - p.h - bezel, o.y, o.y + o.h - p.h, o.y + (o.h - p.h) / 2);
    }
    const nearest = (v, list) => {
        let best = v, gap = reach;
        for (const c of list) {
            if (Math.abs(c - v) < gap) {
                gap = Math.abs(c - v);
                best = c;
            }
        }
        return best;
    };
    return Object.assign({}, p, {
        x: nearest(p.x, xs),
        y: nearest(p.y, ys)
    });
}
