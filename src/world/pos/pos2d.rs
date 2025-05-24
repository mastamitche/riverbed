use super::{chunked, pos3d::Pos3d, unchunked, BlockPos, ChunkPos, CHUNK_S1I};
use crate::world::{CHUNK_S1, Y_CHUNKS};
use bevy::prelude::Vec3;
use std::ops::BitXor;

#[derive(Clone, Copy, Eq, PartialEq, Default, Debug, Hash)]
pub struct Pos2d<const U: usize> {
    pub x: i32,
    pub z: i32,
}

const K: usize = 0x9E3779B9; // Original 64-bit version

impl<const U: usize> Pos2d<U> {
    pub fn dist(&self, other: Pos2d<U>) -> i32 {
        (self.x - other.x).abs().max((self.z - other.z).abs())
    }
    fn _prng(&self, seed: usize) -> usize {
        (seed)
            .rotate_left(5)
            .bitxor(self.x as usize)
            .wrapping_mul(K)
            .rotate_left(5)
            .bitxor(self.z as usize)
            .wrapping_mul(K)
    }

    pub fn prng(&self, seed: i32) -> usize {
        let n = self._prng(seed as usize);
        self._prng(n)
    }
}

impl<const U: usize> From<Pos3d<U>> for Pos2d<U> {
    fn from(pos3d: Pos3d<U>) -> Self {
        Pos2d {
            x: pos3d.x,
            z: pos3d.z,
        }
    }
}

pub type BlockPos2d = Pos2d<1>;
pub type ColPos = Pos2d<CHUNK_S1>;
pub type ColedPos = (usize, usize);

impl From<Vec3> for BlockPos2d {
    fn from(pos: Vec3) -> Self {
        BlockPos2d {
            x: (pos.x * 8.0).floor() as i32,
            z: (pos.z * 8.0).floor() as i32,
        }
    }
}

impl From<(ColPos, ColedPos)> for BlockPos2d {
    fn from((chunk_pos, (dx, dz)): (ColPos, ColedPos)) -> Self {
        BlockPos2d {
            x: unchunked(chunk_pos.x, dx),
            z: unchunked(chunk_pos.z, dz),
        }
    }
}

impl From<BlockPos2d> for (ColPos, ColedPos) {
    fn from(block_pos: BlockPos2d) -> Self {
        let (cx, dx) = chunked(block_pos.x);
        let (cz, dz) = chunked(block_pos.z);
        (ColPos { x: cx, z: cz }, (dx, dz))
    }
}

impl From<BlockPos2d> for ColPos {
    fn from(block_pos2d: BlockPos2d) -> Self {
        let cx = block_pos2d.x / CHUNK_S1I;
        let cz = block_pos2d.z / CHUNK_S1I;
        ColPos { x: cx, z: cz }
    }
}

impl From<BlockPos> for ColPos {
    fn from(block_pos: BlockPos) -> Self {
        let cx = block_pos.x / CHUNK_S1I;
        let cz = block_pos.z / CHUNK_S1I;
        ColPos { x: cx, z: cz }
    }
}

impl From<Vec3> for ColPos {
    fn from(pos: Vec3) -> Self {
        let scaled_x = (pos.x * 8.0).floor() as i32;
        let scaled_z = (pos.z * 8.0).floor() as i32;

        ColPos {
            x: scaled_x / CHUNK_S1I,
            z: scaled_z / CHUNK_S1I,
        }
    }
}

pub fn chunks_in_col(col_pos: &ColPos) -> [ChunkPos; Y_CHUNKS] {
    std::array::from_fn(|y| ChunkPos {
        x: col_pos.x,
        y: y as i32,
        z: col_pos.z,
    })
}
