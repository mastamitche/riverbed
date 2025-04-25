use super::AgentState;
use super::{block_action::BlockActionPlugin, key_binds::KeyBinds};
use crate::world::{BlockPos, BlockRayCastHit, Realm, VoxelWorld};
use crate::{world::RenderDistance, Block};
use avian3d::prelude::{Collider, ComputedMass, Friction, LinearVelocity, LockedAxes, RigidBody};
use bevy::{math::Vec3, prelude::*};
use leafwing_input_manager::prelude::*;

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
        app.insert_resource(KeyBinds::default())
            .insert_resource(PlayerUnlockTimer::default())
            .insert_resource(StepUpSettings {
                max_step_height: 1.0,
                smoothing_factor: 0.15,
                gravity_multiplier: 2.0,
            })
            .add_plugins(BlockActionPlugin)
            .add_plugins(InputManagerPlugin::<Dir>::default())
            .add_plugins(InputManagerPlugin::<Action>::default())
            .add_plugins(InputManagerPlugin::<DevCommand>::default())
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

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum Dir {
    Front,
    Back,
    Up,
    Left,
    Down,
    Right,
}

impl From<Dir> for Vec3 {
    fn from(dir: Dir) -> Self {
        match dir {
            Dir::Front => Vec3::new(0., 0., 1.),
            Dir::Back => Vec3::new(0., 0., -1.),
            Dir::Up => Vec3::new(0., 1., 0.),
            Dir::Down => Vec3::new(0., -1., 0.),
            Dir::Right => Vec3::new(1., 0., 0.),
            Dir::Left => Vec3::new(-1., 0., 0.),
        }
    }
}

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Debug, Hash, Reflect)]
pub enum Action {
    Hit,
    Modify,
}

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Debug, Hash, Reflect)]
pub enum DevCommand {
    ToggleFly,
}
#[derive(Resource)]
pub struct StepUpSettings {
    pub max_step_height: f32,
    pub smoothing_factor: f32,
    pub gravity_multiplier: f32,
}

pub fn check_unlock_player(
    mut commands: Commands,
    mut timer: ResMut<PlayerUnlockTimer>,
    time: Res<Time>,
    player_query: Query<Entity, With<PlayerControlled>>,
) {
    timer.timer.tick(time.delta());
    if timer.timer.finished() {
        info!("World loaded, unlocking player");
        let entity = player_query.single();
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

pub fn spawn_player(
    mut commands: Commands,
    key_binds: Res<KeyBinds>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let realm = Realm::Overworld;

    let rd = RenderDistance(5);
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
        .insert(SpatialListener::new(0.3))
        .insert(InputManagerBundle::<Dir> {
            action_state: ActionState::default(),
            input_map: InputMap::new([
                (Dir::Front, key_binds.forward),
                (Dir::Left, key_binds.left),
                (Dir::Back, key_binds.backward),
                (Dir::Right, key_binds.right),
            ]),
        })
        .insert(InputManagerBundle::<Action> {
            action_state: ActionState::default(),
            input_map: InputMap::new([
                (Action::Hit, key_binds.hit),
                (Action::Modify, key_binds.modify),
            ]),
        })
        .insert(InputManagerBundle::<DevCommand> {
            action_state: ActionState::default(),
            input_map: InputMap::new([(DevCommand::ToggleFly, key_binds.toggle_fly)]),
        })
        .add_child(player_model);
}

pub fn toggle_free_fly(
    mut player_query: Query<(&ActionState<DevCommand>), With<PlayerControlled>>,
    state: Res<State<AgentState>>,
    mut next_state: ResMut<NextState<AgentState>>,
) {
    let action_state = player_query.single_mut();
    if action_state.just_released(&DevCommand::ToggleFly) {
        match state.get() {
            AgentState::Normal => next_state.set(AgentState::FreeFly),
            AgentState::FreeFly => next_state.set(AgentState::Normal),
        }
    }
}

pub fn move_player(
    mut player_query: Query<
        (&Transform, &mut LinearVelocity, &ActionState<Dir>),
        With<PlayerControlled>,
    >,
    cam_query: Query<&Transform, With<Camera>>,
    world: Res<VoxelWorld>,
    time: Res<Time>,
) {
    let cam_transform = if let Ok(ct) = cam_query.get_single() {
        *ct
    } else {
        Transform::default()
    };

    let (transform, mut velocity, action_state) = player_query.single_mut();
    let player_pos = transform.translation;
    let delta_secs = time.delta_secs();

    // Part 1: Handle player input movement
    let mut movement = Vec3::default();
    for action in action_state.get_pressed() {
        movement += Vec3::from(action);
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

fn should_player_update(
    player_query: Query<(&Transform, &LinearVelocity), With<PlayerControlled>>,
    states: Res<State<AgentState>>,
) -> bool {
    !player_query.is_empty() && *states.get() == AgentState::Normal
}
