vec4 effect(vec2 uv) {
    float feather = 1.0 / resolution.x;
    float coverage = 1.0 - smoothstep(progress - feather, progress + feather, uv.x);
    return picture(uv) * coverage;
}
