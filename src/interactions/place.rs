use std::default;

use crate::{
    controls::action_mapping::{ActionState, GameAction},
    render::draw_chunks::BuildingState,
    setup::Block,
    world::{pos3d::Pos3d, VoxelWorld, CHUNK_S1},
};
use bevy::prelude::*;

#[derive(Event, Debug)]
pub struct PlaceBlockEvent {
    pub pos: Pos3d<1>,
    pub block: Block,
    pub destination: PlaceDestination,
}
#[derive(Default, Debug)]
pub enum PlaceDestination {
    #[default]
    World,
    Builder,
}

pub fn read_general_event(
    action_state: Res<ActionState>,
    building_state: Res<BuildingState>,
    mut place_events: EventWriter<PlaceBlockEvent>,
) {
    if action_state.just_released(GameAction::PrimaryAction) {
        if let Some(pos) = building_state.current_position {
            let p: Pos3d<1> = Pos3d {
                x: (pos.x * 8.) as i32,
                y: (pos.y * 8.) as i32,
                z: (pos.z * 8.) as i32,
            };
            place_events.write(PlaceBlockEvent {
                pos: p,
                block: Block::AcaciaLeaves,
                destination: PlaceDestination::World,
            });
        }
    }
}
fn place_block(
    world: ResMut<VoxelWorld>,
    mut place_events: EventReader<PlaceBlockEvent>,
    mut building_state: ResMut<BuildingState>,
) {
    let was_empty = place_events.is_empty();
    for evt in place_events.read() {
        world.set_block(evt.pos, evt.block);
    }
    if was_empty == false {
        building_state.current_position = None;
    }
}

pub struct PlacePlugin;
impl Plugin for PlacePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app
            .add_event::<PlaceBlockEvent>()
            .add_systems(Update, (place_block, read_general_event).chain())
        //b
        ;
    }
}
