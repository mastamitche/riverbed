use crate::{
    controls::action_mapping::{ActionState, GameAction},
    render::draw_chunks::BuildingState,
    setup::Block,
    world::{pos3d::Pos3d, Realm, VoxelWorld, CHUNK_S1},
};
use bevy::prelude::*;

pub fn place_block(
    action_state: Res<ActionState>,
    building_state: Res<BuildingState>,
    mut world: ResMut<VoxelWorld>,
) {
    if action_state.just_released(GameAction::PrimaryAction) {
        if let Some(pos) = building_state.current_position {
            let p: Pos3d<1> = Pos3d {
                x: (pos.x * 8.) as i32,
                y: (pos.y * 8.) as i32,
                z: (pos.z * 8.) as i32,
                realm: Realm::Overworld,
            };
            world.set_block(p, Block::AcaciaLeaves);
        }
    }
}

pub struct PlacePlugin;
impl Plugin for PlacePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(Update, place_block);
    }
}
