screen_info: $0;
t_atlas: $1,0;
s_atlas: $1,1;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4f,
    @location(2) transparency: f32,
    @location(3) worldspace: vec4f,
    @location(4) lighting: vec4f,
}

@vertex
fn vs_main(
    model: VoxelVertex,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = screen_info.camera.view_proj * vec4<f32>(model.position.xyz, 1.0);
    out.normal = model.normal.xyz;
    out.color = model.color;
    out.transparency = model.position.w;
    out.worldspace = (screen_info.camera.view * vec4f(model.position.xyz, 1.0));
    out.lighting = model.lighting;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> DeferredFragmentOutput {
    var output = textureSample(t_atlas, s_atlas, fix_repeats(in.color));
    if output.w == 0.0 {
        discard;
    }
    var out: DeferredFragmentOutput;
    out.color = vec4f(output.xyz * in.lighting.xyz, output.w*in.transparency);
    out.normal = vec4f(in.normal, 1.0);
    out.worldspace = in.worldspace;
    return out;
}

fn fix_repeats(color: vec4f) -> vec2f {
    let x_scaled = color.x*f32(ATLAS_X_BLOCKS);
    let x_scaled_fract = fract(x_scaled);
    let x = (floor(x_scaled)+fract(x_scaled_fract*color.z))/f32(ATLAS_X_BLOCKS);
    let y_scaled = color.y*f32(ATLAS_Y_BLOCKS);
    let y_scaled_fract = fract(y_scaled);
    let y = (floor(y_scaled)+fract(y_scaled_fract*color.w))/f32(ATLAS_Y_BLOCKS);
    return vec2f(x, y);
}