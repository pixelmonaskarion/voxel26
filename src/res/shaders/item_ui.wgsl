t_atlas: $0,0;
s_atlas: $0,1;
subsection: $1;

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
    // return textureSample(t_atlas, s_atlas, vec2f(in.tex_coords.x, in.tex_coords.y));
    // return textureSample(t_atlas, s_atlas, vec2f(in.tex_coords.x, in.tex_coords.y));
    return textureSample(t_atlas, s_atlas, vec2f(in.tex_coords.x*subsection.z+subsection.x, in.tex_coords.y*subsection.w+subsection.y));
}