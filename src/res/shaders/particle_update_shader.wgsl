in_particles: $0;
dst_particles: $1;
delta_seconds: $2;

@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    var particle = in_particles[global_id.z * num_workgroups.x * num_workgroups.y + global_id.y * num_workgroups.x + global_id.x];
    if particle.particle_type != 0 {
        if particle.particle_type == 1 {
            particle.lifetime -= delta_seconds;
            // particle.velocity.x *= 0.9;
            // particle.velocity.z *= 0.9;
            particle.velocity.y -= 20.0 * delta_seconds;
            particle.position += vec4f(particle.velocity.xyz, 0.0) * delta_seconds;
        }
        if particle.lifetime <= 0.0 {
            particle.particle_type = 0;
        } 
    }
    dst_particles[global_id.z * num_workgroups.x * num_workgroups.y + global_id.y * num_workgroups.x + global_id.x] = particle;
}