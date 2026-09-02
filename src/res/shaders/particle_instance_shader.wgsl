in_particles: $0;
dst_instances: $1;
@group(2) @binding(0)
var<storage,read_write> num_instances: atomic<u32>;

@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let particle = in_particles[global_id.z * num_workgroups.x * num_workgroups.y + global_id.y * num_workgroups.x + global_id.x];
    if particle.particle_type != 0 && particle.lifetime > 0.0 {
        var output_instance: ParticleInstance;
        output_instance.color = particle.color;
        output_instance.position = particle.position;
        let loaded_num_instances = atomicAdd(&num_instances, 1u);
        dst_instances[loaded_num_instances] = output_instance;
    }
}