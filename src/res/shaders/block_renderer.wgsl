screen_info: $0;
t_atlas: $1,0;
s_atlas: $1,1;
screen_transform: $2;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4f,
    @location(2) transparency: f32,
}

@vertex
fn vs_main(
    model: VoxelVertex,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = screen_transform * screen_info.camera.view_proj * vec4<f32>(model.position.xyz, 1.0);
    out.normal = model.normal.xyz;
    out.color = model.color;
    out.transparency = model.position.w;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    //stretch
    //fract
    //multiply by z
    //fract
    //1.whole+3
    var output = textureSample(t_atlas, s_atlas, fix_repeats(in.color));
    if output.w == 0.0 {
        discard;
    }
    let lighting = lighting(in.normal);
    output = vec4f(output.xyz*lighting, output.w*in.transparency);
    return output;
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