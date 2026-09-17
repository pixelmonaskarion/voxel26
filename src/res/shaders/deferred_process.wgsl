t_color: $0,0;
s_color: $0,1;
t_normal: $0,2;
s_normal: $0,3;
t_worldspace: $0,4;
s_worldspace: $0,5;
t_depth: $0,6;
s_depth: $0,7;
t_ssao: $1,0;
s_ssao: $1,1;
screen_info: $2;

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
fn fs_main(in: VertexOutput, @builtin(sample_index) sample_index: u32) -> @location(0) vec4<f32> {
    let tex_coords = vec2i(i32(screen_info.screen_size.x*in.tex_coords.x), i32(screen_info.screen_size.y*in.tex_coords.y));
    var color = textureLoad(t_color, tex_coords, sample_index);
    var normal = textureLoad(t_normal, tex_coords, sample_index);
    let ssao = textureSample(t_ssao, s_ssao, in.tex_coords);

    let lighting = lighting(normal.xyz);
    color = vec4f(color.xyz*ssao.r*lighting, color.w);
    return color;
}

fn lighting(normal: vec3f) -> f32 {
    if (normal.x == 0.0 && normal.y == 1.0 && normal.z == 0.0) {
        return 1.0;
    } else if (normal.x == 0.0 && normal.y == 0.0 && normal.z == 1.0) {
        return 0.8;
    } else if (normal.x == 0.0 && normal.y == 0.0 && normal.z == -1.0) { 
        return 0.8;
    } else if (normal.x == 1.0 && normal.y == 0.0 && normal.z == 0.0) {
        return 0.6;
    } else if ((normal.x == -1.0 && normal.y == 0.0 && normal.z == 0.0)) {
        return 0.6;
    } else {
        return 0.5;
    }
}