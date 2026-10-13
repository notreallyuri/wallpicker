vec4 effect(vec2 uv) {
    vec2 sampleAt = uv + direction * (1.0 - progress);
    if (sampleAt.x < 0.0 || sampleAt.x > 1.0 || sampleAt.y < 0.0 || sampleAt.y > 1.0)
        return vec4(0.0);
    return picture(sampleAt);
}
