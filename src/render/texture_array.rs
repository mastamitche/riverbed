use super::mesh_chunks::ATTRIBUTE_VOXEL_DATA;
use crate::{
    block::{Face, FaceSpecifier},
    Block,
};
use bevy::{
    asset::LoadedFolder,
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

pub struct TextureArrayPlugin;

impl Plugin for TextureArrayPlugin {
    fn build(&self, app: &mut App) {
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

fn build_tex_array(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    let handle = materials.add(StandardMaterial {
        perceptual_roughness: 1.,
        reflectance: 0.1,
        alpha_mode: AlphaMode::AlphaToCoverage,
        ..Default::default()
    });
    commands.insert_resource(BlockTextureArray(handle));
}

#[derive(Resource)]
pub struct BlockTextureArray(pub Handle<StandardMaterial>);

#[derive(Asset, AsBindGroup, Debug, Clone, TypePath)]
pub struct ArrayTextureMaterial {}

impl MaterialExtension for ArrayTextureMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/chunk.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/chunk.wgsl".into()
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<ArrayTextureMaterial>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        let vertex_layout = layout
            .0
            .get_layout(&[ATTRIBUTE_VOXEL_DATA.at_shader_location(0)])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}
