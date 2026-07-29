in_t: $0;
dst_b: $1;
@group(2) @binding(0)
var<storage,read_write> vertex_count: atomic<u32>;
offset: $3;

const CHUNK_SIZE: i32 = 32;

@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let position = vec3i(i32(global_id.x), i32(global_id.y), i32(global_id.z));
    let c = block_at_v(position);
    let cpx = block_at_v(position+vec3i(1, 0, 0));
    let cpy = block_at_v(position+vec3i(0, 1, 0));
    let cpz = block_at_v(position+vec3i(0, 0, 1));
    let cnx = block_at_v(position+vec3i(-1, 0, 0));
    let cny = block_at_v(position+vec3i(0, -1, 0));
    let cnz = block_at_v(position+vec3i(0, 0, -1));

    if is_solid(c) {
        if !is_solid(cpx) {
            let vertex_i = atomicAdd(&vertex_count, 4);
            dst_b[vertex_i+0] = VoxelVertex(
                vec4f(f32(global_id.x)+1.0, f32(global_id.y)+1.0, f32(global_id.z), 0.0)+offset, 
                c, 
                vec4f(1.0, 0.0, 0.0, 0.0));
            dst_b[vertex_i+1] = VoxelVertex(
                vec4f(f32(global_id.x)+1.0, f32(global_id.y)+1.0, f32(global_id.z)+1.0, 0.0)+offset, 
                c, 
                vec4f(1.0, 0.0, 0.0, 0.0));
            dst_b[vertex_i+2] = VoxelVertex(
                vec4f(f32(global_id.x)+1.0, f32(global_id.y), f32(global_id.z), 0.0)+offset, 
                c, 
                vec4f(1.0, 0.0, 0.0, 0.0));
            dst_b[vertex_i+3] = VoxelVertex(
                vec4f(f32(global_id.x)+1.0, f32(global_id.y), f32(global_id.z)+1.0, 0.0)+offset, 
                c, 
                vec4f(1.0, 0.0, 0.0, 0.0));
        }
        if !is_solid(cnx) {
            let vertex_i = atomicAdd(&vertex_count, 4);
            dst_b[vertex_i+0] = VoxelVertex(
                vec4f(f32(global_id.x), f32(global_id.y)+1.0, f32(global_id.z)+1.0, 0.0)+offset, 
                c,
                vec4f(-1.0, 0.0, 0.0, 0.0));
            dst_b[vertex_i+1] = VoxelVertex(
                vec4f(f32(global_id.x), f32(global_id.y)+1.0, f32(global_id.z), 0.0)+offset, 
                c,
                vec4f(-1.0, 0.0, 0.0, 0.0));
            dst_b[vertex_i+2] = VoxelVertex(
                vec4f(f32(global_id.x), f32(global_id.y), f32(global_id.z)+1.0, 0.0)+offset, 
                c,
                vec4f(-1.0, 0.0, 0.0, 0.0));
            dst_b[vertex_i+3] = VoxelVertex(
                vec4f(f32(global_id.x), f32(global_id.y), f32(global_id.z), 0.0)+offset, 
                c,
                vec4f(-1.0, 0.0, 0.0, 0.0));
        }
        if !is_solid(cpy) {
            let vertex_i = atomicAdd(&vertex_count, 4);
            dst_b[vertex_i+0] = VoxelVertex(
                vec4f(f32(global_id.x), f32(global_id.y)+1.0, f32(global_id.z)+1.0, 0.0)+offset, 
                c,
                vec4f(0.0, 1.0, 0.0, 0.0));
            dst_b[vertex_i+1] = VoxelVertex(
                vec4f(f32(global_id.x)+1.0, f32(global_id.y)+1.0, f32(global_id.z)+1.0, 0.0)+offset, 
                c,
                vec4f(0.0, 1.0, 0.0, 0.0));
            dst_b[vertex_i+2] = VoxelVertex(
                vec4f(f32(global_id.x), f32(global_id.y)+1.0, f32(global_id.z), 0.0)+offset, 
                c,
                vec4f(0.0, 1.0, 0.0, 0.0));
            dst_b[vertex_i+3] = VoxelVertex(
                vec4f(f32(global_id.x)+1.0, f32(global_id.y)+1.0, f32(global_id.z), 0.0)+offset, 
                c,
                vec4f(0.0, 1.0, 0.0, 0.0));
        }
        if !is_solid(cny) {
            let vertex_i = atomicAdd(&vertex_count, 4);
            dst_b[vertex_i+0] = VoxelVertex(
                vec4f(f32(global_id.x), f32(global_id.y), f32(global_id.z), 0.0)+offset, 
                c,
                vec4f(0.0, -1.0, 0.0, 0.0));
            dst_b[vertex_i+1] = VoxelVertex(
                vec4f(f32(global_id.x)+1.0, f32(global_id.y), f32(global_id.z), 0.0)+offset, 
                c,
                vec4f(0.0, -1.0, 0.0, 0.0));
            dst_b[vertex_i+2] = VoxelVertex(
                vec4f(f32(global_id.x), f32(global_id.y), f32(global_id.z)+1.0, 0.0)+offset, 
                c,
                vec4f(0.0, -1.0, 0.0, 0.0));
            dst_b[vertex_i+3] = VoxelVertex(
                vec4f(f32(global_id.x)+1.0, f32(global_id.y), f32(global_id.z)+1.0, 0.0)+offset, 
                c,
                vec4f(0.0, -1.0, 0.0, 0.0));
        }
        if !is_solid(cpz) {
            let vertex_i = atomicAdd(&vertex_count, 4);
            dst_b[vertex_i+0] = VoxelVertex(
                vec4f(f32(global_id.x)+1.0, f32(global_id.y)+1.0, f32(global_id.z)+1.0, 0.0)+offset, 
                c,
                vec4f(0.0, 0.0, 1.0, 0.0));
            dst_b[vertex_i+1] = VoxelVertex(
                vec4f(f32(global_id.x), f32(global_id.y)+1.0, f32(global_id.z)+1.0, 0.0)+offset, 
                c,
                vec4(0.0, 0.0, 1.0, 0.0));
            dst_b[vertex_i+2] = VoxelVertex(
                vec4f(f32(global_id.x)+1.0, f32(global_id.y), f32(global_id.z)+1.0, 0.0)+offset, 
                c,
                vec4f(0.0, 0.0, 1.0, 0.0));
            dst_b[vertex_i+3] = VoxelVertex(
                vec4f(f32(global_id.x), f32(global_id.y), f32(global_id.z)+1.0, 0.0)+offset, 
                c,
                vec4f(0.0, 0.0, 1.0, 0.0));
        }
        if !is_solid(cnz) {
            let vertex_i = atomicAdd(&vertex_count, 4);
            dst_b[vertex_i+0] = VoxelVertex(
                vec4f(f32(global_id.x), f32(global_id.y)+1.0, f32(global_id.z), 0.0)+offset, 
                c,
                vec4f(0.0, 0.0, -1.0, 0.0));
            dst_b[vertex_i+1] = VoxelVertex(
                vec4f(f32(global_id.x)+1.0, f32(global_id.y)+1.0, f32(global_id.z), 0.0)+offset, 
                c,
                vec4f(0.0, 0.0, -1.0, 0.0));
            dst_b[vertex_i+2] = VoxelVertex(
                vec4f(f32(global_id.x), f32(global_id.y), f32(global_id.z), 0.0)+offset, 
                c,
                vec4f(0.0, 0.0, -1.0, 0.0));
            dst_b[vertex_i+3] = VoxelVertex(
                vec4f(f32(global_id.x)+1.0, f32(global_id.y), f32(global_id.z), 0.0)+offset, 
                c,
                vec4f(0.0, 0.0, -1.0, 0.0));
        }
    }
}

fn block_at(x: i32, y: i32, z: i32) -> vec4f {
    let chunk_pos_f32 = vec3f(f32(x), f32(y), f32(z))/f32(CHUNK_SIZE);
    let chunk_pos = vec3i(i32(floor(chunk_pos_f32.x)), i32(floor(chunk_pos_f32.y)), i32(floor(chunk_pos_f32.z)));
    let in_chunk_pos = vec3u(
        u32(((x % CHUNK_SIZE) + CHUNK_SIZE) % CHUNK_SIZE),
        u32(((y % CHUNK_SIZE) + CHUNK_SIZE) % CHUNK_SIZE),
        u32(((z % CHUNK_SIZE) + CHUNK_SIZE) % CHUNK_SIZE),
    );
    if chunk_pos.x == 0 && chunk_pos.y == 0 && chunk_pos.z == 0 { 
        return textureLoad(in_t, in_chunk_pos);
    // } else if chunk_pos.x == 0 && chunk_pos.y == 1 && chunk_pos.z == 0 {
    //     return py_blocks[index];
    // } else if chunk_pos.x == 0 && chunk_pos.y == -1 && chunk_pos.z == 0 {
    //     return ny_blocks[index];
    // } else if chunk_pos.x == 1 && chunk_pos.y == 0 && chunk_pos.z == 0 {
    //     return px_blocks[index];
    // } else if chunk_pos.x == -1 && chunk_pos.y == 0 && chunk_pos.z == 0 {
    //     return nx_blocks[index];
    // } else if chunk_pos.x == 0 && chunk_pos.y == 0 && chunk_pos.z == 1 {
    //     return pz_blocks[index];
    // } else if chunk_pos.x == 0 && chunk_pos.y == 0 && chunk_pos.z == -1 {
    //     return nz_blocks[index];
    } else {
        return vec4f(0.0);
    }
}

fn block_at_v(v: vec3i) -> vec4f {
    return block_at(v.x, v.y, v.z);
}

fn is_solid(color: vec4f) -> bool {
    if color.w > 0.0 {
        return true;
    }
    return false;
}