use crate::{
    block::{Face, FaceSpecifier},
    Block,
};
use bevy::{
    asset::LoadedFolder,
    pbr::{
        ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline,
        MaterialPipeline, MaterialPipelineKey,
    },
    prelude::*,
    reflect::TypePath,
    render::{
        mesh::{MeshVertexBufferLayout, MeshVertexBufferLayoutRef},
        render_asset::RenderAssetUsages,
        render_resource::{
            AsBindGroup, Extent3d, RenderPipelineDescriptor, ShaderRef,
            SpecializedMeshPipelineError, TextureDimension, TextureFormat,
        },
        storage::ShaderStorageBuffer,
    },
};
use dashmap::DashMap;
use std::sync::Arc;

use super::{
    ao_texture::{self, AOPattern, AOTextureArray},
    mesh_chunks::ATTRIBUTE_AO_DATA,
};

#[derive(Asset, AsBindGroup, Debug, Clone, TypePath)]
pub struct AOExtensionMaterial {
    #[texture(100)]
    #[sampler(101)]
    ao_none: Handle<Image>,

    #[texture(102)]
    #[sampler(103)]
    ao_one_corner: Handle<Image>,

    #[texture(104)]
    #[sampler(105)]
    ao_two_corners: Handle<Image>,

    #[texture(106)]
    #[sampler(107)]
    ao_two_opposite_corners: Handle<Image>,

    #[texture(108)]
    #[sampler(109)]
    ao_three_corners: Handle<Image>,

    #[texture(110)]
    #[sampler(111)]
    ao_four_corners: Handle<Image>,

    #[texture(112)]
    #[sampler(113)]
    ao_one_edge: Handle<Image>,

    #[texture(114)]
    #[sampler(115)]
    ao_opposite_edges: Handle<Image>,

    #[texture(116)]
    #[sampler(117)]
    ao_two_adjacent_edges: Handle<Image>,

    #[texture(118)]
    #[sampler(119)]
    ao_three_edges: Handle<Image>,

    #[texture(120)]
    #[sampler(121)]
    ao_four_edges: Handle<Image>,
}

impl MaterialExtension for AOExtensionMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/voxel_ao.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/voxel_ao.wgsl".into()
    }
    fn specialize(
        pipeline: &MaterialExtensionPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialExtensionKey<AOExtensionMaterial>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        // Get the existing vertex layout
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(5),
            ATTRIBUTE_AO_DATA.at_shader_location(6),
        ])?;

        // Update the descriptor with the new layout
        descriptor.vertex.buffers = vec![vertex_layout];

        Ok(())
    }
}

fn setup_ao_material(
    mut commands: Commands,
    ao_textures: Res<AOTextureArray>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, AOExtensionMaterial>>>,
) {
    let ao_material = materials.add(ExtendedMaterial {
        base: StandardMaterial {
            perceptual_roughness: 1.0,
            reflectance: 0.1,
            ..Default::default()
        },
        extension: AOExtensionMaterial {
            ao_none: ao_textures.textures[&AOPattern::None].clone(),
            ao_one_corner: ao_textures.textures[&AOPattern::OneCorner].clone(),
            ao_two_corners: ao_textures.textures[&AOPattern::TwoCorners].clone(),
            ao_two_opposite_corners: ao_textures.textures[&AOPattern::TwoOppositeCorners].clone(),
            ao_three_corners: ao_textures.textures[&AOPattern::ThreeCorners].clone(),
            ao_four_corners: ao_textures.textures[&AOPattern::FourCorners].clone(),
            ao_one_edge: ao_textures.textures[&AOPattern::OneEdge].clone(),
            ao_opposite_edges: ao_textures.textures[&AOPattern::OppositeEdges].clone(),
            ao_two_adjacent_edges: ao_textures.textures[&AOPattern::TwoAdjacentEdges].clone(),
            ao_three_edges: ao_textures.textures[&AOPattern::ThreeEdges].clone(),
            ao_four_edges: ao_textures.textures[&AOPattern::FourEdges].clone(),
        },
    });
    commands.insert_resource(AOExtensionMaterialResource(ao_material));
}

#[derive(Resource, Default)]
pub struct AOExtensionMaterialResource(
    pub Handle<ExtendedMaterial<StandardMaterial, AOExtensionMaterial>>,
);

pub struct AOExtensionMaterialPlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, SystemSet)]
pub struct AOAssigned;

impl Plugin for AOExtensionMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, AOExtensionMaterial>,
        >::default())
            .add_systems(
                Update,
                setup_ao_material.run_if(run_setup_if).in_set(AOAssigned),
            );
    }
}
fn run_setup_if(
    ao_texture_array: Option<Res<AOTextureArray>>,
    ao_extension_material_resource: Option<Res<AOExtensionMaterialResource>>,
) -> bool {
    ao_texture_array.is_some() && ao_extension_material_resource.is_none()
}
