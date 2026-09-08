use bespoke_engine::{binding::Descriptor, model::ToRaw, InstanceTrait};
use bytemuck::bytes_of;
use glam::{Mat4, Quat, Vec3, vec3};

#[derive(Clone)]
pub struct Instance {
    pub position: Vec3,
    pub rotation: Quat,
}

impl InstanceTrait for Instance {
    fn instance_transform(&self) -> Mat4 {
        Mat4::from_translation(self.position) * Mat4::from_quat(self.rotation)
    }
}

impl Instance {
    pub fn raw(&self) -> InstanceRaw {
        InstanceRaw {model: self.instance_transform().to_cols_array_2d() }
    }
}

impl Default for Instance {
    fn default() -> Self {
        Self { position: vec3(0.0, 0.0, 0.0), rotation: Quat::from_axis_angle(Vec3::Z, 0.0) }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct InstanceRaw {
    model: [[f32; 4]; 4],
}

impl ToRaw for Instance {
    fn to_raw(&self) -> Vec<u8> {
        let raw = self.raw();
        bytes_of(&raw).to_vec()
    }
}

impl Descriptor for Instance {
    fn desc<'a>() -> Option<wgpu::VertexBufferLayout<'a>> {
        use std::mem;
        Some(wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
                // for each vec4. We'll have to reassemble the mat4 in the shader.
                wgpu::VertexAttribute {
                    offset: 0,
                    // While our vertex shader only uses locations 0, and 1 now, in later tutorials, we'll
                    // be using 2, 3, and 4, for Vertex. We'll start at slot 5, not conflict with them later
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        })
    }
}