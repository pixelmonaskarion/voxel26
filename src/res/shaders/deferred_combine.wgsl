a_t_color: $0,0;
a_s_color: $0,1;
a_t_normal: $0,2;
a_s_normal: $0,3;
a_t_worldspace: $0,4;
a_s_worldspace: $0,5;
a_t_depth: $0,6;
a_s_depth: $0,7;

b_t_color: $1,0;
b_s_color: $1,1;
b_t_normal: $1,2;
b_s_normal: $1,3;
b_t_worldspace: $1,4;
b_s_worldspace: $1,5;
b_t_depth: $1,6;
b_s_depth: $1,7;

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

struct FragmentOutput {
  @location(0) color: vec4f,
  @location(1) normal: vec4f,
  @location(2) worldspace: vec4f,
  @builtin(frag_depth) depth: f32,
}

@fragment
fn fs_main(in: VertexOutput, @builtin(sample_index) sample_index: u32) -> FragmentOutput {
    let tex_coords = vec2i(i32(screen_info.screen_size.x*in.tex_coords.x), i32(screen_info.screen_size.y*in.tex_coords.y));
    var out: FragmentOutput;
    
    let a_depth = textureLoad(a_t_depth, tex_coords, sample_index);
    let b_depth = textureLoad(b_t_depth, tex_coords, sample_index);
    if a_depth < b_depth {
        out.color = mix_colors(textureLoad(a_t_color, tex_coords, sample_index), textureLoad(b_t_color, tex_coords, sample_index));
        out.normal = textureLoad(a_t_normal, tex_coords, sample_index);
        out.worldspace = textureLoad(a_t_worldspace, tex_coords, sample_index);
        out.depth = a_depth;
    } else {
        out.color = mix_colors(textureLoad(b_t_color, tex_coords, sample_index), textureLoad(a_t_color, tex_coords, sample_index));
        out.normal = textureLoad(b_t_normal, tex_coords, sample_index);
        out.worldspace = textureLoad(b_t_worldspace, tex_coords, sample_index);
        out.depth = b_depth;
    }
    return out;
}