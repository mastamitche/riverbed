use bevy::prelude::*;

mod free_fly;
mod player;
pub use free_fly::*;
pub use player::*;

pub struct AgentsPlugin;

impl Plugin for AgentsPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AgentState>()
            .add_plugins(PlayerPlugin)
            .add_plugins(FreeFlyPlugin);
    }
}
#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AgentState {
    #[default]
    Normal,
    FreeFly,
}
