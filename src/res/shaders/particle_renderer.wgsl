screen_info: $0;
@group(1) @binding(0)
var<uniform> camera_view: mat4x4f;
@group(2) @binding(0)
var<uniform> camera_projection: mat4x4f;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4f,
};

@vertex
fn vs_main(
    model: VertexInput,
    instance: ParticleInstance,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        vec4f(1.0, 0.0, 0.0, 0.0),
        vec4f(0.0, 1.0, 0.0, 0.0),
        vec4f(0.0, 0.0, 1.0, 0.0),
        instance.position,
    );

    // 1. Get the object's world position (translation) from the model matrix
    var world_position = model_matrix[3];
    world_position.w = 1.0;
    // 2. Get the object's scale from the model matrix
    // (This assumes no shearing in the matrix)
    let scale = vec3(
        length(model_matrix[0].xyz),
        length(model_matrix[1].xyz),
        length(model_matrix[2].xyz)
    );

    // 3. Transform the object's center into view space
    let view_center = camera_view * world_position;

    // 4. Apply the scaled local vertex position *directly in view space*.
    // This effectively ignores the model_matrix's rotation and aligns
    // the object's local X/Y axes with the camera's X/Y axes.
    let final_view_pos = vec4(
        view_center.xyz + (model.position * scale),
        1.0
    );

    // 5. Project the final view-space position to clip space
    var out: VertexOutput;
    out.clip_position = camera_projection * final_view_pos;
    out.tex_coords = model.tex_coords;
    out.color = instance.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // if distance(in.tex_coords, vec2f(0.5, 0.5)) > 0.5 {
    //     discard;
    // }
    // return vec4f(in.color.xyz/100.0, 1.0);
    // return vec4f(in.tex_coords, 0.0, 1.0);
    
    // let a = sample_2d_3d_texture(t_charge, s_charge, in.color.xyz/charge_field_size+vec3f(0.5), texture_dimensions_3d);
    // return a;
    // if a == 0.0 {
    //     discard;
    // }
    return in.color;
}