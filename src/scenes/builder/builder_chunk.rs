use bevy::ecs::resource::Resource;

use crate::world::TrackedChunk;

#[derive(Resource)]
pub struct BuilderChunk {
    pub tracked_chunk: TrackedChunk,
}
impl Default for BuilderChunk {
    fn default() -> Self {
        Self {
            tracked_chunk: TrackedChunk::new(),
        }
    }
}
