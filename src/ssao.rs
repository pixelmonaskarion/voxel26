use bespoke_engine::{binding::WgslType, surface_context::SurfaceCtx, texture::Texture};
use bytemuck::{Pod, Zeroable};
use cgmath::{InnerSpace, vec3};
use image::{DynamicImage, Rgba};
use wgpu::TextureFormat;

#[repr(C)]
#[derive(Pod, Zeroable, Copy, Clone)]
pub struct SSAOKernelSamples {
    samples: [[f32; 4]; 64]
}

impl WgslType for SSAOKernelSamples {
    fn wgsl_name() -> String {
        "SSAOKernelSamples".into()
    }
}

pub fn generate_ssao_kernel_samples() -> SSAOKernelSamples {
    let mut samples = [[0.0; 4]; 64];
    for i in 0..64 {
        let mut sample = vec3(
            rand::random_range(0.0_f32..1.0) * 2.0 - 1.0,
            rand::random_range(0.0_f32..1.0) * 2.0 - 1.0,
            rand::random_range(0.0_f32..1.0),
        );
        sample = sample.normalize();
        sample *= rand::random_range(0.0_f32..1.0);
        samples[i] = sample.extend(0.0).into();
    }
    SSAOKernelSamples { samples }
}

pub fn generate_random_texture(surface_ctx: &dyn SurfaceCtx, width: u32, height: u32, format: TextureFormat) -> Texture {
    let mut image = image::Rgba32FImage::new(width, height);
    for x in 0..width {
        for y in 0..height {
            let rotation = vec3(
                rand::random_range(0.0_f32..1.0) * 2.0 - 1.0,
                rand::random_range(0.0_f32..1.0) * 2.0 - 1.0,
                0.0,
            );
            image.put_pixel(x, y, Rgba(rotation.extend(1.0).into()));
        }
    }
    let texture = Texture::from_image(surface_ctx.device(), surface_ctx.queue(), &DynamicImage::ImageRgba32F(image), Some("Random Texture"), Some(format), None, None, None).unwrap();
    texture
}