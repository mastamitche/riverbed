use super::binary_greedy_meshing::MeshData;
use super::shared_load_area::{setup_shared_load_area, update_shared_load_area, SharedLoadArea};
use super::texture_array::TextureArrayPlugin;
use super::texture_array::{ArrayTextureMaterial, BlockTextureArray};
use crate::scenes::builder::systems::BUILDER_CHUNK_POS;
use crate::world::pos2d::chunks_in_col;
use crate::world::{range_around, ColUnloadEvent, LoadAreaAssigned, PlayerArea};
use crate::world::{ChunkPos, VoxelWorld, CHUNK_S1, Y_CHUNKS};
use avian3d::math::Quaternion;
use avian3d::prelude::{Collider, RigidBody};
use bevy::color::palettes::css;
use bevy::pbr::ExtendedMaterial;
use bevy::prelude::*;
use bevy::render::primitives::Aabb;
use itertools::iproduct;
use std::collections::{BTreeSet, HashMap};
use std::time::Instant;
const GRID_GIZMO_LEN: i32 = 4;

pub const MAX_MESHING_MS: u32 = 5;

#[derive(Debug, Component)]
pub struct LOD(pub usize);

#[derive(Debug, Component)]
pub struct WorldMesh;

fn choose_lod_level(chunk_dist: u32) -> usize {
    1
}

fn chunk_aabb_gizmos(mut gizmos: Gizmos, load_area: Res<PlayerArea>) {
    for (x, y) in iproduct!(
        range_around(load_area.center.x, GRID_GIZMO_LEN),
        0..=Y_CHUNKS
    ) {
        let start = Vec3::new(
            x as f32,
            y as f32,
            (load_area.center.z - GRID_GIZMO_LEN) as f32,
        ) * CHUNK_S1 as f32;
        let end = Vec3::new(
            x as f32,
            y as f32,
            (load_area.center.z + GRID_GIZMO_LEN) as f32,
        ) * CHUNK_S1 as f32;
        gizmos.line(start, end, Color::Srgba(css::YELLOW));
    }
    for (z, y) in iproduct!(
        range_around(load_area.center.z, GRID_GIZMO_LEN),
        0..=Y_CHUNKS
    ) {
        let start = Vec3::new(
            (load_area.center.x - GRID_GIZMO_LEN) as f32,
            y as f32,
            z as f32,
        ) * CHUNK_S1 as f32;
        let end = Vec3::new(
            (load_area.center.x + GRID_GIZMO_LEN) as f32,
            y as f32,
            z as f32,
        ) * CHUNK_S1 as f32;
        gizmos.line(start, end, Color::Srgba(css::YELLOW));
    }
    for (x, z) in iproduct!(
        range_around(load_area.center.x, GRID_GIZMO_LEN),
        range_around(load_area.center.z, GRID_GIZMO_LEN)
    ) {
        let start = Vec3::new(x as f32, 0., z as f32) * CHUNK_S1 as f32;
        let end = Vec3::new(x as f32, Y_CHUNKS as f32, z as f32) * CHUNK_S1 as f32;
        gizmos.line(start, end, Color::Srgba(css::YELLOW));
    }
}

#[derive(Resource, Default)]
pub struct MeshGenerationQueue {
    queue: Vec<(ChunkPos, u32)>,
    in_progress: Option<(ChunkPos, u32)>,
    // Track meshing state between frames
    meshing_state: Option<ChunkMeshingState>,
}

pub struct ChunkMeshingState {
    // Current stage of meshing
    pub stage: MeshingStage,
    pub mesh_data: MeshData,
    pub voxels: Vec<u16>,
    pub transparents: BTreeSet<u16>,
    pub next_vertex_index: i32,
    pub vertex_map: HashMap<
        (
            (i32, i32, i32), // position
            i32,
            i32,
            i32, // normal
            u8,
            u8,
            u8,
            u8, // color
            i32,
            i32, // uv
        ),
        i32,
    >,
    pub all_positions: Vec<[f32; 3]>,
    pub all_normals: Vec<[f32; 3]>,
    pub all_indices: Vec<u16>,
    pub all_uvs: Vec<[f32; 2]>,
    pub all_colors: Vec<[f32; 4]>,
    // pub all_quad_sizes: Vec<[f32; 2]>,
    pub all_physics_quads: Vec<[Vec3; 4]>,
    pub is_empty: bool,
    pub current_face: usize,
    pub current_quad_index: usize,
    pub quad_batch_size: usize,
}
impl Default for ChunkMeshingState {
    fn default() -> Self {
        Self {
            stage: MeshingStage::PrepareData,
            mesh_data: MeshData::new(),
            voxels: Vec::new(),
            transparents: BTreeSet::new(),
            next_vertex_index: 0,
            vertex_map: HashMap::new(),
            all_positions: Vec::new(),
            all_normals: Vec::new(),
            all_indices: Vec::new(),
            all_uvs: Vec::new(),
            all_colors: Vec::new(),
            // all_quad_sizes: Vec::new(),
            all_physics_quads: Vec::new(),
            is_empty: false,
            current_face: 0,
            current_quad_index: 0,
            quad_batch_size: 50,
        }
    }
}
impl ChunkMeshingState {
    pub fn is_overtime(&self, timer: &Instant) -> bool {
        // Convert MAX_MESHING_MS from milliseconds to nanoseconds
        const MAX_MESHING_NS: u128 = (MAX_MESHING_MS as u128) * 1_000_000;

        let elapsed_ns = timer.elapsed().as_nanos();

        elapsed_ns > MAX_MESHING_NS
    }
}
#[derive(Default, PartialEq, Eq, Debug)]
pub enum MeshingStage {
    #[default]
    PrepareData,
    Transparents,
    GreedyMeshing,
    ProcessQuads,
    Finalize,
    Complete,
}

pub fn queue_mesh_generation(
    mut mesh_queue: ResMut<MeshGenerationQueue>,
    shared_load_area: Res<SharedLoadArea>,
    blocks: Res<VoxelWorld>,
) {
    if mesh_queue.in_progress.is_none() {
        if let Some(shared_area) = shared_load_area.0.try_read() {
            if let Some((chunk_pos, dist)) = shared_area.pop_closest_change(&blocks.chunks) {
                mesh_queue.queue.push((chunk_pos, dist));
            }
        }
    }
}

fn still_in_load_area(chunk_pos: ChunkPos, load_area: &PlayerArea) -> bool {
    load_area.col_dists.contains_key(&chunk_pos.into()) || chunk_pos == BUILDER_CHUNK_POS
}
#[allow(clippy::collapsible_else_if)]
#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
pub fn process_mesh_queue(
    mut mesh_queue: ResMut<MeshGenerationQueue>,
    blocks: Res<VoxelWorld>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, ArrayTextureMaterial>>>,
    mut chunk_ents: ResMut<ChunkEntities>,
    mut mesh_query: Query<(
        &mut Mesh3d,
        &mut MeshMaterial3d<ExtendedMaterial<StandardMaterial, ArrayTextureMaterial>>,
        &mut LOD,
        &mut Collider,
        &mut Transform,
    )>,
    mut meshes: ResMut<Assets<Mesh>>,
    block_tex_array: Res<BlockTextureArray>,
    load_area: Res<PlayerArea>,
) {
    // Process current mesh if there is one
    if let Some((chunk_pos, dist)) = mesh_queue.in_progress {
        // Skip if the chunk is no longer in the load area
        if !still_in_load_area(chunk_pos, &load_area) {
            mesh_queue.in_progress = None;
            return;
        }

        let lod = choose_lod_level(dist);
        if let Some(mut chunk) = blocks.chunks.get_mut(&chunk_pos) {
            let face_mesh = chunk.create_face_meshes(mesh_queue.meshing_state.as_mut().unwrap());
            let meshing_state = mesh_queue.meshing_state.as_ref().unwrap();
            if meshing_state.is_empty {
                //remove empty mesh chunk
                if let Some(ent) = chunk_ents.0.remove(&chunk_pos) {
                    commands.entity(ent).despawn();
                }
                mesh_queue.in_progress = None;
            } else {
                if meshing_state.stage == MeshingStage::Complete {
                    chunk.changed = false;
                    mesh_queue.in_progress = None;

                    if let Some((mesh, physics_quads)) = face_mesh {
                        let chunk_aabb =
                            Aabb::from_min_max(Vec3::ZERO, Vec3::splat((CHUNK_S1 as f32) / 8.));
                        // Create compound collider from all cuboids
                        let new_collider = Collider::trimesh_from_mesh(&mesh).unwrap();
                        // Check if entity already exists for this chunk face
                        if let Some(ent) = chunk_ents.0.get(&chunk_pos) {
                            if let Ok((mut handle, mut mat, mut old_lod, mut collider, _)) =
                                mesh_query.get_mut(*ent)
                            {
                                let image = chunk.create_ao_texture_data();
                                let ao_image_handle = images.add(image);
                                chunk.ao_image = Some(ao_image_handle.clone());
                                chunk.meshing = false;

                                let ref_mat = materials.get_mut(&block_tex_array.0).unwrap();
                                let base = ref_mat.base.clone();
                                let new_material = materials.add(ExtendedMaterial {
                                    base: StandardMaterial { ..base },
                                    extension: ArrayTextureMaterial {
                                        ao_data: chunk.ao_image.clone().unwrap(),
                                    },
                                });

                                mat.0 = new_material;
                                handle.0 = meshes.add(mesh);
                                *old_lod = LOD(lod);
                                *collider = new_collider;
                            } else {
                                println!("couldn't get_mut mesh for chunk {}", chunk_pos);
                            }
                        } else {
                            // Create new entity if it doesn't exist
                            if chunk.ao_image.is_none() {
                                let image = chunk.create_ao_texture_data();
                                let ao_image_handle = images.add(image);
                                chunk.ao_image = Some(ao_image_handle.clone());
                                chunk.meshing = false;
                            }

                            let ref_mat = materials.get_mut(&block_tex_array.0).unwrap();
                            let base = ref_mat.base.clone();
                            let new_material = materials.add(ExtendedMaterial {
                                base: StandardMaterial { ..base },
                                extension: ArrayTextureMaterial {
                                    ao_data: chunk.ao_image.clone().unwrap(),
                                },
                            });
                            let mesh_handle = meshes.add(mesh);
                            let mesh_pos = Vec3::new(
                                (chunk_pos.x as f32) / 8.,
                                (chunk_pos.y as f32) / 8.,
                                (chunk_pos.z as f32) / 8.,
                            ) * CHUNK_S1 as f32;
                            let ent = commands
                                .spawn((
                                    Mesh3d(mesh_handle.clone()),
                                    MeshMaterial3d(new_material),
                                    Transform::from_translation(mesh_pos),
                                    // NoFrustumCulling,
                                    chunk_aabb,
                                    LOD(lod),
                                    Pickable {
                                        should_block_lower: true,
                                        is_hoverable: true,
                                    },
                                    WorldMesh,
                                    //SimplifiedMesh(mesh_handle),
                                    //Physics
                                    RigidBody::Static, // Static for terrain
                                    new_collider,
                                ))
                                .observe(
                                    |trigger: Trigger<Pointer<Move>>,
                                     mut building_state: ResMut<BuildingState>,
                                     mut preview_query: Query<
                                        (&mut Transform, &mut Visibility),
                                        With<BuildingPreview>,
                                    >| {
                                        let mv = trigger.event();
                                        // Convert world position to voxel grid (1/8 unit per voxel)
                                        if let Some(world_position) = mv.hit.position {
                                            if let Some(world_normal) = mv.hit.normal {
                                                let voxel_size = 0.125;
                                                let voxel_half_size = 0.0625;
                                                let voxel_pos = world_position / voxel_size;
                                                let target_voxel_pos =
                                                    (voxel_pos + world_normal * voxel_size).floor()
                                                        * voxel_size
                                                        + voxel_half_size;

                                                if let Ok((mut transform, mut visibility)) =
                                                    preview_query.single_mut()
                                                {
                                                    transform.translation = target_voxel_pos;
                                                    building_state.current_position =
                                                        Some(target_voxel_pos - voxel_half_size);
                                                    *visibility = Visibility::Visible;
                                                }
                                            }
                                        }
                                    },
                                )
                                .id();
                            chunk_ents.0.insert(chunk_pos, ent);
                        }
                    }
                }
            }
        }
    }

    // Get next mesh to process
    if mesh_queue.in_progress.is_none() && !mesh_queue.queue.is_empty() {
        mesh_queue.in_progress = mesh_queue.queue.pop();
        mesh_queue.meshing_state = Some(ChunkMeshingState::default());
    }
}

pub fn on_col_unload(
    mut commands: Commands,
    mut ev_unload: EventReader<ColUnloadEvent>,
    mut chunk_ents: ResMut<ChunkEntities>,
    mesh_query: Query<&Mesh3d>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for col_ev in ev_unload.read() {
        for chunk_pos in chunks_in_col(&col_ev.0) {
            if let Some(ent) = chunk_ents.0.remove(&chunk_pos) {
                if let Ok(handle) = mesh_query.get(ent) {
                    meshes.remove(handle);
                }
                commands.entity(ent).despawn();
            }
        }
    }
}

fn setup_building_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn an initially invisible preview cube
    commands.spawn((
        BuildingPreview,
        Mesh3d(meshes.add(Cuboid::new(0.125, 0.125, 0.125))),
        MeshMaterial3d(materials.add(Color::srgba(1., 1., 1., 0.6))),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Pickable::IGNORE,
        Visibility::Hidden,
    ));
}
#[derive(Resource, Default)]
pub struct BuildingState {
    pub current_position: Option<Vec3>,
    pub current_normal: Option<Vec3>,
}
#[derive(Component)]
pub struct BuildingPreview;

#[derive(Resource)]
pub struct ChunkEntities(pub HashMap<ChunkPos, Entity>);

impl ChunkEntities {
    pub fn new() -> Self {
        ChunkEntities(HashMap::new())
    }
}

pub struct Draw3d;

impl Plugin for Draw3d {
    fn build(&self, app: &mut App) {
        app.add_plugins(TextureArrayPlugin)
            .init_resource::<BuildingState>()
            .add_systems(Startup, setup_building_system)
            .init_resource::<MeshGenerationQueue>()
            .insert_resource(ChunkEntities::new())
            .add_systems(
                Startup,
                (setup_shared_load_area, apply_deferred)
                    .chain()
                    .after(LoadAreaAssigned::Assigned),
            )
            .add_systems(Update, (queue_mesh_generation, process_mesh_queue).chain())
            .add_systems(Update, update_shared_load_area)
            .add_systems(Update, on_col_unload)
            //.add_systems(Update, chunk_aabb_gizmos)
            // .add_systems(PostUpdate, chunk_culling)
            //
            ;
    }
}
