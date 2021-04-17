use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BlendState, Buffer, BufferBindingType, BufferUsage,
    ColorTargetState, ColorWrite, CommandEncoder, CullMode, Device, FragmentState, FrontFace,
    LoadOp, MultisampleState, Operations, Origin3d, PipelineLayoutDescriptor, PolygonMode,
    PrimitiveState, PrimitiveTopology, Queue, RenderPassColorAttachmentDescriptor,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderStage,
    SwapChainDescriptor, SwapChainTexture, TextureCopyView, VertexState,
};

use super::{Uniform, Vertex, VERTICES};

use crate::{image::Image, texture::MyTexture, Result};

pub struct CanvasPipeline {
    pub canvas_pipeline: RenderPipeline,
    pub canvas_texture: MyTexture,
    pub canvas_image: Image,
    pub canvas_uniform_buffer: Buffer,
    pub canvas_uniform_bind_group: BindGroup,
    pub quad_vertex_buffer: Buffer,
}

impl CanvasPipeline {
    pub fn execute(
        &self,
        encoder: &mut CommandEncoder,
        queue: &Queue,
        frame: &SwapChainTexture,
        width: f32,
        height: f32,
    ) {
        queue.write_texture(
            TextureCopyView {
                texture: &self.canvas_texture.texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
            },
            &self.canvas_image.as_raw(),
            self.canvas_texture.layout.clone(),
            self.canvas_texture.size.clone(),
        );

        {
            let mut rp = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("render pass"),
                color_attachments: &[RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });

            rp.set_viewport(0., 0., width, height, 0., 1.);

            rp.set_pipeline(&self.canvas_pipeline);

            rp.set_bind_group(0, &self.canvas_texture.group, &[]);
            rp.set_bind_group(1, &self.canvas_uniform_bind_group, &[]);

            rp.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));

            let len = VERTICES.len() as u32;
            rp.draw(0..len, 0..1);
        }
    }

    pub fn new(device: &Device, queue: &Queue, sc_desc: &SwapChainDescriptor) -> Result<Self> {
        let (canvas_texture, image) = MyTexture::load(device, queue, "res/4751549.png")?;
        //let (texture, image) = MyTexture::load(&device, &queue, "happy-tree.bdff8a19.png")?;

        let canvas_image = Image::from(image);

        let initial_uniform = Uniform {
            scale_x: 1.0,
            scale_y: 1.0,
            xform_x: 1.0,
            xform_y: 1.0,
            zoom: 1.0f32,
        };

        let canvas_uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("uniform"),
            contents: bytemuck::cast_slice(&[initial_uniform]),
            usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
        });

        let canvas_uniform_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("uniform bgl"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStage::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let canvas_uniform_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("uniform b group"),
            layout: &canvas_uniform_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: canvas_uniform_buffer.as_entire_binding(),
            }],
        });

        let canvas_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("pipeline layout"),
            bind_group_layouts: &[
                &canvas_texture.group_layout,
                &canvas_uniform_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let vs_module =
            device.create_shader_module(&wgpu::include_spirv!("../../shaders/shader.vert.spv"));
        let fs_module =
            device.create_shader_module(&wgpu::include_spirv!("../../shaders/shader.frag.spv"));

        let canvas_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Pipeline"),
            layout: Some(&canvas_pipeline_layout),
            vertex: VertexState {
                module: &vs_module,
                entry_point: "main",
                buffers: &[Vertex::desc()],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Cw,
                cull_mode: CullMode::None,
                polygon_mode: PolygonMode::Fill,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                module: &fs_module,
                entry_point: "main",
                targets: &[ColorTargetState {
                    format: sc_desc.format,
                    alpha_blend: BlendState::REPLACE,
                    color_blend: BlendState::REPLACE,
                    write_mask: ColorWrite::ALL,
                }],
            }),
        });

        let quad_vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("vertex buffer"),
            contents: bytemuck::cast_slice(&VERTICES),
            usage: BufferUsage::VERTEX,
        });

        Ok(Self {
            canvas_pipeline,
            canvas_texture,
            canvas_image,
            canvas_uniform_buffer,
            canvas_uniform_bind_group,
            quad_vertex_buffer,
        })
    }
}
