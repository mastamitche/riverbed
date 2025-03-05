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
        render_resource::{
            AsBindGroup, CompareFunction, DepthBiasState, DepthStencilState, Extent3d, ShaderRef,
            StencilState, TextureDimension, TextureFormat,
        },
        storage::ShaderStorageBuffer,
    },
    window::{PrimaryWindow, WindowResolution, WindowWrapper},
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

fn build_tex_array(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    window: Query<&Window>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, ArrayTextureMaterial>>>,
) {
    let window = window.get_single().unwrap();
    let size = Extent3d {
        width: window.physical_width(),
        height: window.physical_height(),
        depth_or_array_layers: 1,
    };

    // Create SSAO texture
    let ssao_texture = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0; 4],
        TextureFormat::R8Unorm, // Single channel, 8-bit normalized
        RenderAssetUsages::RENDER_WORLD,
    );
    let ssao_texture_handle = images.add(ssao_texture);

    let handle = materials.add(ExtendedMaterial {
        base: StandardMaterial {
            perceptual_roughness: 1.,
            reflectance: 0.1,
            alpha_mode: AlphaMode::AlphaToCoverage,
            ..Default::default()
        },
        extension: ArrayTextureMaterial {
            ssao_texture: ssao_texture_handle.clone(),
            // We'll get the depth texture from Bevy later
            depth_texture: Handle::default(),
        },
    });

    commands.insert_resource(SSAOTexture(ssao_texture_handle));
    commands.insert_resource(BlockTextureArray(handle));
}
//for ssao
#[derive(Resource)]
struct SSAOTexture(Handle<Image>);

#[derive(Resource)]
pub struct BlockTextureArray(pub Handle<ExtendedMaterial<StandardMaterial, ArrayTextureMaterial>>);

#[derive(Asset, AsBindGroup, Debug, Clone, TypePath)]
pub struct ArrayTextureMaterial {
    #[texture(20, dimension = "2d")]
    #[sampler(21)]
    pub depth_texture: Handle<Image>,

    #[texture(22, dimension = "2d")]
    #[sampler(23)]
    pub ssao_texture: Handle<Image>,
}

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

        descriptor.depth_stencil = Some(DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: CompareFunction::LessEqual,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        });
        Ok(())
    }
}
