// Text recolored with a left-to-right gradient, keeping its own antialiased
// edges (its alpha). GradientText.qml; compiled to .qsb with
//   /usr/lib/qt6/bin/qsb --glsl "100es,120,150" --hlsl 50 --msl 12 \
//     -o gradient-text.frag.qsb gradient-text.frag
#version 440
layout(location = 0) in vec2 qt_TexCoord0;
layout(location = 0) out vec4 fragColor;
layout(std140, binding = 0) uniform buf {
    mat4 qt_Matrix;
    float qt_Opacity;
    vec4 from;
    vec4 to;
};
layout(binding = 1) uniform sampler2D source;

void main() {
    float alpha = texture(source, qt_TexCoord0).a;
    vec4 color = mix(from, to, qt_TexCoord0.x);
    // Premultiplied, as Qt Quick blends.
    fragColor = vec4(color.rgb * color.a, color.a) * alpha * qt_Opacity;
}
