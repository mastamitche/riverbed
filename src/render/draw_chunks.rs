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
use crossbeam::channel::{unbounded, Receiver};
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

fn mark_lod_remesh(
    load_area: Res<PlayerArea>,
    chunk_ents: ResMut<ChunkEntities>,
    lods: Query<&LOD>,
    blocks: ResMut<VoxelWorld>,
) {
    // FIXME: this only remesh chunks that previously had a mesh
    // However in some rare cases a chunk with some blocs can produce an empty mesh at certain LODs
    // and never get remeshed even though it should
    if !load_area.is_changed() {
        return;
    }
    for ((chunk_pos, _), entity) in chunk_ents
        .0
        .iter()
        .unique_by(|((chunk_pos, _), _)| chunk_pos)
    {
        let Some(dist) = load_area.col_dists.get(&(*chunk_pos).into()) else {
            continue;
        };
        let new_lod = choose_lod_level(*dist);
        let Ok(old_lod) = lods.get(*entity) else {
            continue;
        };
        if new_lod != old_lod.0 {
            let Some(mut chunk) = blocks.chunks.get_mut(chunk_pos) else {
                continue;
            };
            chunk.changed = true;
        }
    }
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

#[derive(Resource)]
pub struct MeshReciever(Receiver<(Option<Mesh>, ChunkPos, Face, LOD)>);

fn setup_mesh_thread(
    mut commands: Commands,
    blocks: Res<VoxelWorld>,
    shared_load_area: Res<SharedLoadArea>,
    texture_map: Res<TextureMap>,
) {
    let thread_pool = AsyncComputeTaskPool::get();
    let chunks = Arc::clone(&blocks.chunks);
    let (mesh_sender, mesh_reciever) = unbounded();
    commands.insert_resource(MeshReciever(mesh_reciever));
    let shared_load_area = Arc::clone(&shared_load_area.0);
    let texture_map = Arc::clone(&texture_map.0);
    thread_pool
        .spawn(async move {
            loop {
                let Some((chunk_pos, dist)) = shared_load_area.read().pop_closest_change(&chunks)
                else {
                    yield_now();
                    continue;
                };
                //println!("meshing chunk {:?} with dist {}", chunk_pos, dist);
                let lod = choose_lod_level(dist);
                if chunk_pos.x == 8 && chunk_pos.y == 0 && chunk_pos.z == 2 {
                    println!("meshing chunk {:?} with dist {}", chunk_pos, dist);
                }
                let Some(mut chunk) = chunks.get_mut(&chunk_pos) else {
                    continue;
                };
                let face_meshes = chunk.create_face_meshes(&*texture_map, lod);
                chunk.changed = false;
                for (i, face_mesh) in face_meshes.into_iter().enumerate() {
                    let face = i.into();
                    if mesh_sender
                        .send((face_mesh, chunk_pos, face, LOD(lod)))
                        .is_err()
                    {
                        println!("mesh for {:?} couldn't be sent", chunk_pos)
                    };
                }
            }
        })
        .detach();
}

pub fn pull_meshes(
    mut commands: Commands,
    mesh_reciever: Res<MeshReciever>,
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
    blocks: Res<VoxelWorld>,
) {
    let received_meshes: Vec<_> = mesh_reciever
        .0
        .try_iter()
        .filter(|(_, chunk_pos, _, _)| load_area.col_dists.contains_key(&(*chunk_pos).into()))
        .collect();
    for (mesh_opt, chunk_pos, face, lod) in received_meshes
        .into_iter()
        .rev()
        .unique_by(|(_, pos, face, _)| (*pos, *face))
    {
        let Some(mesh) = mesh_opt else {
            if let Some(ent) = chunk_ents.0.remove(&(chunk_pos, face)) {
                commands.entity(ent).despawn();
            }
            continue;
        };
        //println!("Mesh available");
        let chunk_aabb = Aabb::from_min_max(Vec3::ZERO, Vec3::splat(CHUNK_S1 as f32));
        if let Some(ent) = chunk_ents.0.get(&(chunk_pos, face)) {
            if let Ok((mut handle, mut mat, mut old_lod)) = mesh_query.get_mut(*ent) {
                if chunk_pos.x == 8 && chunk_pos.y == 0 && chunk_pos.z == 2 {
                    println!("updating chunk {:?}", chunk_pos);
                }
                let mut chunk = blocks.chunks.get_mut(&chunk_pos).unwrap();
                let image = chunk.create_ao_texture_data(chunk_pos);
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
                *old_lod = lod;
            } else {
                // the entity is not instanciated yet, we put it back
                println!("entity wasn't ready to recieve updated mesh");
            }
        } else if blocks.chunks.contains_key(&chunk_pos) {
            let mut chunk = blocks.chunks.get_mut(&chunk_pos).unwrap();
            if chunk.ao_image.is_none() {
                let image = chunk.create_ao_texture_data(chunk_pos);
                let ao_image_handle = images.add(image);
                chunk.ao_image = Some(ao_image_handle.clone());
                chunk.meshing = false;
            }
            if chunk_pos.x == 8 && chunk_pos.y == 0 && chunk_pos.z == 2 {
                println!("sending chunk {:?}", chunk_pos);
            }
            let ref_mat = materials.get_mut(&block_tex_array.0).unwrap();
            let base = ref_mat.base.clone();
            // Create a new material instance for this chunk
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
                        Vec3::new(chunk_pos.x as f32, chunk_pos.y as f32, chunk_pos.z as f32)
                            * CHUNK_S1 as f32,
                    ),
                    NoFrustumCulling,
                    chunk_aabb,
                    lod,
                    face,
                ))
                .id();
            chunk_ents.0.insert((chunk_pos, face), ent);
        }
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
            .insert_resource(ChunkEntities::new())
            .add_systems(
                Startup,
                (
                    setup_shared_load_area,
                    apply_deferred,
                    setup_mesh_thread,
                    apply_deferred,
                )
                    .chain()
                    .after(LoadAreaAssigned),
            )
            .add_systems(Update, update_shared_load_area)
            .add_systems(Update, mark_lod_remesh)
            .add_systems(Update, pull_meshes)
            .add_systems(Update, on_col_unload)
            //.add_systems(Update, chunk_aabb_gizmos)
            .add_systems(PostUpdate, chunk_culling);
    }
}
