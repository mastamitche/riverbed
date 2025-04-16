pub mod pos2d;
pub mod pos3d;
use bevy::math::{I64Vec3, Vec3};
pub use pos2d::{BlockPos2d, ColPos, ColedPos};
pub use pos3d::{BlockPos, ChunkPos, ChunkedPos};

use super::{CHUNKP_S1I, CHUNK_S1I};
const CHUNK_S1F: f32 = CHUNK_S1I as f32;

pub fn chunked(x: i32) -> (i32, usize) {
    let r = x.rem_euclid(CHUNK_S1I);
    ((x - r) / CHUNK_S1I, r as usize)
}
pub fn un_padded_chunked(x: i32) -> (i32, usize) {
    let r = x.rem_euclid(CHUNKP_S1I);
    ((x - r) / CHUNKP_S1I, r as usize)
}

pub fn unchunked(cx: i32, dx: usize) -> i32 {
    cx * CHUNK_S1I + dx as i32
}

pub fn chunk_pos(pos: Vec3) -> I64Vec3 {
    I64Vec3::new(
        (pos.x / CHUNK_S1F).floor() as i64,
        (pos.y / CHUNK_S1F).floor() as i64,
        (pos.z / CHUNK_S1F).floor() as i64,
    )
}
