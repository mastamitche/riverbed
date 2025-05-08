use bevy::{
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
};
use std::collections::HashMap;

// Define our game actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameAction {
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
    Jump,
    ToggleFreeFly,
    PrimaryAction,
    SecondaryAction,
    MiddleAction,
}

// Input source types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputSource {
    Keyboard(KeyCode),
    GamepadButton(GamepadButton),
    GamepadAxis(GamepadAxis),
    MouseButton(MouseButton),
    MouseMotion(MouseAxis),
    MouseWheel(MouseWheelAxis),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseAxis {
    X,
    Y,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseWheelAxis {
    X,
    Y,
}

// Action state with strength (for analog inputs)
#[derive(Default, Debug, Clone, Resource)]
pub struct ActionState {
    actions: HashMap<GameAction, f32>,
    previous_actions: HashMap<GameAction, f32>,
    deltas: HashMap<GameAction, f32>,
}

impl ActionState {
    pub fn pressed(&self, action: GameAction) -> bool {
        self.value(action) > 0.5
    }

    pub fn just_pressed(&self, action: GameAction) -> bool {
        self.pressed(action) && !self.was_pressed(action)
    }

    pub fn just_released(&self, action: GameAction) -> bool {
        !self.pressed(action) && self.was_pressed(action)
    }

    pub fn value(&self, action: GameAction) -> f32 {
        *self.actions.get(&action).unwrap_or(&0.0)
    }

    fn was_pressed(&self, action: GameAction) -> bool {
        self.previous_value(action) > 0.5
    }

    fn previous_value(&self, action: GameAction) -> f32 {
        *self.previous_actions.get(&action).unwrap_or(&0.0)
    }

    pub fn reset_deltas(&mut self) {
        self.deltas.clear();
    }

    pub fn set_delta(&mut self, action: GameAction, value: f32) {
        self.deltas.insert(action, value);
    }
}

// Input mapping configuration
#[derive(Debug, Clone, Resource)]
pub struct InputMap {
    action_mappings: HashMap<InputSource, GameAction>,
}

impl Default for InputMap {
    fn default() -> Self {
        let mut map = Self {
            action_mappings: HashMap::new(),
        };
        // Movement controls
        map.bind(
            InputSource::Keyboard(KeyCode::KeyW),
            GameAction::MoveBackward,
        );
        map.bind(
            InputSource::Keyboard(KeyCode::KeyS),
            GameAction::MoveForward,
        );
        map.bind(InputSource::Keyboard(KeyCode::KeyA), GameAction::MoveLeft);
        map.bind(InputSource::Keyboard(KeyCode::KeyD), GameAction::MoveRight);
        map.bind(InputSource::Keyboard(KeyCode::Space), GameAction::Jump);
        map.bind(
            InputSource::Keyboard(KeyCode::F1),
            GameAction::ToggleFreeFly,
        );

        // Gamepad controls
        map.bind(
            InputSource::GamepadAxis(GamepadAxis::LeftStickY),
            GameAction::MoveForward,
        );
        map.bind(
            InputSource::GamepadAxis(GamepadAxis::LeftStickX),
            GameAction::MoveRight,
        );
        map.bind(
            InputSource::GamepadButton(GamepadButton::South),
            GameAction::Jump,
        );
        map.bind(
            InputSource::MouseButton(MouseButton::Left),
            GameAction::PrimaryAction,
        );
        map.bind(
            InputSource::MouseButton(MouseButton::Right),
            GameAction::SecondaryAction,
        );
        map.bind(
            InputSource::MouseButton(MouseButton::Middle),
            GameAction::MiddleAction,
        );

        map
    }
}

impl InputMap {
    pub fn bind(&mut self, input: InputSource, action: GameAction) {
        self.action_mappings.insert(input, action);
    }

    pub fn unbind(&mut self, input: InputSource) {
        self.action_mappings.remove(&input);
    }

    pub fn get_action(&self, input: &InputSource) -> Option<GameAction> {
        self.action_mappings.get(input).copied()
    }
}

// Bevy plugin for the input system
pub struct InputControllerPlugin;

impl Plugin for InputControllerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputMap>()
            .init_resource::<ActionState>()
            .add_systems(Update, update_action_state)
            // .add_systems(Update, (debug_input_system))
            //break
            ;
    }
}

fn update_action_state(
    mut action_state: ResMut<ActionState>,
    input_map: Res<InputMap>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    gamepads: Query<(Entity, &Gamepad)>,
) {
    // Store previous frame's actions
    action_state.previous_actions = action_state.actions.clone();

    // Reset deltas for this frame
    action_state.reset_deltas();

    // Create a temporary map to collect maximum values for each action
    let mut action_values: HashMap<GameAction, f32> = HashMap::new();

    // Process mouse motion events
    let mut mouse_delta_x = 0.0;
    let mut mouse_delta_y = 0.0;
    for event in mouse_motion_events.read() {
        mouse_delta_x += event.delta.x;
        mouse_delta_y += event.delta.y;
    }

    // Process mouse wheel events
    let mut wheel_delta_x = 0.0;
    let mut wheel_delta_y = 0.0;
    for event in mouse_wheel_events.read() {
        wheel_delta_x += event.x;
        wheel_delta_y += event.y;
    }

    // Process all inputs
    for (input, action) in input_map.action_mappings.iter() {
        let value = match input {
            InputSource::Keyboard(key) => {
                if keyboard_input.pressed(*key) {
                    1.0
                } else {
                    0.0
                }
            }
            InputSource::MouseButton(button) => {
                if mouse_button_input.pressed(*button) {
                    1.0
                } else {
                    0.0
                }
            }
            InputSource::MouseMotion(axis) => {
                match axis {
                    MouseAxis::X => {
                        action_state.set_delta(*action, mouse_delta_x);
                        mouse_delta_x.abs().min(1.0) // Normalize for pressed state
                    }
                    MouseAxis::Y => {
                        action_state.set_delta(*action, mouse_delta_y);
                        mouse_delta_y.abs().min(1.0) // Normalize for pressed state
                    }
                }
            }
            InputSource::MouseWheel(axis) => match axis {
                MouseWheelAxis::X => {
                    action_state.set_delta(*action, wheel_delta_x);
                    wheel_delta_x.abs().min(1.0)
                }
                MouseWheelAxis::Y => {
                    action_state.set_delta(*action, wheel_delta_y);
                    wheel_delta_y.abs().min(1.0)
                }
            },
            InputSource::GamepadButton(button_type) => {
                let mut button_pressed = false;
                for (_, gamepad) in gamepads.iter() {
                    if gamepad.pressed(*button_type) {
                        button_pressed = true;
                        break;
                    }
                }
                if button_pressed {
                    1.0
                } else {
                    0.0
                }
            }
            InputSource::GamepadAxis(axis_type) => {
                let mut max_value = 0.0;
                for (_, gamepad) in gamepads.iter() {
                    if let Some(value) = gamepad.get(*axis_type) {
                        max_value = f32::max(max_value, value.abs());
                    }
                }
                max_value
            }
        };

        // Keep the maximum value for each action
        let current = action_values.entry(*action).or_insert(0.0);
        *current = f32::max(*current, value);
    }

    let cloned_values = action_values.clone();
    // Update action state with collected values
    for (action, value) in action_values {
        action_state.actions.insert(action, value);
    }
    // For actions that were in previous frame but not in this frame,
    // set them to 0.0 to properly handle releases
    for action in action_state.previous_actions.clone().keys() {
        if !cloned_values.contains_key(action) {
            action_state.actions.insert(*action, 0.0);
        }
    }
}

// Debug system to print active inputs (can be removed in production)
fn debug_input_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    action_state: Res<ActionState>,
    mut last_debug: Local<f64>,
    time: Res<Time>,
) {
    // Only print every second to avoid spamming the console
    if time.elapsed_secs_f64() - *last_debug > 1.0 {
        for (action, value) in action_state.actions.iter() {
            if *value != 0.0 {
                println!("Action {:?}: {}", action, value);
            }
        }
        *last_debug = time.elapsed_secs_f64();
    }
}
