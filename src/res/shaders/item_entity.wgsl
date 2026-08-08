screen_info: $0;
@group(1) @binding(0)
var<uniform> camera_view: mat4x4f;
@group(2) @binding(0)
var<uniform> camera_projection: mat4x4f;
t_atlas: $3,0;
s_atlas: $3,1;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct ItemInstance {
    @location(5) position: vec4f,
    @location(6) texture_offsets: vec4f,
    @location(7) atlas_subsection: vec4f,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) atlas_subsection: vec4f,
};

@vertex
fn vs_main(
    model: VertexInput,
    instance: ItemInstance,
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
    out.atlas_subsection = instance.atlas_subsection;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let subsection = in.atlas_subsection;
    let out = textureSample(t_atlas, s_atlas, vec2f(in.tex_coords.x*subsection.z+subsection.x, in.tex_coords.y*subsection.w+subsection.y));
    if out.w == 0.0 {
        discard;
    }
    return out;
}