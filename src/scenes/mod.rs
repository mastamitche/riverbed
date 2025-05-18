use bevy::prelude::*;

pub mod builder;

pub struct ScenesPlugin;
impl Plugin for ScenesPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app
            .add_plugins(builder::BuilderPlugin)
        //b
        ;
    }
}
