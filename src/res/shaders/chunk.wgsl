screen_info: $0;
t_atlas: $1,0;
s_atlas: $1,1;
lighting_data: $2;
chunk_position: $3;
skylight_data: $4;

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
    out.lighting = model.position;
    // out.lighting = vec4f(block_position(model.position.xyz, model.normal.xyz, model.color.xy)/64.0, 1.0);
    // out.lighting = vec4f(vec3f(get_color(vec3u(block_position(model.position.xyz, model.normal.xyz, model.color.xy))))/15.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> DeferredFragmentOutput {
    var output = textureSample(t_atlas, s_atlas, fix_repeats(in.color));
    if output.w == 0.0 {
        discard;
    }
    var out: DeferredFragmentOutput;
    let block_position = vec3u(block_position(in.lighting.xyz, in.normal.xyz, in.color.xy));
    let block_light = vec3f(get_lighting(block_position))/15.0;
    let skylight = f32(get_skylight(block_position))/15.0;
    let light = vec3f(max(block_light.x, skylight), max(block_light.y, skylight), max(block_light.z, skylight));
    out.color = vec4f(output.xyz * light, output.w*in.transparency);
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

const COLORS_PER_BYTE: u32 = 2u;
const COLORS_PER_BLOCK: u32 = 3u;
const CHUNK_SIZE: u32 = 32;

fn index_in_chunk(x: u32, y: u32, z: u32) -> u32 {
    return y * CHUNK_SIZE * CHUNK_SIZE + x * CHUNK_SIZE + z;
}

// fn get_byte(byte_index: u32) -> u32 {
//     let word = lighting_data[byte_index >> 2u];
//     return (word >> ((byte_index & 3u) * 8u)) & 0xFFu;
// }

fn get_lighting_byte(byte_index: u32) -> u32 {
    let word_index = byte_index / 4u;
    let byte_offset = (byte_index % 4u) * 8u;
    let word = lighting_data[word_index];
    
    return (word >> byte_offset) & 0xffu;
}

fn get_skylight_byte(byte_index: u32) -> u32 {
    let word_index = byte_index / 4u;
    let byte_offset = (byte_index % 4u) * 8u;
    let word = skylight_data[word_index];
    
    return (word >> byte_offset) & 0xffu;
}

fn get_lighting(block_pos: vec3u) -> vec3<u32> {
    let x = block_pos.x;
    let y = block_pos.y;
    let z = block_pos.z;
    let block_index = index_in_chunk(x, y, z);
    let light_index = block_index * COLORS_PER_BLOCK / COLORS_PER_BYTE;
    let light_remainder = block_index * COLORS_PER_BLOCK % COLORS_PER_BYTE;

    let b0 = get_lighting_byte(light_index);
    let b1 = get_lighting_byte(light_index + 1u);

    if light_remainder == 0u {
        return vec3<u32>(b0 >> 4u, b0 & 0x0Fu, b1 >> 4u);
    } else {
        return vec3<u32>(b0 & 0x0Fu, b1 >> 4u, b1 & 0x0Fu);
    }
}

fn get_skylight(block_pos: vec3u) -> u32 {
    let x = block_pos.x;
    let y = block_pos.y;
    let z = block_pos.z;
    let block_index = index_in_chunk(x, y, z);
    let light_index = block_index / COLORS_PER_BYTE;
    let light_remainder = block_index % COLORS_PER_BYTE;

    let b0 = get_skylight_byte(light_index);

    if light_remainder == 0u {
        return b0 >> 4u;
    } else {
        return b0 & 0x0Fu;
    }
}

fn block_position(vertex_position: vec3f, normal: vec3f, tex_coords: vec2f) -> vec3f {
    let tex_x_scaled = tex_coords.x*f32(ATLAS_X_BLOCKS);
    let tex_x_scaled_fract = fract(tex_x_scaled);
    let tex_y_scaled = tex_coords.y*f32(ATLAS_Y_BLOCKS);
    let tex_y_scaled_fract = fract(tex_y_scaled);
    let normal_offset = floor((normal+vec3f(1.0))/2.0);
    var tex_offset = vec3f(1.0);
    if all(normal == vec3f(-1.0, 0.0, 0.0)) {
        tex_offset = vec3f(0.0, tex_x_scaled_fract, tex_y_scaled_fract);
    } else if all(normal == vec3f(1.0, 0.0, 0.0)) {
        tex_offset = vec3f(0.0, tex_x_scaled_fract, tex_y_scaled_fract);
    } else if all(normal == vec3f(0.0, -1.0, 0.0)) {
        tex_offset = vec3f(tex_y_scaled_fract, 0.0, tex_x_scaled_fract);
    } else if all(normal == vec3f(0.0, 1.0, 0.0)) {
        tex_offset = vec3f(-tex_y_scaled_fract, 0.0, tex_x_scaled_fract);
    } else if all(normal == vec3f(0.0, 0.0, -1.0)) {
        tex_offset = vec3f(tex_x_scaled_fract, tex_y_scaled_fract, 0.0);
    } else { // 0.0, 0.0, 1.0
        tex_offset = vec3f(tex_x_scaled_fract, tex_y_scaled_fract, 0.0);
    }
    let chunk_offset = chunk_position * f32(CHUNK_SIZE);
    // return tex_offset;
    return vertex_position - normal_offset - chunk_offset;
}