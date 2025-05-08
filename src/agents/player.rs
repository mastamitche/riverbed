use super::block_action::BlockActionPlugin;
use super::AgentState;
use crate::controls::action_mapping::{ActionState, GameAction};
use crate::world::{BlockPos, BlockRayCastHit, Realm, VoxelWorld};
use crate::{world::RenderDistance, Block};
use avian3d::prelude::{Collider, ComputedMass, Friction, LinearVelocity, LockedAxes, RigidBody};
use bevy::{math::Vec3, prelude::*};

const WALK_SPEED: f32 = 200.;

const SPAWN: Vec3 = Vec3 {
    x: 500.,
    y: 8.,
    z: 500.,
};

pub struct PlayerPlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, SystemSet)]
pub struct PlayerSpawn;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PlayerUnlockTimer::default())
            .add_plugins(BlockActionPlugin)
            .add_systems(
                Startup,
                (spawn_player, apply_deferred).chain().in_set(PlayerSpawn),
            )
            .add_systems(Update, toggle_free_fly)
            .add_systems(
                Update,
                check_unlock_player
                    .run_if(resource_exists::<PlayerUnlockTimer>.and(in_state(AgentState::Normal))),
            )
            .add_systems(Update, (move_player).run_if(should_player_update));
    }
}
#[derive(Resource)]
pub struct PlayerUnlockTimer {
    timer: Timer,
}

impl Default for PlayerUnlockTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(3.0, TimerMode::Once),
        }
    }
}

#[derive(Component)]
pub struct PlayerControlled;

#[derive(Component)]
pub struct TargetBlock(pub Option<BlockRayCastHit>);

pub fn check_unlock_player(
    mut commands: Commands,
    mut timer: ResMut<PlayerUnlockTimer>,
    time: Res<Time>,
    player_query: Query<Entity, With<PlayerControlled>>,
) {
    timer.timer.tick(time.delta());
    if timer.timer.finished() {
        info!("World loaded, unlocking player");
        if let Ok(entity) = player_query.single() {
            commands.entity(entity).insert((
                RigidBody::Dynamic,
                Collider::sphere(0.5),
                ComputedMass::new(80.0),
                Friction::new(0.4),
                LinearVelocity(Vec3::new(0., 0., 0.)),
                LockedAxes::new()
                    .lock_rotation_y()
                    .lock_rotation_x()
                    .lock_rotation_z(),
            ));
            commands.remove_resource::<PlayerUnlockTimer>();
        }
    }
}

pub fn spawn_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let realm = Realm::Overworld;

    let rd = RenderDistance(7);
    let player_model = commands
        .spawn((
            Transform::from_xyz(0., 0.5, 0.),
            Mesh3d(meshes.add(Capsule3d::new(0.5, 1.0))),
            MeshMaterial3d(materials.add(Color::srgb_u8(255, 0, 0))),
        ))
        .id();
    commands
        .spawn((
            Transform {
                translation: SPAWN,
                ..default()
            },
            Visibility::default(),
            realm,
            rd,
            TargetBlock(None),
            PlayerControlled,
        ))
        .observe(
            |trigger: Trigger<Pointer<Move>>, mut transforms: Query<&mut Transform>| {
                let mv = trigger.event();
            },
        )
        .insert(SpatialListener::new(0.3))
        .add_child(player_model);
}

pub fn toggle_free_fly(
    action_state: Res<ActionState>,
    state: Res<State<AgentState>>,
    mut next_state: ResMut<NextState<AgentState>>,
) {
    if action_state.just_released(GameAction::ToggleFreeFly) {
        match state.get() {
            AgentState::Normal => next_state.set(AgentState::FreeFly),
            AgentState::FreeFly => next_state.set(AgentState::Normal),
        }
    }
}

pub fn move_player(
    action_state: Res<ActionState>,
    mut player_query: Query<(&Transform, &mut LinearVelocity), With<PlayerControlled>>,
    cam_query: Query<&Transform, With<Camera>>,
    world: Res<VoxelWorld>,
    time: Res<Time>,
) {
    let cam_transform = if let Ok(ct) = cam_query.single() {
        *ct
    } else {
        Transform::default()
    };

    if let Ok((transform, mut velocity)) = player_query.single_mut() {
        let player_pos = transform.translation;
        let delta_secs = time.delta_secs();

        // Part 1: Handle player input movement
        let mut movement = Vec3::default();

        // Check each movement direction and add to the movement vector
        if action_state.pressed(GameAction::MoveForward) {
            movement.z -= 1.0;
        }
        if action_state.pressed(GameAction::MoveBackward) {
            movement.z += 1.0;
        }
        if action_state.pressed(GameAction::MoveLeft) {
            movement.x -= 1.0;
        }
        if action_state.pressed(GameAction::MoveRight) {
            movement.x += 1.0;
        }

        if movement.length_squared() > 0. {
            movement = movement.normalize();
            movement = Vec3::Y.cross(*cam_transform.right()) * movement.z
                + cam_transform.right() * movement.x
                + movement.y * Vec3::Y;

            // Only set horizontal movement from input
            let horizontal_speed = WALK_SPEED * delta_secs;
            velocity.0.x = movement.x * horizontal_speed;
            velocity.0.z = movement.z * horizontal_speed;
        } else {
            velocity.0.x = 0.0;
            velocity.0.z = 0.0;
        }

        // Part 2: Handle physics (gravity and step-up)
        // Check if player is on ground
        let below_pos = player_pos + Vec3::new(0.0, -1.05, 0.0);
        let block_below = world.get_block_safe(BlockPos::from((below_pos, Realm::Overworld)));
        let on_ground = block_below != Block::Air;

        // Handle jumping
        if action_state.just_pressed(GameAction::Jump) && on_ground {
            velocity.0.y = 8.0; // Jump velocity
        }

        // Simple stair-stepping logic - only if we're moving horizontally
        if on_ground && (velocity.0.x.abs() + velocity.0.z.abs() > 0.1) {
            let movement_dir = Vec3::new(velocity.0.x, 0.0, velocity.0.z).normalize();
            let step_pos = player_pos + movement_dir * 0.8 + Vec3::new(0.0, 0.5, 0.0);
            let step_block = world.get_block_safe(BlockPos::from((step_pos, Realm::Overworld)));

            if step_block == Block::Air
                && world.get_block_safe(BlockPos::from((
                    step_pos + Vec3::new(0.0, -0.5, 0.0),
                    Realm::Overworld,
                ))) != Block::Air
            {
                // Found a step, apply gentle upward velocity
                velocity.0.y = 5.0;
            }
        }

        // Apply gravity when in air
        if !on_ground {
            // Cap falling speed to prevent excessive acceleration
            velocity.0.y = (velocity.0.y - 20.0 * delta_secs).max(-20.0);
        } else if velocity.0.y < 0.0 {
            // Stop falling when on ground
            velocity.0.y = 0.0;
        }
    }
}

fn should_player_update(
    player_query: Query<(&Transform, &LinearVelocity), With<PlayerControlled>>,
    states: Res<State<AgentState>>,
) -> bool {
    !player_query.is_empty() && *states.get() == AgentState::Normal
}
