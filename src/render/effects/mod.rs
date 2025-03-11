use bevy::app::Plugin;
mod normal_ao;

pub use normal_ao::NormalAoSettings;

pub struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins(normal_ao::NormalAoPlugin);
    }
}
