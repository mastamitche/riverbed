use super::mesh_chunks::ATTRIBUTE_QUAD_SIZE;
use crate::{
    block::{Face, FaceSpecifier},
    world::CHUNK_S1,
    Block,
};
use bevy::{
    asset::{load_internal_asset, LoadedFolder},
    pbr::{ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline},
    prelude::*,
    reflect::TypePath,
    render::{
        mesh::MeshVertexBufferLayoutRef,
        render_asset::RenderAssetUsages,
        render_resource::{AsBindGroup, Extent3d, ShaderRef, TextureDimension, TextureFormat},
        storage::ShaderStorageBuffer,
    },
};
use dashmap::DashMap;
use std::sync::Arc;

const CHUNK_MATERIAL_SHADER: Handle<Shader> = Handle::weak_from_u128(102258915422227);

pub struct TextureArrayPlugin;

impl Plugin for TextureArrayPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, CHUNK_MATERIAL_SHADER, "chunk.wgsl", Shader::from_wgsl);
        app.insert_resource(TextureMap(Arc::new(DashMap::new())))
            .add_plugins(MaterialPlugin::<
                ExtendedMaterial<StandardMaterial, ArrayTextureMaterial>,
            >::default())
            .add_systems(Startup, build_tex_array);
    }
}

#[derive(Resource)]
pub struct TextureMap(pub Arc<DashMap<(Block, FaceSpecifier), usize>>);

pub trait TextureMapTrait {
    fn get_texture_index(&self, block: Block, face: Face) -> usize;
}

impl TextureMapTrait for &DashMap<(Block, FaceSpecifier), usize> {
    // TODO: need to allow the user to create a json with "texture files links" such as:
    // grass_block_bottom.png -> dirt.png
    // furnace_bottom.png -> stone.png
    // etc ...
    fn get_texture_index(&self, block: Block, face: Face) -> usize {
        for specifier in face.specifiers() {
            if let Some(i) = self.get(&(block, *specifier)) {
                return *i;
            }
        }
        0
    }
}

fn build_tex_array(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, ArrayTextureMaterial>>>,
) {
    let size = 16_u32;
    let image_handle = images.add(Image::new_fill(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: size,
        },
        TextureDimension::D3,
        &[0, 0, 0, 0],
        TextureFormat::R32Uint,
        RenderAssetUsages::RENDER_WORLD,
    ));
    let handle = materials.add(ExtendedMaterial {
        base: StandardMaterial {
            perceptual_roughness: 1.,
            reflectance: 0.1,
            alpha_mode: AlphaMode::AlphaToCoverage,
            ..Default::default()
        },
        extension: ArrayTextureMaterial {
            ao_data: image_handle,
        },
    });
    commands.insert_resource(BlockTextureArray(handle));
}

#[derive(Resource)]
pub struct BlockTextureArray(pub Handle<ExtendedMaterial<StandardMaterial, ArrayTextureMaterial>>);

#[derive(Asset, AsBindGroup, Debug, Clone, TypePath)]
pub struct ArrayTextureMaterial {
    #[texture(100, dimension = "3d", sample_type = "u_int")]
    #[sampler(101, sampler_type = "non_filtering")] // Note: filtering = false
    pub ao_data: Handle<Image>,
}

impl MaterialExtension for ArrayTextureMaterial {
    fn vertex_shader() -> ShaderRef {
        CHUNK_MATERIAL_SHADER.into()
    }

    fn fragment_shader() -> ShaderRef {
        CHUNK_MATERIAL_SHADER.into()
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<ArrayTextureMaterial>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        let mut pos_position = 0;
        let mut normal_position = 1;
        let mut color_position = 5;
        let mut uv_position = 2;
        if let Some(label) = &mut descriptor.label {
            // println!("Label is: {}", label);
            if label == "pbr_prepass_pipeline" {
                pos_position = 0;
                uv_position = 1;
                normal_position = 3;
                color_position = 7;
            }
        }
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(pos_position),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(normal_position),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(color_position),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(uv_position),
            // Mesh::ATTRIBUTE_TANGENT.at_shader_location(4),
            ATTRIBUTE_QUAD_SIZE.at_shader_location(50),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}
