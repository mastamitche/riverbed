pub mod action_mapping;

use action_mapping::*;
use bevy::app::Plugin;

pub struct ActionMappingPlugin;

impl Plugin for ActionMappingPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins(InputControllerPlugin)
            //b
            ;
    }
}
