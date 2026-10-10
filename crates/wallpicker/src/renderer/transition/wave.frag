vec4 effect(vec2 uv) {
    float edge = progress + 0.06 * sin(uv.y * 25.132741 + progress * 6.283185) * sin(progress * 3.141593);
    float feather = 1.0 / resolution.x;
    float coverage = 1.0 - smoothstep(edge - feather, edge + feather, uv.x);
    return texture2D(image, uv) * coverage;
}
