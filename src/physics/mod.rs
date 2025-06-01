use avian3d::prelude::*;
use bevy::prelude::*;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app
            .insert_resource(Gravity(Vec3::NEG_Y * 9.81))
            .add_plugins((
                PhysicsPlugins::default(),
                // PhysicsDebugPlugin::default()
            ))
            // .insert_gizmo_config(
            //     PhysicsGizmos {
            //         aabb_color: None,
            //         ..default()
            //     },
            //     GizmoConfig::default(),
            // )
            //b
            ;
    }
}
