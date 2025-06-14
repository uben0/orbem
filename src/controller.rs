use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, WindowFocused},
};

#[derive(SystemSet, Clone, PartialEq, Eq, Debug, Hash)]
pub struct ControllerFetch;

pub struct ControllerPlugin;
#[derive(Resource)]
pub struct ControllerState {
    pub linear_3d: Vec3,
    pub linear_2d: Vec3,
    pub jump: bool,
    pub sneak: bool,
    pub mouse: Vec2,
    pub sprint: bool,
}
// #[derive(Event)]
// pub enum ControllerEvent {
//     Rotation(Vec2),
// }

impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ControllerState {
            linear_3d: Vec3::ZERO,
            linear_2d: Vec3::ZERO,
            jump: false,
            sneak: false,
            sprint: false,
            mouse: Vec2::ZERO,
        })
        // .add_event::<ControllerEvent>()
        .add_systems(
            Update,
            (window_focus, keyboard_input, mouse_input).in_set(ControllerFetch),
        )
        .add_systems(Startup, setup);
    }
}

fn setup(mut window: Single<&mut Window>) {
    window.cursor_options.grab_mode = CursorGrabMode::Locked;
    window.cursor_options.visible = false;
}

fn window_focus(mut events: EventReader<WindowFocused>, mut windows: Query<&mut Window>) {
    for event in events.read() {
        let mut window = windows.get_mut(event.window).unwrap();
        if event.focused {
            window.cursor_options.grab_mode = CursorGrabMode::Locked;
            window.cursor_options.visible = false;
        } else {
            window.cursor_options.grab_mode = CursorGrabMode::None;
            window.cursor_options.visible = true;
        }
    }
}

pub fn keyboard_input(keys: Res<ButtonInput<KeyCode>>, mut controller: ResMut<ControllerState>) {
    let mut dir = Vec3::ZERO;
    controller.jump = keys.pressed(KeyCode::Space);
    controller.sneak = keys.pressed(KeyCode::KeyZ);
    controller.sprint = keys.pressed(KeyCode::KeyA);
    if keys.pressed(KeyCode::KeyE) {
        dir -= Vec3::Z;
    }
    if keys.pressed(KeyCode::KeyD) {
        dir += Vec3::Z;
    }
    if keys.pressed(KeyCode::KeyF) {
        dir += Vec3::X;
    }
    if keys.pressed(KeyCode::KeyS) {
        dir -= Vec3::X;
    }
    controller.linear_2d = dir.normalize_or_zero();
    if controller.jump {
        dir += Vec3::Y;
    }
    if controller.sneak {
        dir -= Vec3::Y;
    }
    controller.linear_3d = dir.normalize_or_zero();
}

pub fn mouse_input(mut mouse: EventReader<MouseMotion>, mut controller: ResMut<ControllerState>) {
    controller.mouse = mouse.read().map(|e| e.delta).sum();
}
