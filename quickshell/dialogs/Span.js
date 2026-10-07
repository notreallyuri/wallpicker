// The monitors as a span sees them (core's span.rs): each at its place in
// Hyprland's layout in logical pixels (its pixels / its scale, turned
// ones tall), pushed apart by the bezel gap once for each monitor wholly
// to its left (above).
.pragma library

/// {width, height, monitors: [{name, x, y, w, h}]}, from the top left of
/// the layout.
function layout(list, gap) {
    const places = list.map(m => {
        const turned = (m.transform || 0) % 2 === 1;
        const s = m.scale > 0 ? m.scale : 1;
        return {
            name: m.name,
            x: m.x || 0,
            y: m.y || 0,
            w: (turned ? m.height : m.width) / s,
            h: (turned ? m.width : m.height) / s
        };
    });
    const moved = places.map(p => ({
                name: p.name,
                x: p.x + places.filter(o => o.x + o.w <= p.x + 0.5).length * gap,
                y: p.y + places.filter(o => o.y + o.h <= p.y + 0.5).length * gap,
                w: p.w,
                h: p.h
            }));
    if (moved.length === 0)
        return {
            width: 16,
            height: 9,
            monitors: []
        };
    const minX = Math.min(...moved.map(p => p.x));
    const minY = Math.min(...moved.map(p => p.y));
    return {
        width: Math.max(...moved.map(p => p.x + p.w)) - minX,
        height: Math.max(...moved.map(p => p.y + p.h)) - minY,
        monitors: moved.map(p => Object.assign(p, {
                x: p.x - minX,
                y: p.y - minY
            }))
    };
}

/// A look as it comes: Cover over the layout, not mirrored.
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
