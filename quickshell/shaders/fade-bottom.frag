// Fades an item out toward its bottom edge (FadeBottom.qml): fully drawn
// above `edge` (a fraction of its height from the bottom), gone at the
// edge itself, eased between. Compiled to .qsb with
//   /usr/lib/qt6/bin/qsb --glsl "100es,120,150" --hlsl 50 --msl 12 \
//     -o fade-bottom.frag.qsb fade-bottom.frag
#version 440
layout(location = 0) in vec2 qt_TexCoord0;
layout(location = 0) out vec4 fragColor;
layout(std140, binding = 0) uniform buf {
    mat4 qt_Matrix;
    float qt_Opacity;
    float edge;
};
layout(binding = 1) uniform sampler2D source;

void main() {
    float left = 1.0 - qt_TexCoord0.y;
    float keep = edge > 0.0 ? smoothstep(0.0, edge, left) : 1.0;
    // Premultiplied: scaling all four channels fades it.
    fragColor = texture(source, qt_TexCoord0) * keep * qt_Opacity;
}
