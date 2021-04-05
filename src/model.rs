use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingResource, Buffer,
    BufferAddress, BufferDescriptor, BufferUsage, Device, IndexFormat, InputStepMode, Queue,
    RenderPass, VertexAttribute, VertexBufferLayout, VertexFormat,
};

use crate::{texture::MyTexture, Result};
use wgpu::util::{BufferInitDescriptor, DeviceExt};

pub trait Vertex {
    fn desc<'a>() -> VertexBufferLayout<'a>;
}

pub trait DrawModel<'a, 'b>
where
    'b: 'a,
{
    fn draw_mesh(&mut self, mesh: &'b Mesh, material: &'b Material, uniforms: &'b BindGroup);

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        uniforms: &'b BindGroup,
        instances: std::ops::Range<u32>,
    );

    fn draw_model(&mut self, model: &'b Model, uniforms: &'b BindGroup);

    fn draw_model_instanced(
        &mut self,
        model: &'b Model,
        uniforms: &'b BindGroup,
        instances: std::ops::Range<u32>,
    );
}

impl<'a, 'b> DrawModel<'a, 'b> for RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(&mut self, mesh: &'b Mesh, material: &'b Material, uniforms: &'b BindGroup) {
        self.draw_mesh_instanced(mesh, material, uniforms, 0..1);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        uniforms: &'b BindGroup,
        instances: std::ops::Range<u32>,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), IndexFormat::Uint32);

        self.set_bind_group(0, &material.bind_group, &[]);
        self.set_bind_group(1, &uniforms, &[]);

        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    fn draw_model(&mut self, model: &'b Model, uniforms: &'b BindGroup) {
        self.draw_model_instanced(model, uniforms, 0..1)
    }

    fn draw_model_instanced(
        &mut self,
        model: &'b Model,
        uniforms: &'b BindGroup,
        instances: std::ops::Range<u32>,
    ) {
        for mesh in &model.meshes {
            let material = &model.materials[mesh.material];
            self.draw_mesh_instanced(mesh, material, uniforms, instances.clone());
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    position: [f32; 3],
    normal: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vertex for ModelVertex {
    fn desc<'a>() -> VertexBufferLayout<'a> {
        use std::mem::size_of;
        VertexBufferLayout {
            array_stride: size_of::<ModelVertex>() as BufferAddress,
            step_mode: InputStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float3,
                },
                VertexAttribute {
                    offset: size_of::<[f32; 3]>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float3,
                },
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 6]>() as BufferAddress,
                    shader_location: 2,
                    format: VertexFormat::Float2,
                },
            ],
        }
    }
}

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

pub struct Material {
    pub name: String,
    pub diffuse_texture: MyTexture,
    pub bind_group: BindGroup,
}

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub num_elements: u32,
    pub material: usize,
}

impl Model {
    pub fn load(
        device: &Device,
        queue: &Queue,
        layout: &BindGroupLayout,
        path: impl AsRef<std::path::Path>,
    ) -> Result<Self> {
        let (obj_meshes, obj_materials) = tobj::load_obj(path.as_ref(), true)?;
        let mut meshes = Vec::new();
        let mut materials = Vec::new();

        let containing_folder = path.as_ref().parent().unwrap();

        for material in obj_materials {
            let diffuse_path = material.diffuse_texture;
            let diffuse_texture =
                MyTexture::load(device, queue, containing_folder.join(&diffuse_path))?;

            let bind_group = device.create_bind_group(&BindGroupDescriptor {
                label: Some(&diffuse_path),
                layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&diffuse_texture.view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&diffuse_texture.sampler),
                    },
                ],
            });

            materials.push(Material {
                name: material.name,
                diffuse_texture,
                bind_group,
            });
        }

        for mesh in obj_meshes {
            let mut vertices = Vec::new();

            for ((position, normal), tex_coords) in mesh
                .mesh
                .positions
                .chunks(3)
                .zip(mesh.mesh.normals.chunks(3))
                .zip(mesh.mesh.texcoords.chunks(2))
            {
                use std::convert::TryInto;
                println!("{:?} {:?} {:?}", position, normal, tex_coords);
                vertices.push(ModelVertex {
                    position: position.try_into().unwrap(),
                    normal: normal.try_into().unwrap(),
                    tex_coords: tex_coords.try_into().unwrap(),
                });
            }

            assert_eq!(mesh.mesh.positions.len() % 3, 0);
            assert_eq!(mesh.mesh.normals.len() % 3, 0);
            assert_eq!(mesh.mesh.texcoords.len() % 2, 0);

            assert_eq!(vertices.len(), mesh.mesh.positions.len() / 3);
            assert_eq!(vertices.len(), mesh.mesh.normals.len() / 3);
            assert_eq!(vertices.len(), mesh.mesh.texcoords.len() / 2);

            let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
                label: Some(&format!("{} vert buf", mesh.name)),
                contents: bytemuck::cast_slice(&vertices),
                usage: BufferUsage::VERTEX,
            });

            let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
                label: Some(&format!("{} index buf", mesh.name)),
                contents: bytemuck::cast_slice(&mesh.mesh.indices),
                usage: BufferUsage::INDEX,
            });

            meshes.push(Mesh {
                name: mesh.name,
                vertex_buffer,
                index_buffer,
                num_elements: mesh.mesh.indices.len() as u32,
                material: mesh.mesh.material_id.unwrap_or(0),
            });
        }

        Ok(Self { meshes, materials })
    }
}
