vec4 effect(vec2 uv) {
    vec2 aspect = vec2(resolution.x / resolution.y, 1.0);
    vec2 farthest = max(direction, 1.0 - direction) * aspect;
    float radius = length(farthest) * progress;
    float distanceToOrigin = length((uv - direction) * aspect);
    float feather = 1.0 / resolution.y;
    float coverage = 1.0 - smoothstep(radius - feather, radius + feather, distanceToOrigin);
    return texture2D(image, uv) * coverage;
}
