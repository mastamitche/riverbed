pub mod ActionMapping;

use bevy::app::Plugin;
use ActionMapping::*;

pub struct ActionMappingPlugin;

impl Plugin for ActionMappingPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins(InputControllerPlugin);
    }
}
