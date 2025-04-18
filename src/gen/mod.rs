mod debug_gen;
mod earth_gen;
mod terrain_gen;

pub use terrain_gen::setup_gen_thread;

use crate::Block;
use std::ops::Range;

type Soils = Vec<([Range<f32>; 2], Block)>;
