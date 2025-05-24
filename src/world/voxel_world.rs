use super::{
    chunked, pos2d::chunks_in_col, BlockPos, BlockPos2d, Chunk, ChunkPos, ChunkedPos, ColPos,
    ColedPos, CHUNKP_S1, CHUNK_S1, CHUNK_S1I, MAX_HEIGHT, Y_CHUNKS,
};
use crate::{world::chunk, Block};
use bevy::{
    asset::Handle,
    image::Image,
    prelude::{Resource, Vec3},
};
use dashmap::DashMap;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

pub struct TrackedChunk {
    chunk: Chunk,
    pub ao_image: Option<Handle<Image>>,
    pub loaded: bool,
    pub meshing: bool,
    pub changed: bool,
}

impl TrackedChunk {
    pub fn new() -> Self {
        Self {
            chunk: Chunk::new(),
            ao_image: None,
            loaded: true,
            meshing: false,
            changed: true,
        }
    }
}

impl Deref for TrackedChunk {
    type Target = Chunk;

    fn deref(&self) -> &Self::Target {
        &self.chunk
    }
}

impl DerefMut for TrackedChunk {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.chunk
    }
}

pub struct BlockRayCastHit {
    pub pos: BlockPos,
    pub normal: Vec3,
}

impl PartialEq for BlockRayCastHit {
    fn eq(&self, other: &Self) -> bool {
        self.pos == other.pos
    }
}

#[derive(Resource)]
pub struct VoxelWorld {
    pub chunks: Arc<DashMap<ChunkPos, TrackedChunk>>,
}

impl VoxelWorld {
    pub fn new() -> Self {
        VoxelWorld {
            chunks: Arc::new(DashMap::new()),
        }
    }

    pub fn new_with(chunks: Arc<DashMap<ChunkPos, TrackedChunk>>) -> Self {
        VoxelWorld { chunks }
    }

    pub fn set_block(&self, pos: BlockPos, block: Block) {
        let (chunk_pos, chunked_pos) = <(ChunkPos, ChunkedPos)>::from(pos);

        // Try to get the chunk if it exists
        if let Some(mut chunk) = self.chunks.get_mut(&chunk_pos) {
            chunk.set(chunked_pos, block);
        } else {
            // If it doesn't exist, insert a new chunk with the block already set
            let mut new_chunk = TrackedChunk::new();
            new_chunk.set(chunked_pos, block);
            self.chunks.insert(chunk_pos, new_chunk);
        }

        self.mark_change(chunk_pos, chunked_pos, block);
    }

    pub fn set_block_safe(&self, pos: BlockPos, block: Block) -> bool {
        if pos.y < 0 || pos.y >= MAX_HEIGHT as i32 {
            return false;
        }
        let (chunk_pos, chunked_pos) = <(ChunkPos, ChunkedPos)>::from(pos);
        self.chunks
            .entry(chunk_pos)
            .or_insert_with(TrackedChunk::new)
            .set(chunked_pos, block);
        self.mark_change(chunk_pos, chunked_pos, block);
        true
    }

    pub fn set_loaded(&self, chunk_pos: ChunkPos) -> bool {
        if let Some(mut chunk) = self.chunks.get_mut(&chunk_pos) {
            chunk.loaded = true;
            chunk.changed = true;
            true
        } else {
            false // Chunk doesn't exist, so we can't mark it as loaded
        }
    }

    pub fn set_yrange(
        &self,
        col_pos: ColPos,
        (x, z): ColedPos,
        top: i32,
        height: usize,
        block: Block,
    ) {
        //TODO: Logging if this is a border: log count of copy to neighbour
        // Convert column position and coordinates to base BlockPos
        let base_x = col_pos.x * CHUNK_S1I + x as i32;
        let base_z = col_pos.z * CHUNK_S1I + z as i32;

        // Starting from the top, set each block down to the specified height
        for y_offset in 0..height {
            let y = top - y_offset as i32;
            if y < 0 {
                break; // Don't go below zero
            }

            let pos = BlockPos {
                x: base_x,
                y,
                z: base_z,
            };

            self.set_block(pos, block);
        }
    }

    pub fn set_if_empty(&self, pos: BlockPos, block: Block) {
        let (chunk_pos, chunked_pos) = <(ChunkPos, ChunkedPos)>::from(pos);
        if self
            .chunks
            .entry(chunk_pos)
            .or_insert_with(TrackedChunk::new)
            .set_if_empty(chunked_pos, block)
        {
            self.mark_change(chunk_pos, chunked_pos, block);
        }
    }

    pub fn get_block(&self, pos: BlockPos) -> Block {
        let (chunk_pos, chunked_pos) = <(ChunkPos, ChunkedPos)>::from(pos);
        match self.chunks.get(&chunk_pos) {
            None => Block::Air,
            Some(chunk) => chunk.get(chunked_pos).clone(),
        }
    }

    pub fn get_block_safe(&self, pos: BlockPos) -> Block {
        if pos.y < 0 || pos.y >= MAX_HEIGHT as i32 {
            Block::Air
        } else {
            self.get_block(pos)
        }
    }

    pub fn top_block(&self, pos: BlockPos2d) -> (Block, i32) {
        let (col_pos, pos2d) = pos.into();
        for y in (0..Y_CHUNKS as i32).rev() {
            let chunk_pos = ChunkPos {
                x: col_pos.x,
                y,
                z: col_pos.z,
            };
            if let Some(chunk) = self.chunks.get(&chunk_pos) {
                let (block, block_y) = chunk.top(pos2d);
                if *block != Block::Air {
                    return (block.clone(), y * CHUNK_S1 as i32 + block_y as i32);
                }
            }
        }
        (Block::Air, 0)
    }

    pub fn is_col_loaded(&self, player_pos: Vec3) -> bool {
        let (chunk_pos, _): (ChunkPos, _) = <BlockPos>::from(player_pos).into();
        for y in (0..Y_CHUNKS as i32).rev() {
            let chunk = ChunkPos {
                x: chunk_pos.x,
                y,
                z: chunk_pos.z,
            };
            if self.chunks.contains_key(&chunk) {
                return true;
            }
        }
        false
    }

    pub fn mark_change_col(&self, col_pos: ColPos) {
        // USE BY TERRAIN GEN to mass mark change on chunks for efficiency
        for chunk_pos in chunks_in_col(&col_pos) {
            self.mark_change_single(chunk_pos);
        }
    }

    pub fn unload_col(&self, col: ColPos) {
        for y in 0..Y_CHUNKS as i32 {
            let chunk_pos = ChunkPos {
                x: col.x,
                y,
                z: col.z,
            };
            self.chunks.remove(&chunk_pos);
        }
    }

    pub fn mark_change_single(&self, chunk_pos: ChunkPos) {
        if let Some(mut chunk) = self.chunks.get_mut(&chunk_pos) {
            chunk.changed = true;
        } else {
            println!("couldn't get_mut chunk {:?}", chunk_pos);
        }
    }

    fn border_sign(coord: usize) -> i32 {
        if coord == 0 {
            -1
        } else if coord == CHUNK_S1 - 1 {
            1
        } else {
            0
        }
    }
    pub fn mark_change(&self, chunk_pos: ChunkPos, chunked_pos: ChunkedPos, block: Block) {
        // Get border signs for each dimension
        let border_sign_x = VoxelWorld::border_sign(chunked_pos.0);
        let border_sign_y = VoxelWorld::border_sign(chunked_pos.1);
        let border_sign_z = VoxelWorld::border_sign(chunked_pos.2);
        // Mark the current chunk as changed
        self.mark_change_single(chunk_pos);
        let chunked_64_space = (chunked_pos.0 + 1, chunked_pos.1 + 1, chunked_pos.2 + 1);
        // Only proceed if we're at a border in at least one dimension
        if border_sign_x == 0 && border_sign_y == 0 && border_sign_z == 0 {
            return;
        }

        // X-axis neighbors (if we're at an x-border)
        if border_sign_x != 0 {
            let neighbor_chunk_pos = ChunkPos {
                x: chunk_pos.x + border_sign_x,
                y: chunk_pos.y,
                z: chunk_pos.z,
            };

            let neighbor_x = if border_sign_x < 0 { CHUNKP_S1 - 1 } else { 0 };
            // Make sure y and z are within the valid 62³ space (0 to CHUNK_S1-1)
            let neighbor_y = chunked_64_space.1;
            let neighbor_z = chunked_64_space.2;
            let neighbor_chunked_pos = (neighbor_x, neighbor_y, neighbor_z);
            // println!(
            //     "Updting x neighbor {:?} from {:?}, {:?}",
            //     neighbor_chunk_pos, chunk_pos, neighbor_chunked_pos
            // );

            self.update_neighbor_chunk(neighbor_chunk_pos, neighbor_chunked_pos, block);
        }

        // Y-axis neighbors (if we're at a y-border)
        if border_sign_y != 0 {
            let neighbor_chunk_pos = ChunkPos {
                x: chunk_pos.x,
                y: chunk_pos.y + border_sign_y,
                z: chunk_pos.z,
            };

            let neighbor_y = if border_sign_y < 0 { CHUNKP_S1 - 1 } else { 0 };
            // Make sure x and z are within the valid 62³ space (0 to CHUNK_S1-1)
            let neighbor_x = chunked_64_space.0;
            let neighbor_z = chunked_64_space.2;
            let neighbor_chunked_pos = (neighbor_x, neighbor_y, neighbor_z);

            //println!("Updting y neighbor {}", border_sign_y);

            self.update_neighbor_chunk(neighbor_chunk_pos, neighbor_chunked_pos, block);
        }

        // Z-axis neighbors (if we're at a z-border)
        if border_sign_z != 0 {
            let neighbor_chunk_pos = ChunkPos {
                x: chunk_pos.x,
                y: chunk_pos.y,
                z: chunk_pos.z + border_sign_z,
            };

            let neighbor_z = if border_sign_z < 0 { CHUNKP_S1 - 1 } else { 0 };
            // Make sure x and y are within the valid 62³ space (0 to CHUNK_S1-1)
            let neighbor_x = chunked_64_space.0;
            let neighbor_y = chunked_64_space.1;
            let neighbor_chunked_pos = (neighbor_x, neighbor_y, neighbor_z);

            // println!(
            //     "Updting z neighbor {:?} from {:?}, {:?}",
            //     neighbor_chunk_pos, chunk_pos, neighbor_chunked_pos
            // );

            self.update_neighbor_chunk(neighbor_chunk_pos, neighbor_chunked_pos, block);
        }

        // XY diagonal neighbors (if we're at both x and y borders)
        if border_sign_x != 0 && border_sign_y != 0 {
            let neighbor_chunk_pos = ChunkPos {
                x: chunk_pos.x + border_sign_x,
                y: chunk_pos.y + border_sign_y,
                z: chunk_pos.z,
            };

            let neighbor_x = if border_sign_x < 0 { CHUNKP_S1 - 1 } else { 0 };
            let neighbor_y = if border_sign_y < 0 { CHUNKP_S1 - 1 } else { 0 };
            let neighbor_chunked_pos = (neighbor_x, neighbor_y, chunked_64_space.2);

            self.update_neighbor_chunk(neighbor_chunk_pos, neighbor_chunked_pos, block);
        }

        // XZ diagonal neighbors (if we're at both x and z borders)
        if border_sign_x != 0 && border_sign_z != 0 {
            let neighbor_chunk_pos = ChunkPos {
                x: chunk_pos.x + border_sign_x,
                y: chunk_pos.y,
                z: chunk_pos.z + border_sign_z,
            };

            let neighbor_x = if border_sign_x < 0 { CHUNKP_S1 - 1 } else { 0 };
            let neighbor_z = if border_sign_z < 0 { CHUNKP_S1 - 1 } else { 0 };
            let neighbor_chunked_pos = (neighbor_x, chunked_64_space.1, neighbor_z);

            self.update_neighbor_chunk(neighbor_chunk_pos, neighbor_chunked_pos, block);
        }

        // YZ diagonal neighbors (if we're at both y and z borders)
        if border_sign_y != 0 && border_sign_z != 0 {
            let neighbor_chunk_pos = ChunkPos {
                x: chunk_pos.x,
                y: chunk_pos.y + border_sign_y,
                z: chunk_pos.z + border_sign_z,
            };

            let neighbor_y = if border_sign_y < 0 { CHUNKP_S1 - 1 } else { 0 };
            let neighbor_z = if border_sign_z < 0 { CHUNKP_S1 - 1 } else { 0 };
            let neighbor_chunked_pos = (chunked_64_space.0, neighbor_y, neighbor_z);

            self.update_neighbor_chunk(neighbor_chunk_pos, neighbor_chunked_pos, block);
        }

        // XYZ diagonal neighbors (if we're at all three borders)
        if border_sign_x != 0 && border_sign_y != 0 && border_sign_z != 0 {
            let neighbor_chunk_pos = ChunkPos {
                x: chunk_pos.x + border_sign_x,
                y: chunk_pos.y + border_sign_y,
                z: chunk_pos.z + border_sign_z,
            };

            let neighbor_x = if border_sign_x < 0 { CHUNKP_S1 - 1 } else { 0 };
            let neighbor_y = if border_sign_y < 0 { CHUNKP_S1 - 1 } else { 0 };
            let neighbor_z = if border_sign_z < 0 { CHUNKP_S1 - 1 } else { 0 };
            let neighbor_chunked_pos = (neighbor_x, neighbor_y, neighbor_z);

            self.update_neighbor_chunk(neighbor_chunk_pos, neighbor_chunked_pos, block);
        }
    }

    // Helper method to update a neighboring chunk
    fn update_neighbor_chunk(
        &self,
        neighbor_chunk_pos: ChunkPos,
        neighbor_chunked_pos: ChunkedPos,
        block: Block,
    ) {
        // if neighbor_chunk_pos.x == 8 && neighbor_chunk_pos.y == 0 && neighbor_chunk_pos.z == 2 {
        //     println!(
        //         "Updating Neighbour at chunkedpos {:?} at Chunked pos {:?} ",
        //         neighbor_chunk_pos, neighbor_chunked_pos
        //     );
        // }
        if let Some(mut chunk) = self.chunks.get_mut(&neighbor_chunk_pos) {
            chunk.set_no_padding(neighbor_chunked_pos, block);
        } else {
            let mut new_chunk = TrackedChunk::new();
            new_chunk.set_no_padding(neighbor_chunked_pos, block);
            self.chunks.insert(neighbor_chunk_pos, new_chunk);
        }

        self.mark_change_single(neighbor_chunk_pos);
    }

    pub fn raycast(&self, start: Vec3, dir: Vec3, dist: f32) -> Option<BlockRayCastHit> {
        let mut pos = BlockPos {
            x: start.x.floor() as i32,
            y: start.y.floor() as i32,
            z: start.z.floor() as i32,
        };
        let mut last_pos;
        let sx = dir.x.signum() as i32;
        let sy = dir.y.signum() as i32;
        let sz = dir.z.signum() as i32;
        if sx == 0 && sy == 0 && sz == 0 {
            return None;
        }
        let next_x = (pos.x + sx.max(0)) as f32;
        let next_y = (pos.y + sy.max(0)) as f32;
        let next_z = (pos.z + sz.max(0)) as f32;
        let mut t_max_x = (next_x - start.x) / dir.x;
        let mut t_max_y = (next_y - start.y) / dir.y;
        let mut t_max_z = (next_z - start.z) / dir.z;
        let slope_x = 1. / dir.x.abs();
        let slope_y = 1. / dir.y.abs();
        let slope_z = 1. / dir.z.abs();
        loop {
            last_pos = pos;
            if t_max_x < t_max_y {
                if t_max_x < t_max_z {
                    if t_max_x >= dist {
                        return None;
                    };
                    pos.x += sx;
                    t_max_x += slope_x;
                } else {
                    if t_max_z >= dist {
                        return None;
                    };
                    pos.z += sz;
                    t_max_z += slope_z;
                }
            } else if t_max_y < t_max_z {
                if t_max_y >= dist {
                    return None;
                };
                pos.y += sy;
                t_max_y += slope_y;
            } else {
                if t_max_z >= dist {
                    return None;
                };
                pos.z += sz;
                t_max_z += slope_z;
            }
            if self.get_block_safe(pos).is_targetable() {
                return Some(BlockRayCastHit {
                    pos,
                    normal: Vec3 {
                        x: (last_pos.x - pos.x) as f32,
                        y: (last_pos.y - pos.y) as f32,
                        z: (last_pos.z - pos.z) as f32,
                    },
                });
            }
        }
    }
}
