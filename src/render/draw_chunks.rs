use super::chunk_culling::chunk_culling;
use super::shared_load_area::{setup_shared_load_area, update_shared_load_area, SharedLoadArea};
use super::texture_array::{ArrayTextureMaterial, BlockTextureArray};
use super::texture_array::{TextureArrayPlugin, TextureMap};
use crate::block::Face;
use crate::world::pos2d::chunks_in_col;
use crate::world::{range_around, ColUnloadEvent, LoadAreaAssigned, PlayerArea};
use crate::world::{ChunkPos, VoxelWorld, CHUNK_S1, Y_CHUNKS};
use bevy::color::palettes::css;
use bevy::pbr::ExtendedMaterial;
use bevy::prelude::*;
use bevy::render::primitives::Aabb;
use bevy::render::view::NoFrustumCulling;
use bevy::tasks::AsyncComputeTaskPool;
use itertools::{iproduct, Itertools};
use std::collections::HashMap;
use std::sync::Arc;
use std::thread::yield_now;
use strum::IntoEnumIterator;
const GRID_GIZMO_LEN: i32 = 4;

#[derive(Debug, Component)]
pub struct LOD(pub usize);

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
    )>,
    mut meshes: ResMut<Assets<Mesh>>,
    block_tex_array: Res<BlockTextureArray>,
    load_area: Res<PlayerArea>,
) {
    // Process current mesh if there is one
    if let Some((chunk_pos, dist)) = mesh_queue.in_progress.take() {
        // Skip if the chunk is no longer in the load area
        if !load_area.col_dists.contains_key(&chunk_pos.into()) {
            return;
        }

        let lod = choose_lod_level(dist);
        if let Some(mut chunk) = blocks.chunks.get_mut(&chunk_pos) {
            let face_meshes = chunk.create_face_meshes();
            chunk.changed = false;

            for (i, face_mesh) in face_meshes.into_iter().enumerate() {
                let face: Face = i.into();

                if let Some(mesh) = face_mesh {
                    let chunk_aabb = Aabb::from_min_max(Vec3::ZERO, Vec3::splat(CHUNK_S1 as f32));

                    // Check if entity already exists for this chunk face
                    if let Some(ent) = chunk_ents.0.get(&(chunk_pos, face)) {
                        if let Ok((mut handle, mut mat, mut old_lod)) = mesh_query.get_mut(*ent) {
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

                        let ent = commands
                            .spawn((
                                Mesh3d(meshes.add(mesh)),
                                MeshMaterial3d(new_material),
                                Transform::from_translation(
                                    Vec3::new(
                                        chunk_pos.x as f32,
                                        chunk_pos.y as f32,
                                        chunk_pos.z as f32,
                                    ) * CHUNK_S1 as f32,
                                ),
                                NoFrustumCulling,
                                chunk_aabb,
                                LOD(lod),
                                face,
                            ))
                            .id();
                        chunk_ents.0.insert((chunk_pos, face), ent);
                    }
                } else {
                    // If there's no mesh for this face, remove any existing entity
                    if let Some(ent) = chunk_ents.0.remove(&(chunk_pos, face)) {
                        commands.entity(ent).despawn();
                    }
                }
            }
        }
    }

    // Get next mesh to process
    if mesh_queue.in_progress.is_none() && !mesh_queue.queue.is_empty() {
        mesh_queue.in_progress = mesh_queue.queue.pop();
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
            for face in Face::iter() {
                if let Some(ent) = chunk_ents.0.remove(&(chunk_pos, face)) {
                    if let Ok(handle) = mesh_query.get(ent) {
                        meshes.remove(handle);
                    }
                    commands.entity(ent).despawn();
                }
            }
        }
    }
}

#[derive(Resource)]
pub struct ChunkEntities(pub HashMap<(ChunkPos, Face), Entity>);

impl ChunkEntities {
    pub fn new() -> Self {
        ChunkEntities(HashMap::new())
    }
}

pub struct Draw3d;

impl Plugin for Draw3d {
    fn build(&self, app: &mut App) {
        app.add_plugins(TextureArrayPlugin)
            .init_resource::<MeshGenerationQueue>()
            .insert_resource(ChunkEntities::new())
            .add_systems(
                Startup,
                (
                    setup_shared_load_area,
                    apply_deferred,
                    // apply_deferred,
                )
                    .chain()
                    .after(LoadAreaAssigned),
            )
            .add_systems(Update, (queue_mesh_generation, process_mesh_queue))
            .add_systems(Update, update_shared_load_area)
            .add_systems(Update, on_col_unload)
            //.add_systems(Update, chunk_aabb_gizmos)
            .add_systems(PostUpdate, chunk_culling);
    }
}
