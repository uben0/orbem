mod controller;
mod octahedron;
mod ray_travel;
mod terrain;

use bevy::{math::NormedVectorSpace, prelude::*, render::view::RenderLayers};
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
                show_collider,
                move_collider,
                axis_overlay,
                apply_gravity,
            ),
        )
        .insert_gizmo_config(
            AxisOverlay,
            GizmoConfig {
                enabled: true,
                line: GizmoLineConfig {
                    width: 4.0,
                    ..default()
                },
                depth_bias: 0.0,
                render_layers: RenderLayers::layer(1),
            },
        )
        .insert_resource(PointedBlock { at: None })
        .run();
}

#[derive(GizmoConfigGroup, Default, Reflect)]
struct AxisOverlay;

#[derive(Component)]
struct Collider {
    size: Vec3,
}

#[derive(Component)]
struct Velocity {
    linear: Vec3,
}

#[derive(Component)]
struct MainCamera;

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
        AmbientLight {
            color: Color::WHITE,
            brightness: 10_000.0,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 100.0).looking_at(Vec3::ZERO, Vec3::Y),
        Camera3d::default(),
        Projection::Orthographic(OrthographicProjection::default_3d()),
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        // Projection::Orthographic(OrthographicProjection::default_3d()),
        RenderLayers::layer(1),
    ));
    commands.spawn((
        MainCamera,
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
    commands.spawn((
        Transform::from_xyz(10.0, 30.0, 10.0),
        Collider {
            size: vec3(0.8, 1.9, 0.8),
        },
        Velocity {
            linear: vec3(1.9, 0.0, 0.3),
        },
    ));
}

trait GizmosExt {
    fn block(&mut self, global: IVec3, color: Color);
    fn rect(&mut self, corner: Vec3, size: Vec3, color: Color);
}
impl<'w, 's> GizmosExt for Gizmos<'w, 's> {
    fn block(&mut self, global: IVec3, color: Color) {
        self.cuboid(
            Transform::from_translation(global.as_vec3() + Vec3::splat(0.5))
                .with_scale(Vec3::splat(1.002)),
            color,
        );
    }

    fn rect(&mut self, corner: Vec3, size: Vec3, color: Color) {
        self.cuboid(
            Transform {
                translation: corner + 0.5 * size,
                rotation: default(),
                scale: size,
            },
            color,
        );
    }
}

fn axis_overlay(mut gizmos: Gizmos<AxisOverlay>, transform: Single<&Transform, With<MainCamera>>) {
    const SCALE: f32 = 20.0;
    let orient = transform.rotation.inverse();
    for (base, color) in [
        (Vec3::X, Color::srgb(1.0, 0.0, 0.0)),
        (Vec3::Y, Color::srgb(0.0, 1.0, 0.0)),
        (Vec3::Z, Color::srgb(0.0, 0.0, 1.0)),
    ] {
        gizmos.line(Vec3::ZERO, orient * base * SCALE, color);
    }
}

#[derive(Debug, Clone, Copy)]
enum DimSwap {
    XX,
    XY,
    XZ,
}
impl std::ops::Mul<Vec3> for DimSwap {
    type Output = Vec3;

    fn mul(self, Vec3 { x, y, z }: Vec3) -> Self::Output {
        match self {
            DimSwap::XX => vec3(x, y, z),
            DimSwap::XY => vec3(y, x, z),
            DimSwap::XZ => vec3(z, y, x),
        }
    }
}
impl std::ops::Mul<IVec3> for DimSwap {
    type Output = IVec3;

    fn mul(self, IVec3 { x, y, z }: IVec3) -> Self::Output {
        match self {
            DimSwap::XX => ivec3(x, y, z),
            DimSwap::XY => ivec3(y, x, z),
            DimSwap::XZ => ivec3(z, y, x),
        }
    }
}

fn apply_gravity(velocity: Query<&mut Velocity, With<Collider>>) {
    for mut velocity in velocity {
        velocity.linear += Vec3::NEG_Y * 9.81;
    }
}

fn show_collider(
    mut gizmos: Gizmos,
    chunks: Res<ChunksIndex>,
    blocks: Query<&ChunkBlocks>,
    collider: Single<(&mut Transform, &Collider, &mut Velocity)>,
    time: Res<Time>,
) {
    let (mut tr, cl, mut vl) = collider.into_inner();

    let a = [
        (vl.linear.x, cl.size.x),
        (vl.linear.y, cl.size.y),
        (vl.linear.z, cl.size.z),
    ];
    let b: Vec3 = a
        .map(|(linear, size)| if linear < 0.0 { 0.0 } else { size })
        .into();

    let keyframe = time.elapsed_secs().rem_euclid(1.0);
    gizmos.rect(tr.translation, cl.size, Color::srgb(1.0, 0.3, 0.2));
    gizmos.rect(
        tr.translation + vl.linear * keyframe,
        cl.size,
        Color::srgb(1.0, 0.5, 0.0),
    );
    gizmos.line(
        tr.translation + b,
        tr.translation + b + vl.linear,
        Color::srgb(1.0, 0.5, 0.0),
    );

    // let a: [f32; 3] = cl.linear.into();
    // let a = a.map(|scalar| if scalar < 0.0 { Side::Neg } else { Side::Pos });
    // a.map(|side| match side {
    //     Side::Pos => todo!(),
    //     Side::Neg => todo!(),
    // });

    let mut shift = vl.linear * time.delta_secs();
    let mut new_vl = vl.linear;

    // let mut i = 0;
    'search: while let Ok(dir) = shift.try_into() {
        // i += 1;
        // println!("{:?}, {}", shift, i);
        for step in RayTraveler::new(tr.translation + b, dir, shift.norm()) {
            let (swap, color) = match step.dir {
                IVec3::X | IVec3::NEG_X => (DimSwap::XX, Color::srgb(1.0, 0.0, 0.0)),
                IVec3::Y | IVec3::NEG_Y => (DimSwap::XY, Color::srgb(0.0, 1.0, 0.0)),
                IVec3::Z | IVec3::NEG_Z => (DimSwap::XZ, Color::srgb(0.0, 0.0, 1.0)),
                _ => unreachable!(),
            };
            let collision = swap * (step.at - b);
            let size = swap * cl.size;
            let step_to = swap * step.to;
            for y in collision.y.floor() as i32..=(collision.y + size.y).floor() as i32 {
                for z in collision.z.floor() as i32..=(collision.z + size.z).floor() as i32 {
                    let selected = swap * ivec3(step_to.x, y, z);
                    if chunks.get(blocks, selected) {
                        let new_shift = swap * shift;
                        let new_shift =
                            new_shift.with_x(new_shift.x * (step.time / shift.norm() - 1e-4));
                        shift = swap * new_shift;
                        new_vl = swap * (swap * new_vl).with_x(0.0);
                        continue 'search;
                    }
                    // println!("{}", selected);
                    gizmos.block(selected, color);
                }
            }
            // match step.dir {
            //     IVec3::X | IVec3::NEG_X => {}
            //     _ => {}
            // }
            // if step.dir == IVec3::X {}
            // let color = match step.dir {
            //     IVec3::X | IVec3::NEG_X => Color::srgb(1.0, 0.0, 0.0),
            //     IVec3::Y | IVec3::NEG_Y => Color::srgb(0.0, 1.0, 0.0),
            //     IVec3::Z | IVec3::NEG_Z => Color::srgb(0.0, 0.0, 1.0),
            //     _ => Color::WHITE,
            // };
            // gizmos.block(step.to, color);
        }
        break 'search;
    }
    tr.translation += shift;
    vl.linear = new_vl;
}

fn move_collider(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    collider: Single<(&mut Transform, &mut Velocity), With<Collider>>,
) {
    let (mut tr, mut vl) = collider.into_inner();
    const RATE: f32 = 1.0;
    let tr = if keys.pressed(KeyCode::ControlRight) {
        &mut vl.linear
    } else {
        &mut tr.translation
    };
    if keys.pressed(KeyCode::KeyI) {
        tr.x += time.delta_secs() * RATE;
    }
    if keys.pressed(KeyCode::KeyK) {
        tr.x -= time.delta_secs() * RATE;
    }
    if keys.pressed(KeyCode::KeyL) {
        tr.z += time.delta_secs() * RATE;
    }
    if keys.pressed(KeyCode::KeyJ) {
        tr.z -= time.delta_secs() * RATE;
    }
    if keys.pressed(KeyCode::KeyU) {
        tr.y += time.delta_secs() * RATE;
    }
    if keys.pressed(KeyCode::KeyO) {
        tr.y -= time.delta_secs() * RATE;
    }
}

fn current_chunk_highlight(camera: Single<&Transform, With<MainCamera>>, mut gizmos: Gizmos) {
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
    if let Some((at, from)) = pointed.at {
        if button.just_pressed(MouseButton::Left) {
            modifications.push(Modify::Remove { at });
        } else if button.just_pressed(MouseButton::Right) {
            modifications.push(Modify::Place { at: from });
        }
    }
}

#[derive(Resource)]
struct PointedBlock {
    at: Option<(IVec3, IVec3)>,
}

fn pointed_block_show(
    camera: Single<&Transform, With<MainCamera>>,
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
                translation: side.as_vec3() + Vec3::ONE / 2.0,
                rotation: default(),
                scale: Vec3::ONE * 0.8,
            },
            Color::WHITE,
        );
    }
}

fn pointed_block(
    camera: Single<&Transform, With<MainCamera>>,
    blocks: Query<&ChunkBlocks>,
    terrain: Res<ChunksIndex>,
    mut pointed: ResMut<PointedBlock>,
) {
    let ray = camera.rotation * -Dir3::Z;
    let traveler = RayTraveler::new(camera.translation, ray, 16.0);
    for step in traveler {
        if let Some((chunk, local)) = terrain.global_to_local(step.to) {
            if let Ok(blocks) = blocks.get(chunk) {
                if blocks.get(local) {
                    pointed.at = Some((step.to, step.from));
                    return;
                }
            }
        }
    }
    pointed.at = None;
}

fn move_camera(
    mut camera: Single<&mut Transform, With<MainCamera>>,
    controller_state: Res<ControllerState>,
    time: Res<Time>,
) {
    const ROTATION_SENSITIVITY: f32 = 0.2;
    const TRANSLATION_SENSITIVITY: f32 = 20.0;

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
