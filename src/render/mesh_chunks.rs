use std::collections::BTreeSet;

use bevy::{
    log::info_span,
    prelude::Mesh,
    render::{
        mesh::{Indices, MeshVertexAttribute},
        render_asset::RenderAssetUsages,
        render_resource::{PrimitiveTopology, VertexFormat},
    },
};
use binary_greedy_meshing as bgm;

use crate::world::CHUNK_S1;
use crate::{
    block::Face,
    world::{pad_linearize, Chunk, CHUNKP_S3},
    Block,
};

const MASK_6: u64 = 0b111111;
const MASK_6_u32: u32 = 0b111111;
const MASK_XYZ: u64 = 0b111111_111111_111111;

impl Chunk {
    pub fn voxel_data_lod(&self, lod: usize) -> Vec<u16> {
        let voxels = self.data.unpack_u16();
        if lod == 1 {
            return voxels;
        }
        let mut res = vec![0; CHUNKP_S3];
        for x in 0..CHUNK_S1 {
            for y in 0..CHUNK_S1 {
                for z in 0..CHUNK_S1 {
                    let lod_i = pad_linearize(x / lod, y / lod, z / lod);
                    if res[lod_i] == 0 {
                        res[lod_i] = voxels[pad_linearize(x, y, z)];
                    }
                }
            }
        }
        res
    }

    /// Doesn't work with lod > 2, because chunks are of size 62 (to get to 64 with padding) and 62 = 2*31
    /// TODO: make it work with lod > 2 if necessary (by truncating quads)
    pub fn create_face_meshes(
        &self,
        lod: usize,
    ) -> [Option<Mesh>; 6] {
        // Gathering binary greedy meshing input data
        let mesh_data_span = info_span!("mesh voxel data", name = "mesh voxel data").entered();
        let voxels = self.voxel_data_lod(lod);
        let mut mesh_data = bgm::MeshData::new();
        mesh_data_span.exit();
        let mesh_build_span = info_span!("mesh build", name = "mesh build").entered();
        let transparents =
            BTreeSet::from_iter(self.palette.iter().enumerate().filter_map(|(i, block)| {
                if i != 0 && !block.is_opaque() {
                    Some(i as u16)
                } else {
                    None
                }
            }));
        bgm::mesh(&voxels, &mut mesh_data, transparents);
        let mut meshes = core::array::from_fn(|_| None);
        for (face_n, quads) in mesh_data.quads.iter().enumerate() {
            let mut positions = Vec::with_capacity(quads.len() * 4);
            let mut normals = Vec::with_capacity(quads.len() * 4);
            let mut uvs = Vec::with_capacity(quads.len() * 4);
            let mut colors = Vec::with_capacity(quads.len() * 4);
            let indices = bgm::indices(quads.len());
            let face: Face = face_n.into();
            let face_normal = face.n().map(|n| n as f32);
            for quad in quads {
                let voxel_i = (quad >> 32) as usize;
                let w = MASK_6 & (quad >> 18);
                let h = MASK_6 & (quad >> 24);
                let xyz = MASK_XYZ & quad;
                let block = self.palette[voxel_i];
                
                let color = match (block, face) {
                    (Block::GrassBlock, Face::Up) => [0.0, 1.0, 0.0, 1.0],
                    (Block::SeaBlock, _) => [0.0, 0.0, 1.0, 1.0],
                    (block, _) if block.is_foliage() => [0.0, 0.5, 0.0, 1.0],
                    _ => [1.0, 1.0, 1.0, 1.0],
                };
                let packed_vertices =
                    face.vertices_packed(xyz as u32, w as u32, h as u32, lod as u32);
                for packed_vertex in packed_vertices.iter() {
                    let x = (packed_vertex & MASK_6_u32) as f32;
                    let y = ((packed_vertex >> 6) & MASK_6_u32) as f32;
                    let z = ((packed_vertex >> 12) & MASK_6_u32) as f32;
                    let u = ((packed_vertex >> 18) & MASK_6_u32) as f32;
                    let v = ((packed_vertex >> 24) & MASK_6_u32) as f32;

                    positions.push([x, y, z]);
                    normals.push(face_normal);
                    uvs.push([u / 63.0, v / 63.0]); // Normalize UV coordinates
                    colors.push(color);
                }
            }
            meshes[face_n] = Some(
                Mesh::new(
                    PrimitiveTopology::TriangleList,
                    RenderAssetUsages::RENDER_WORLD,
                )
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
                .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
                .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
                .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
                .with_inserted_indices(Indices::U32(indices)),
            )
        }
        mesh_build_span.exit();
        meshes
    }
}
