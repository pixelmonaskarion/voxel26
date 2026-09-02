t_screen: $0,0;
s_screen: $0,1;
radius: $1;
steps: $2;
axis: $3;
blur_kernel: $4;

struct BlurKernel {
    kernel: array<vec4f, 11>,
};

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(model.position, 1.0);
    out.tex_coords = model.tex_coords;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = vec4f(0.0);
    for (var i = 0; i < steps; i++) {
        let tex_coords = in.tex_coords + vec2f(radius.x*axis.x, radius.y*axis.y) * f32(i - (steps-1)/2);
        color += textureSample(t_screen, s_screen, tex_coords) * blur_kernel.kernel[i].x;
    }
    return color;
}