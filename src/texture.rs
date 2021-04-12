use crate::{Context, Result};

use image::{DynamicImage, GenericImageView, RgbaImage};

use futures::AsyncReadExt;
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
    pub size: Extent3d,
    pub layout: TextureDataLayout,
    pub sampler: Sampler,
    pub group: BindGroup,
    pub group_layout: BindGroupLayout,
}

#[allow(dead_code)]
impl MyTexture {
    pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

    pub fn from_bytes_with_format(
        device: &Device,
        queue: &Queue,
        bytes: &[u8],
        label: &str,
    ) -> Result<(MyTexture, RgbaImage)> {
        let img = image::load_from_memory(bytes).context("Couldn't load image from memory")?;
        Self::from_image(device, queue, &img, label)
    }

    pub fn empty(device: &Device, queue: &Queue, label: &str) -> Result<(Self, RgbaImage)> {
        let width = 800;
        let height = 675;

        use std::iter::once;
        let data = vec![(0x0f, 0x0f, 0x0f, 0xff); width as usize * height as usize]
            .into_iter()
            .flat_map(|(r, g, b, a)| once(r).chain(once(g)).chain(once(b)).chain(once(a)))
            .collect::<Vec<u8>>();

        let image: RgbaImage = image::ImageBuffer::from_vec(width, height, data.clone()).unwrap();

        Self::from_image(
            device,
            queue,
            &DynamicImage::ImageRgba8(image.clone()),
            label,
        )
    }

    pub fn from_image(
        device: &Device,
        queue: &Queue,
        image: &DynamicImage,
        label: &str,
    ) -> Result<(Self, RgbaImage)> {
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

        let layout = TextureDataLayout {
            offset: 0,
            bytes_per_row: 4 * dimensions.0,
            rows_per_image: dimensions.1,
        };

        queue.write_texture(
            TextureCopyView {
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
            },
            &rgba,
            layout.clone(),
            size,
        );

        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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
            layout: &group_layout,
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

        Ok((
            Self {
                texture,
                view,
                size,
                layout,
                sampler,
                group,
                group_layout,
            },
            rgba,
        ))
    }

    pub fn load(
        device: &Device,
        queue: &Queue,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(MyTexture, RgbaImage)> {
        let path_copy = path.as_ref().to_path_buf();
        let label = path_copy.to_str().unwrap();
        let image = image::open(path).context("Couldn't find image")?;
        Self::from_image(device, queue, &image, label)
    }
}
