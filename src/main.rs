mod controller;
mod octahedron;
mod ray_travel;
mod terrain;

use bevy::prelude::*;
use bevy_framepace::FramepacePlugin;
use controller::{ControllerFetch, ControllerPlugin, ControllerState};
use ray_travel::RayTraveler;
use std::f32::consts::PI;
use terrain::{ChunkBlocks, ChunksIndex, Modifications, Modify, TerrainLoader, TerrainPlugin};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            TerrainPlugin,
            ControllerPlugin,
            FramepacePlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                move_camera.after(ControllerFetch),
                pointed_block,
                current_chunk_highlight,
                pointed_block_show.after(pointed_block),
                block_place_or_remove.after(pointed_block),
            ),
        )
        .insert_resource(PointedBlock { at: None })
        .run();
}

fn setup(
    mut commands: Commands,
    // mut framepace: ResMut<FramepaceSettings>,
) {
    // window.present_mode = PresentMode::AutoNoVsync;
    // framepace.limiter = Limiter::from_framerate(120.0);
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

fn current_chunk_highlight(camera: Single<&Transform, With<Camera3d>>, mut gizmos: Gizmos) {
    let camera = camera.translation.round().as_ivec3();
    let (chunk, _) = terrain::global_to_local(camera);
    let center: Vec3A = terrain::chunk_center(chunk).into();
    let color = Color::srgb(0.3, 0.5, 0.7);
    let cells = UVec2::splat(32);
    let size = Vec2::splat(1.0);
    gizmos
        .grid(
            Isometry3d {
                rotation: default(),
                translation: center + terrain::CHUNK_WIDTH as f32 / 2.0 * -Vec3A::Z,
            },
            cells,
            size,
            color,
        )
        .outer_edges();
    gizmos
        .grid(
            Isometry3d {
                rotation: default(),
                translation: center + terrain::CHUNK_WIDTH as f32 / 2.0 * Vec3A::Z,
            },
            cells,
            size,
            color,
        )
        .outer_edges();
    gizmos
        .grid(
            Isometry3d {
                rotation: Quat::from_rotation_y(PI / 2.0),
                translation: center + terrain::CHUNK_WIDTH as f32 / 2.0 * -Vec3A::X,
            },
            cells,
            size,
            color,
        )
        .outer_edges();
    gizmos
        .grid(
            Isometry3d {
                rotation: Quat::from_rotation_y(PI / 2.0),
                translation: center + terrain::CHUNK_WIDTH as f32 / 2.0 * Vec3A::X,
            },
            cells,
            size,
            color,
        )
        .outer_edges();
}

fn block_place_or_remove(
    button: Res<ButtonInput<MouseButton>>,
    pointed: Res<PointedBlock>,
    mut modifications: ResMut<Modifications>,
) {
    if let Some((at, dir)) = pointed.at {
        if button.just_pressed(MouseButton::Left) {
            modifications.push(Modify::Remove { at });
        } else if button.just_pressed(MouseButton::Right) {
            modifications.push(Modify::Place { at: at + dir });
        }
    }
}

#[derive(Resource)]
struct PointedBlock {
    at: Option<(IVec3, IVec3)>,
}

fn pointed_block_show(
    camera: Single<&Transform, With<Camera3d>>,
    pointed: Res<PointedBlock>,
    mut gizmos: Gizmos,
) {
    if let Some((at, side)) = pointed.at {
        gizmos.cuboid(
            Transform {
                translation: at.as_vec3() + Vec3::ONE / 2.0 + 0.01 * (camera.rotation * Dir3::Z),
                rotation: default(),
                scale: Vec3::ONE,
            },
            Color::BLACK,
        );
        gizmos.cuboid(
            Transform {
                translation: (at + side).as_vec3() + Vec3::ONE / 2.0,
                rotation: default(),
                scale: Vec3::ONE * 0.8,
            },
            Color::WHITE,
        );
    }
}

fn pointed_block(
    camera: Single<&Transform, With<Camera3d>>,
    blocks: Query<&ChunkBlocks>,
    terrain: Res<ChunksIndex>,
    mut pointed: ResMut<PointedBlock>,
) {
    let ray = camera.rotation * -Dir3::Z;
    let traveler = RayTraveler::new(camera.translation, ray, 16.0);
    for (block, dir) in traveler {
        if let Some((chunk, local)) = terrain.global_to_local(block) {
            if let Ok(blocks) = blocks.get(chunk) {
                if blocks.get(local) {
                    pointed.at = Some((block, dir));
                    return;
                }
            }
        }
    }
    pointed.at = None;
}

fn move_camera(
    mut camera: Single<&mut Transform, With<Camera3d>>,
    controller_state: Res<ControllerState>,
    time: Res<Time>,
) {
    const ROTATION_SENSITIVITY: f32 = 0.2;
    const TRANSLATION_SENSITIVITY: f32 = 40.0;

    let (yaw, pitch, _) = camera.rotation.to_euler(default());
    let aligned = Quat::from_euler(EulerRot::default(), yaw, 0.0, 0.0);

    let delta =
        vec2(yaw, pitch) - ROTATION_SENSITIVITY * time.delta_secs() * controller_state.mouse;

    camera.translation +=
        aligned * TRANSLATION_SENSITIVITY * time.delta_secs() * controller_state.linear_3d;
    camera.rotation = Quat::from_euler(
        EulerRot::default(),
        delta.x.rem_euclid(2.0 * PI),
        delta.y.clamp(-PI / 2.0, PI / 2.0),
        0.0,
    );
}
