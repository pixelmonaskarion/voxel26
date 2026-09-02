t_normal: $0,0;
s_normal: $0,1;
t_worldspace: $1,0;
s_worldspace: $1,1;
screen_info: $2;
kernel_samples: $3;
t_random: $4,0;

struct SSAOKernelSamples {
    samples: array<vec4f, 64>,
}

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
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let tex_coords = vec2i(i32(screen_info.screen_size.x*in.tex_coords.x), i32(screen_info.screen_size.y*in.tex_coords.y));
    return calculate_ssao(tex_coords);
    // return vec4f(-textureLoad(t_normal, tex_coords, 0).xyz, 1.0);
}

fn calculate_ssao(tex_coords: vec2i) -> vec4f {
    let frag_pos   = textureLoad(t_worldspace, tex_coords, 0).xyz;
    var normal4    = textureLoad(t_normal, tex_coords, 0);
    // normal4 = screen_info.camera.proj * vec4f(normal4.xyz;
    // let normal = normal4.xyz/normal4.w;
    let view3x3 = mat3x3f(
        screen_info.camera.view[0].xyz,
        screen_info.camera.view[1].xyz,
        screen_info.camera.view[2].xyz
    );
    let normal = view3x3 * normal4.xyz;
    let random_vec = textureLoad(t_random, vec2i(tex_coords.x % i32(textureDimensions(t_random).x), tex_coords.y % i32(textureDimensions(t_random).y)), 0).xyz;

    let tangent   = normalize(random_vec - normal * dot(random_vec, normal));
    let bitangent = cross(normal, tangent);
    let tbn = mat3x3f(tangent, bitangent, normal);

    var occlusion = 0.0;

    let kernel_size: u32 = 16;
    let radius = 0.5;
    let bias = 0.05;
    for(var i: u32 = 0; i < kernel_size; i += 1)
    {
        // get sample position
        var sample_pos = tbn * kernel_samples.samples[i].xyz;
        sample_pos = frag_pos + sample_pos * radius; 
        
        var offset = vec4(sample_pos, 1.0);
        offset      = screen_info.camera.proj * offset;    // from view to clip-space
        offset = vec4f(offset.xyz/offset.w, offset.w);               // perspective divide
        offset = vec4f(offset.x * 0.5 + 0.5, offset.y * -0.5 + 0.5, offset.z * -0.5 + 0.5, offset.w); // transform to range 0.0 - 1.0 

        let sample_depth = textureLoad(t_worldspace, vec2i(i32(offset.x * screen_info.screen_size.x), i32(offset.y * screen_info.screen_size.y)), 0).z; 
        if sample_depth >= sample_pos.z + bias {
            let range_check = smoothstep(0.0, 1.0, radius / abs(frag_pos.z - sample_depth));
            occlusion += range_check;
        }
    }
    
    // return vec4f(normal.xyz, 1.0);
    return vec4f(1.0 - (occlusion / f32(kernel_size)), 0.0, 0.0, 1.0);
}