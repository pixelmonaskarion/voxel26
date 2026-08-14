t_ui: $0,0;
s_ui: $0,1;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) repeat_count: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) repeat_count: vec2<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(model.position, 1.0);
    out.tex_coords = model.tex_coords;
    out.repeat_count = model.repeat_count;
    return out;
}

fn fix_repeats(tex_coords: vec2f, repeat_count: vec2f) -> vec2f {
    let x_scaled = tex_coords.x*f32(ATLAS_X_BLOCKS);
    let x_scaled_fract = fract(x_scaled);
    let x = (floor(x_scaled)+fract(x_scaled_fract*repeat_count.x))/f32(ATLAS_X_BLOCKS);
    let y_scaled = tex_coords.y*f32(ATLAS_Y_BLOCKS);
    let y_scaled_fract = fract(y_scaled);
    let y = (floor(y_scaled)+fract(y_scaled_fract*repeat_count.y))/f32(ATLAS_Y_BLOCKS);
    return vec2f(x, y);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_ui, s_ui, fix_repeats(in.tex_coords, in.repeat_count));
}