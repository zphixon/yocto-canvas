use crate::Result;

use image::{DynamicImage, GenericImageView};

use wgpu::{
    Device, Extent3d, Origin3d, Queue, Sampler, SamplerDescriptor, Texture, TextureCopyView,
    TextureDataLayout, TextureDescriptor, TextureDimension, TextureFormat, TextureUsage,
    TextureView, TextureViewDescriptor,
};

pub struct MyTexture {
    pub texture: Texture,
    pub view: TextureView,
    pub sampler: Sampler,
}

impl MyTexture {
    pub fn from_bytes(device: &Device, queue: &Queue, bytes: &[u8], label: &str) -> Result<Self> {
        let img = image::load_from_memory(bytes)?;
        Self::from_image(device, queue, &img, label)
    }

    pub fn from_image(
        device: &Device,
        queue: &Queue,
        image: &DynamicImage,
        label: &str,
    ) -> Result<Self> {
        println!("{}", &format!("{:?}", image)[..60]);
        let rgba = image.as_rgba8().unwrap();
        let dimensions = image.dimensions();

        let size = Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth: 1,
        };

        let texture = device.create_texture(&TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsage::SAMPLED | TextureUsage::COPY_DST,
        });

        queue.write_texture(
            TextureCopyView {
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
            },
            rgba,
            TextureDataLayout {
                offset: 0,
                bytes_per_row: 4 * dimensions.0,
                rows_per_image: dimensions.1,
            },
            size,
        );

        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
        })
    }
}
