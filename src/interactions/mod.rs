use bevy::prelude::*;
pub mod place;

use place::PlacePlugin;

pub struct PlayerInteractionsPlugin;

impl Plugin for PlayerInteractionsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(PlacePlugin)
            //b
        ;
    }
}
