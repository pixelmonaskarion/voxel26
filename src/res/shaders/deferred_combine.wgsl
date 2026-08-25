t_color: $0,0;
s_color: $0,1;
t_normal: $1,0;
s_normal: $1,1;
t_worldspace: $2,0;
s_worldspace: $2,1;
t_depth: $3,0;
s_depth: $3,1;
screen_info: $4;
kernel_samples: $5;
t_random: $6,0;

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
fn fs_main(in: VertexOutput, @builtin(sample_index) sample_index: u32) -> @location(0) vec4<f32> {
    let tex_coords = vec2i(i32(screen_info.screen_size.x*in.tex_coords.x), i32(screen_info.screen_size.y*in.tex_coords.y));
    var color = textureLoad(t_color, tex_coords, sample_index);
    color = vec4f(color.xyz/calculate_ssao(tex_coords, sample_index), color.w);
    return color;
    // return vec4f(1.0);
    // return vec4f(0.0);
    // return vec4f(vec3f(textureLoad(t_depth, tex_coords, sampleIndex)), 1.0);
}

fn calculate_ssao(tex_coords: vec2i, sample_index: u32) -> f32 {
    let frag_pos   = textureLoad(t_worldspace, tex_coords, sample_index).xyz;
    let normal    = textureLoad(t_normal, tex_coords, sample_index).rgb;
    let random_vec = textureLoad(t_random, vec2i(tex_coords.x % i32(textureDimensions(t_random).x), tex_coords.y % i32(textureDimensions(t_random).y)), 0).xyz;

    let tangent   = normalize(random_vec - normal * dot(random_vec, normal));
    let bitangent = cross(normal, tangent);
    let tbn = mat3x3f(tangent, bitangent, normal);

    var occlusion = 0.0;

    let kernel_size: u32 = 4;
    let radius = 0.5;
    let bias = 0.025;
    for(var i: u32 = 0; i < kernel_size; i += 1)
    {
        // get sample position
        var sample_pos = tbn * kernel_samples.samples[i].xyz;
        sample_pos = frag_pos + sample_pos * radius; 
        
        var offset = vec4(sample_pos, 1.0);
        offset      = screen_info.camera.proj * offset;    // from view to clip-space
        offset = vec4f(offset.xyz/offset.w, offset.w);               // perspective divide
        offset = vec4f(offset.xyz * 0.5 + 0.5, offset.w); // transform to range 0.0 - 1.0 

        let sample_depth = textureLoad(t_worldspace, vec2i(i32(offset.x * screen_info.screen_size.x), i32(offset.y * screen_info.screen_size.y)), sample_index).z; 
        if sample_depth >= sample_pos.z + bias {
            let range_check = smoothstep(0.0, 1.0, radius / abs(frag_pos.z - sample_depth));
            occlusion += 1.0;
        }
    }
    
    return 1.0 - (occlusion / f32(kernel_size));
}

fn pcg_hash3(p: vec3<u32>) -> vec3<u32> {
    var v = p * 1664525u + 1013904223u;
    v.x += v.y * v.z; v.y += v.z * v.x; v.z += v.x * v.y;
    v ^= v >> vec3<u32>(16u);
    v.x += v.y * v.z; v.y += v.z * v.x; v.z += v.x * v.y;
    return v;
}

fn random_vec3(seed: vec3<f32>) -> vec3<f32> {
    let hashed = pcg_hash3(vec3<u32>(bitcast<u32>(seed.x), bitcast<u32>(seed.y), bitcast<u32>(seed.z)));
    return vec3<f32>(hashed) / 4294967295.0;
}

fn pcg_hash(input: u32) -> u32 {
    let state = input * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn rand_pcg(seed: u32) -> f32 {
    let hash = pcg_hash(seed);
    return f32(hash) / f32(0xffffffffu);
}