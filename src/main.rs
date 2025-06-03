use bevy::{input::mouse::AccumulatedMouseMotion, prelude::*};
use ray_travel::RayTraveler;
use std::f32::consts::PI;
use terrain::{ChunkBlocks, ChunksIndex, TerrainLoader, TerrainPlugin};

mod octahedron;
mod ray_travel;
mod terrain;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TerrainPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, (move_camera, pointing_at_block))
        .run();
}

fn setup(mut commands: Commands) {
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.8, 0.9, 1.0),
        brightness: 1000.0,
        ..default()
    });
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 100.0f32.to_radians(),
            ..default()
        }),
        Transform::from_xyz(20.0, 20.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        TerrainLoader::new(64.0, 20.0),
    ));
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(2.5, 5.0, 1.8).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn pointing_at_block(
    camera: Single<&Transform, With<Camera3d>>,
    blocks: Query<&ChunkBlocks>,
    terrain: Res<ChunksIndex>,
    mut gizmos: Gizmos,
) {
    let ray = camera.rotation * -Dir3::Z;
    let traveler = RayTraveler::new(camera.translation, ray, 16.0);
    for block in traveler {
        if terrain.get(blocks, block) == Some(true) {
            gizmos.cuboid(
                Transform {
                    translation: block.as_vec3() + Vec3::ONE / 2.0 - 0.01 * ray,
                    rotation: default(),
                    scale: Vec3::ONE,
                },
                Color::BLACK,
            );
            break;
        }
    }
}

fn move_camera(
    mut camera: Single<&mut Transform, With<Camera3d>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<AccumulatedMouseMotion>,
    time: Res<Time>,
) {
    const ROTATION_SENSITIVITY: f32 = 0.2;
    const TRANSLATION_SENSITIVITY: f32 = 20.0;

    let (yaw, pitch, _) = camera.rotation.to_euler(default());
    let aligned = Quat::from_euler(EulerRot::default(), yaw, 0.0, 0.0);
    let mut dir = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyE) {
        dir -= aligned.mul_vec3(Vec3::Z);
    }
    if keys.pressed(KeyCode::KeyD) {
        dir += aligned.mul_vec3(Vec3::Z);
    }
    if keys.pressed(KeyCode::KeyF) {
        dir += aligned.mul_vec3(Vec3::X);
    }
    if keys.pressed(KeyCode::KeyS) {
        dir -= aligned.mul_vec3(Vec3::X);
    }
    if keys.pressed(KeyCode::Space) {
        dir += Vec3::Y;
    }
    if keys.pressed(KeyCode::KeyZ) {
        dir -= Vec3::Y;
    }
    let delta = ROTATION_SENSITIVITY * time.delta_secs() * mouse.delta;
    let pitch = pitch - delta.y;
    let yaw = yaw - delta.x;

    camera.translation += TRANSLATION_SENSITIVITY * time.delta_secs() * dir.normalize_or_zero();
    camera.rotation = Quat::from_euler(
        EulerRot::default(),
        yaw.rem_euclid(2.0 * PI),
        pitch.clamp(-PI / 2.0, PI / 2.0),
        0.0,
    );
}
