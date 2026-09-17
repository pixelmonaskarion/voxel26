@group(0) @binding(0)
var<storage,read_write> num_instances: atomic<u32>;
@group(1) @binding(0)
var<storage,read_write> indirect: DrawIndexedIndirectArgs;

struct DrawIndexedIndirectArgs {
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    base_vertex: i32,
    first_instance: u32,
}

@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    indirect.index_count = 6;
    indirect.instance_count = atomicLoad(&num_instances);
    indirect.first_index = 0;
    indirect.base_vertex = 0;
    indirect.first_instance = 0;
    atomicStore(&num_instances, 0u);
}