use bevy::{
    asset::{load_internal_asset, RenderAssetUsages},
    image::ImageSampler,
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{
        AsBindGroup, Extent3d, ShaderRef, ShaderType, TextureDimension, TextureFormat,
    },
};

const CHUNK_MATERIAL_SHADER: Handle<Shader> = Handle::weak_from_u128(102258915422227);
pub struct TextureArrayPlugin;

impl Plugin for TextureArrayPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            CHUNK_MATERIAL_SHADER,
            "chunk_material.wgsl",
            Shader::from_wgsl
        );
        app.add_systems(Startup, build_base_mat)
            .add_plugins(MaterialPlugin::<
                ExtendedMaterial<StandardMaterial, VoxelChunkMaterial>,
            >::default());
    }
}

fn build_base_mat(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, VoxelChunkMaterial>>>,
) {
    let texture_size = UVec3::new(32, 32, 16);
    let texture_data = vec![0u8; (texture_size.x * texture_size.y * texture_size.z) as usize];

    let mut image = Image::new(
        Extent3d {
            width: texture_size.x,
            height: texture_size.y,
            depth_or_array_layers: texture_size.z,
        },
        TextureDimension::D3,
        texture_data,
        TextureFormat::R8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );

    image.sampler = ImageSampler::nearest();
    let base_image_handle = images.add(image);
    let handle = materials.add(ExtendedMaterial {
        base: StandardMaterial {
            perceptual_roughness: 1.,
            reflectance: 0.1,
            alpha_mode: AlphaMode::AlphaToCoverage,
            ..Default::default()
        },
        extension: VoxelChunkMaterial {
            chunk_data: ChunkDataModel {
                chunk_position: IVec3::ZERO,
            },
            voxel_data: base_image_handle,
        },
    });
    commands.insert_resource(VoxelAOMaterial(handle));
}

#[derive(Resource)]
pub struct VoxelAOMaterial(pub Handle<ExtendedMaterial<StandardMaterial, VoxelChunkMaterial>>);

#[derive(ShaderType, Debug, Clone)]
pub struct ChunkDataModel {
    pub chunk_position: IVec3,
}
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct VoxelChunkMaterial {
    #[uniform(100)]
    pub chunk_data: ChunkDataModel,

    #[texture(101, dimension = "3d")]
    #[sampler(102)]
    pub voxel_data: Handle<Image>,
}

impl MaterialExtension for VoxelChunkMaterial {
    fn fragment_shader() -> ShaderRef {
        CHUNK_MATERIAL_SHADER.into()
    }
}
