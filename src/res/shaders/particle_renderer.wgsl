screen_info: $0;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4f,
    @location(2) worldspace: vec4f,
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

    var world_position = model_matrix[3];
    world_position.w = 1.0;
    let scale = vec3(
        length(model_matrix[0].xyz),
        length(model_matrix[1].xyz),
        length(model_matrix[2].xyz)
    );

    let view_center = screen_info.camera.view * world_position;

    let final_view_pos = vec4(
        view_center.xyz + (model.position * scale),
        1.0
    );

    var out: VertexOutput;
    out.clip_position = screen_info.camera.proj * final_view_pos;
    out.tex_coords = model.tex_coords;
    out.color = instance.color;
    out.worldspace = final_view_pos;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> DeferredFragmentOutput {
    // if distance(in.tex_coords, vec2f(0.5, 0.5)) > 0.5 {
    //     discard;
    // }
    // return vec4f(in.color.xyz/100.0, 1.0);
    var out: DeferredFragmentOutput;
    out.color = in.color;
    out.normal = vec4f(0.0);
    out.worldspace = in.worldspace;
    return out;
}