uniform int effect_kind;

void main() {
    // Source rows are top-first. Effect coordinates follow the same convention.
    vec2 uv = vec2(texcoord.x, 1.0 - texcoord.y);
    if (progress <= 0.0) { gl_FragColor = vec4(0.0); return; }
    if (progress >= 1.0) { gl_FragColor = picture(uv); return; }
    if (effect_kind == 1) gl_FragColor = draw_wipe(uv);
    else if (effect_kind == 2) gl_FragColor = draw_slide(uv);
    else if (effect_kind == 3) gl_FragColor = draw_grow(uv);
    else if (effect_kind == 4) gl_FragColor = draw_shrink(uv);
    else if (effect_kind == 5) gl_FragColor = draw_wave(uv);
    else gl_FragColor = picture(uv) * progress;
}
