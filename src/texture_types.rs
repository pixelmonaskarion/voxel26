use bespoke_engine::{binding::{Binding, Resource}, shader::ShaderType, surface_context::SurfaceCtx, texture::{DepthTexture, DepthTextureLayoutConfig, Texture, TextureLayoutConfig}, window::MULTISAMPLE_COUNT};
use wgpu::TextureFormat;

pub struct TextureLayer {
    pub color: Texture,
    pub normal: Texture,
    pub worldspace: Texture,
    pub depth: DepthTexture,
    pub config: TextureLayerLayoutConfig,
}

impl TextureLayer {
    pub fn new(format: TextureFormat, surface_ctx: &dyn SurfaceCtx) -> Self {
        let color = Texture::blank_texture(surface_ctx.device(), surface_ctx.config().width, surface_ctx.config().height, surface_ctx.config().format, *MULTISAMPLE_COUNT.lock().unwrap(), None, None);
        let normal = Texture::blank_texture(surface_ctx.device(), surface_ctx.config().width, surface_ctx.config().height, format, *MULTISAMPLE_COUNT.lock().unwrap(), None, None);
        let worldspace = Texture::blank_texture(surface_ctx.device(), surface_ctx.config().width, surface_ctx.config().height, format, *MULTISAMPLE_COUNT.lock().unwrap(), None, None);
        let depth = DepthTexture::create_depth_texture(surface_ctx.device(), surface_ctx.config().width, surface_ctx.config().height, "Deferred", MULTISAMPLE_COUNT.lock().unwrap().clone());
        let config = TextureLayerLayoutConfig {
            color: color.config.clone(),
            normal: normal.config.clone(),
            worldspace: worldspace.config.clone(),
            depth: depth.config.clone(),
        };
        Self {
            color,
            normal,
            worldspace,
            depth,
            config,
        }
    }
}

pub struct TextureLayerLayoutConfig {
    pub color: TextureLayoutConfig,
    pub normal: TextureLayoutConfig,
    pub worldspace: TextureLayoutConfig,
    pub depth: DepthTextureLayoutConfig,
}

impl Binding for TextureLayer {
    type LayoutConfig = TextureLayerLayoutConfig;
    fn layout_config(&self) -> &Self::LayoutConfig {
        &self.config
    }
    
    fn layout(config: &TextureLayerLayoutConfig, ty: Option<wgpu::BindingType>) -> Vec<wgpu::BindGroupLayoutEntry> {
        [
            Texture::layout(&config.color, ty),
            Texture::layout(&config.normal, ty).into_iter().map(|mut it| { it.binding += 2; it }).collect(),
            Texture::layout(&config.worldspace, ty).into_iter().map(|mut it| { it.binding += 4; it }).collect(),
            DepthTexture::layout(&config.depth, ty).into_iter().map(|mut it| { it.binding += 6; it }).collect(),
        ].into_iter().flatten().collect()
    }

    fn create_resources<'a>(&'a self) -> Vec<bespoke_engine::binding::Resource<'a>> {
        vec![
            Resource::Bespoke(wgpu::BindingResource::TextureView(&self.color.view)),
            Resource::Bespoke(wgpu::BindingResource::Sampler(&self.color.sampler)),
            Resource::Bespoke(wgpu::BindingResource::TextureView(&self.normal.view)),
            Resource::Bespoke(wgpu::BindingResource::Sampler(&self.normal.sampler)),
            Resource::Bespoke(wgpu::BindingResource::TextureView(&self.worldspace.view)),
            Resource::Bespoke(wgpu::BindingResource::Sampler(&self.worldspace.sampler)),
            Resource::Bespoke(wgpu::BindingResource::TextureView(&self.depth.view)),
            Resource::Bespoke(wgpu::BindingResource::Sampler(&self.depth.sampler)),
        ]
    }

    fn shader_type(config: &TextureLayerLayoutConfig) -> ShaderType {
        let mut t_var_types = vec![];
        let mut t_wgsl_types = vec![];
        
        for ShaderType { mut var_types, mut wgsl_types } in [
            Texture::shader_type(&config.color),
            Texture::shader_type(&config.normal),
            Texture::shader_type(&config.worldspace),
            DepthTexture::shader_type(&config.depth),
        ] {
            t_var_types.append(&mut var_types);
            t_wgsl_types.extend_from_slice(&mut wgsl_types);
        }
        ShaderType { var_types: t_var_types, wgsl_types: t_wgsl_types }
    }
}