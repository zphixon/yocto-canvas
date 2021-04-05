use crate::{Context, Result};

use image::{DynamicImage, GenericImageView, ImageBuffer, ImageFormat, RgbaImage};

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Device, Extent3d, Origin3d, Queue, Sampler,
    SamplerDescriptor, ShaderStage, Texture, TextureCopyView, TextureDataLayout, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsage, TextureView,
    TextureViewDescriptor, TextureViewDimension,
};

pub struct MyTexture {
    pub texture: Texture,
    pub view: TextureView,
    pub sampler: Sampler,
    pub layout: BindGroupLayout,
    pub group: BindGroup,
}

#[allow(dead_code)]
impl MyTexture {
    pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

    pub fn from_bytes_with_format(
        device: &Device,
        queue: &Queue,
        bytes: &[u8],
        label: &str,
    ) -> Result<Self> {
        let img = image::load_from_memory(bytes).context("Couldn't load image from memory")?;
        Self::from_image(device, queue, &img, label)
    }

    pub fn empty(device: &Device, queue: &Queue, label: &str) -> Result<Self> {
        let width = 800;
        let height = 675;

        let data = vec![0xffu8; width as usize * height as usize * 4];
        let image: RgbaImage = image::ImageBuffer::from_vec(width, height, data).unwrap();

        Self::from_image(device, queue, &DynamicImage::ImageRgba8(image), label)
    }

    pub fn from_image(
        device: &Device,
        queue: &Queue,
        image: &DynamicImage,
        label: &str,
    ) -> Result<Self> {
        let rgba = image.to_rgba8();
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
            &rgba,
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

        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some(&format!("{} layout", label)),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStage::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStage::FRAGMENT,
                    ty: BindingType::Sampler {
                        filtering: false,
                        comparison: false,
                    },
                    count: None,
                },
            ],
        });

        let group = device.create_bind_group(&BindGroupDescriptor {
            label: Some(&format!("{} group", label)),
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        Ok(Self {
            texture,
            view,
            sampler,
            layout,
            group,
        })
    }

    pub fn load(device: &Device, queue: &Queue, path: impl AsRef<std::path::Path>) -> Result<Self> {
        let path_copy = path.as_ref().to_path_buf();
        let label = path_copy.to_str().unwrap();
        let image = image::open(path).context("Couldn't find image")?;
        Self::from_image(device, queue, &image, label)
    }
}
