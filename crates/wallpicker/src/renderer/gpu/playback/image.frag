precision highp float;
varying vec2 texcoord;
uniform sampler2D image;
uniform vec2 resolution;
uniform vec2 source_size;
uniform vec4 source_rect;
uniform vec4 destination_rect;
uniform vec2 flip;
uniform bool tiled;

vec3 sample_black(vec2 pixel) {
    vec4 color = texture2D(image, (pixel + 0.5) / source_size);
    return color.rgb * color.a;
}

vec4 picture(vec2 uv) {
    vec2 pixel = uv * resolution;
    vec2 local = (pixel - destination_rect.xy) / destination_rect.zw;
    if (any(lessThan(local, vec2(0.0))) || any(greaterThanEqual(local, vec2(1.0)))) {
        return vec4(0.0, 0.0, 0.0, 1.0);
    }
    if (tiled) local = fract(pixel / source_rect.zw);
    local = mix(local, 1.0 - local, flip);
    vec2 inset = min(vec2(0.5), source_rect.zw * 0.5);
    vec2 position = clamp(source_rect.xy + local * source_rect.zw,
        source_rect.xy + inset, source_rect.xy + source_rect.zw - inset) - 0.5;
    vec2 cell = floor(position);
    vec2 fraction = fract(position);
    // Interpolate AFTER premultiplication: transparent RGB cannot produce halos.
    vec3 top = mix(sample_black(cell), sample_black(cell + vec2(1.0, 0.0)), fraction.x);
    vec3 bottom = mix(sample_black(cell + vec2(0.0, 1.0)), sample_black(cell + vec2(1.0, 1.0)), fraction.x);
    return vec4(mix(top, bottom, fraction.y), 1.0);
}
