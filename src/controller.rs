use bevy::{input::mouse::MouseMotion, prelude::*};

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
            mouse: Vec2::ZERO,
        })
        // .add_event::<ControllerEvent>()
        .add_systems(
            Update,
            (keyboard_input, mouse_input).in_set(ControllerFetch),
        );
    }
}

pub fn keyboard_input(keys: Res<ButtonInput<KeyCode>>, mut controller: ResMut<ControllerState>) {
    let mut dir = Vec3::ZERO;
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
    if keys.pressed(KeyCode::Space) {
        controller.jump = true;
        dir += Vec3::Y;
    }
    if keys.pressed(KeyCode::KeyZ) {
        controller.sneak = true;
        dir -= Vec3::Y;
    }
    controller.linear_3d = dir.normalize_or_zero();
}

pub fn mouse_input(mut mouse: EventReader<MouseMotion>, mut controller: ResMut<ControllerState>) {
    controller.mouse = mouse.read().map(|e| e.delta).sum();
}
