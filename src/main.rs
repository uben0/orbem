mod controller;
mod octahedron;
mod physics;
mod ray_travel;
mod swizzle;
mod terrain;

use bevy::{
    asset::RenderAssetUsages,
    image::{CompressedImageFormats, ImageSampler},
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::{
        mesh::{Indices, MeshVertexAttribute, MeshVertexBufferLayoutRef, PrimitiveTopology},
        render_resource::{
            AsBindGroup, RenderPipelineDescriptor, ShaderRef, SpecializedMeshPipelineError,
        },
        view::RenderLayers,
    },
};
use bevy_framepace::FramepacePlugin;
use controller::{ControllerFetch, ControllerPlugin, ControllerState};
use physics::{ApplyPhysics, Collider, Grounded, PhysicsPlugin, Velocity};
use ray_travel::RayTraveler;
use std::{f32::consts::PI, fmt::Write};
use terrain::{ChunkBlocks, ChunksIndex, Modifications, Modify, TerrainLoader, TerrainPlugin};

const ATTRIBUTE_TEXTURE_INDEX: MeshVertexAttribute = MeshVertexAttribute::new(
    "TextureIndex",
    2760892297209218923,
    bevy::render::mesh::VertexFormat::Uint32,
);

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            TerrainPlugin,
            ControllerPlugin,
            FramepacePlugin,
            PhysicsPlugin,
            MaterialPlugin::<MyMaterial>::default(),
            MaterialPlugin::<MyMaterial2>::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                inspect_ui,
                pointed_block,
                // current_chunk_highlight,
                pointed_block_show.after(pointed_block),
                block_place_or_remove.after(pointed_block),
                player_toggle_flying,
                player_move_flying.after(ControllerFetch),
                (player_move_physics, player_rotate)
                    .before(ApplyPhysics)
                    .after(ControllerFetch),
                axis_overlay,
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
        .insert_gizmo_config(
            BlockHighligh,
            GizmoConfig {
                enabled: true,
                line: GizmoLineConfig {
                    width: 2.0,
                    ..default()
                },
                depth_bias: -0.001,
                render_layers: default(),
            },
        )
        .insert_resource(PointedBlock { at: None })
        .run();
}

#[derive(GizmoConfigGroup, Default, Reflect)]
struct AxisOverlay;

#[derive(GizmoConfigGroup, Default, Reflect)]
struct BlockHighligh;

#[derive(Component)]
struct Player;

fn load_texture_atlas() -> Image {
    let bytes = std::fs::read("assets/textures/blocks.png").unwrap();
    let mut textures = Image::from_buffer(
        &bytes,
        bevy::image::ImageType::Format(ImageFormat::Png),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::nearest(),
        RenderAssetUsages::RENDER_WORLD,
    )
    .unwrap();
    textures.reinterpret_stacked_2d_as_array(2);
    textures
}
fn load_texture_test() -> Image {
    let bytes = std::fs::read("assets/textures/dirt-side.png").unwrap();
    let mut textures = Image::from_buffer(
        &bytes,
        bevy::image::ImageType::Format(ImageFormat::Png),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::nearest(),
        RenderAssetUsages::RENDER_WORLD,
    )
    .unwrap();
    textures
}

fn make_cube_mesh(
    tr: Vec3,
    x_pos: bool,
    x_neg: bool,
    y_pos: bool,
    y_neg: bool,
    z_pos: bool,
    z_neg: bool,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    texture_uvs: &mut Vec<[f32; 2]>,
    texture_indices: &mut Vec<u32>,
) {
    if x_pos {
        let index = positions.len() as u32;
        positions.extend([
            [tr.x + 1.0, tr.y + 0.0, tr.z + 0.0],
            [tr.x + 1.0, tr.y + 0.0, tr.z + 1.0],
            [tr.x + 1.0, tr.y + 1.0, tr.z + 0.0],
            [tr.x + 1.0, tr.y + 1.0, tr.z + 1.0],
        ]);
        normals.extend([[1.0, 0.0, 0.0]; 4]);
        indices.extend([
            index + 0,
            index + 2,
            index + 1,
            index + 1,
            index + 2,
            index + 3,
        ]);
        texture_uvs.extend([[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]]);
        texture_indices.extend([0; 4]);
    }
    if x_neg {
        let index = positions.len() as u32;
        positions.extend([
            [tr.x + 0.0, tr.y + 0.0, tr.z + 0.0],
            [tr.x + 0.0, tr.y + 0.0, tr.z + 1.0],
            [tr.x + 0.0, tr.y + 1.0, tr.z + 0.0],
            [tr.x + 0.0, tr.y + 1.0, tr.z + 1.0],
        ]);
        normals.extend([[-1.0, 0.0, 0.0]; 4]);
        indices.extend([
            index + 0,
            index + 1,
            index + 2,
            index + 1,
            index + 3,
            index + 2,
        ]);
        texture_uvs.extend([[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]]);
        texture_indices.extend([0; 4]);
    }
    if y_pos {
        let index = positions.len() as u32;
        positions.extend([
            [tr.x + 0.0, tr.y + 1.0, tr.z + 0.0],
            [tr.x + 0.0, tr.y + 1.0, tr.z + 1.0],
            [tr.x + 1.0, tr.y + 1.0, tr.z + 0.0],
            [tr.x + 1.0, tr.y + 1.0, tr.z + 1.0],
        ]);
        normals.extend([[0.0, 1.0, 0.0]; 4]);
        indices.extend([
            index + 0,
            index + 1,
            index + 2,
            index + 1,
            index + 3,
            index + 2,
        ]);
        texture_uvs.extend([[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]]);
        texture_indices.extend([0; 4]);
    }
    if y_neg {
        let index = positions.len() as u32;
        positions.extend([
            [tr.x + 0.0, tr.y + 0.0, tr.z + 0.0],
            [tr.x + 0.0, tr.y + 0.0, tr.z + 1.0],
            [tr.x + 1.0, tr.y + 0.0, tr.z + 0.0],
            [tr.x + 1.0, tr.y + 0.0, tr.z + 1.0],
        ]);
        normals.extend([[0.0, -1.0, 0.0]; 4]);
        indices.extend([
            index + 0,
            index + 2,
            index + 1,
            index + 1,
            index + 2,
            index + 3,
        ]);
        texture_uvs.extend([[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]]);
        texture_indices.extend([0; 4]);
    }
    if z_pos {
        let index = positions.len() as u32;
        positions.extend([
            [tr.x + 0.0, tr.y + 0.0, tr.z + 1.0],
            [tr.x + 0.0, tr.y + 1.0, tr.z + 1.0],
            [tr.x + 1.0, tr.y + 0.0, tr.z + 1.0],
            [tr.x + 1.0, tr.y + 1.0, tr.z + 1.0],
        ]);
        normals.extend([[0.0, 0.0, 1.0]; 4]);
        indices.extend([
            index + 0,
            index + 2,
            index + 1,
            index + 1,
            index + 2,
            index + 3,
        ]);
        texture_uvs.extend([[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]]);
        texture_indices.extend([0; 4]);
    }
    if z_neg {
        let index = positions.len() as u32;
        positions.extend([
            [tr.x + 0.0, tr.y + 0.0, tr.z + 0.0],
            [tr.x + 0.0, tr.y + 1.0, tr.z + 0.0],
            [tr.x + 1.0, tr.y + 0.0, tr.z + 0.0],
            [tr.x + 1.0, tr.y + 1.0, tr.z + 0.0],
        ]);
        normals.extend([[0.0, 0.0, -1.0]; 4]);
        indices.extend([
            index + 0,
            index + 1,
            index + 2,
            index + 1,
            index + 3,
            index + 2,
        ]);
        texture_uvs.extend([[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]]);
        texture_indices.extend([0; 4]);
    }
}

fn create_test_mesh() -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    let mut texture_uvs = Vec::new();
    let mut texture_indices = Vec::new();
    make_cube_mesh(
        Vec3::ZERO,
        true,
        true,
        true,
        true,
        true,
        true,
        &mut positions,
        &mut normals,
        &mut indices,
        &mut texture_uvs,
        &mut texture_indices,
    );
    Mesh::new(PrimitiveTopology::TriangleList, default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, texture_uvs)
        .with_inserted_attribute(ATTRIBUTE_TEXTURE_INDEX, texture_indices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_indices(Indices::U32(indices))
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<MyMaterial>>,
    mut materials2: ResMut<Assets<StandardMaterial>>,
    mut materials3: ResMut<Assets<MyMaterial2>>,
) {
    commands.insert_resource(ClearColor(Color::srgb(0.7, 0.9, 1.0)));
    commands.spawn((
        Transform::from_xyz(0.0, 16.0, 4.0),
        Mesh3d(meshes.add(create_test_mesh())),
        MeshMaterial3d(materials.add(MyMaterial {
            texture: images.add(load_texture_atlas()),
        })),
    ));
    commands.spawn((
        Transform::from_xyz(0.0, 16.0, 2.0),
        Mesh3d(meshes.add(create_test_mesh())),
        MeshMaterial3d(materials3.add(MyMaterial2 {
            texture: images.add(load_texture_test()),
        })),
    ));
    commands.spawn((
        Transform::from_xyz(0.0, 16.0, 0.0),
        Mesh3d(meshes.add(create_test_mesh())),
        MeshMaterial3d(materials2.add(images.add(load_texture_test()))),
    ));
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.8, 0.9, 1.0),
        brightness: 1000.0,
        ..default()
    });
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(2.5, 5.0, 1.8).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Player,
        TerrainLoader::new(64.0, 20.0),
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 100.0f32.to_radians(),
            ..default()
        }),
        Transform::from_xyz(2.0, 18.0, 1.0).looking_at(vec3(0.0, 18.0, 0.0), Vec3::Y),
        Collider {
            size: vec3(0.8, 1.9, 0.8),
            anchor: vec3(0.4, 1.7, 0.4),
        },
    ));
    commands.spawn((
        Transform::from_xyz(0.0, 0.0, 100.0).looking_at(Vec3::ZERO, Vec3::Y),
        Camera3d::default(),
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        RenderLayers::layer(1),
        Projection::Orthographic(OrthographicProjection::default_3d()),
    ));
    let font = TextFont {
        font_size: 12.0,
        ..default()
    };
    commands.spawn((
        Node {
            top: Val::Px(5.0),
            left: Val::Px(5.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(5.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.4)),
        children![
            (Text("x:".to_string()), font.clone()),
            (Text("y:".to_string()), font.clone()),
            (Text("z:".to_string()), font.clone()),
        ],
        InspectUi,
    ));
}

#[derive(Component)]
struct InspectUi;

fn inspect_ui(
    mut texts: Query<&mut Text>,
    root: Single<(Entity, &Children), With<InspectUi>>,
    player: Single<&Transform, With<Player>>,
) {
    let (_, children) = root.into_inner();

    for (axis, child, value) in [
        ("x", 0, player.translation.x),
        ("y", 1, player.translation.y),
        ("z", 2, player.translation.z),
    ] {
        let text = &mut texts.get_mut(children[child]).unwrap().0;
        text.clear();
        write!(text, "{}: {:>+8.3}", axis, value).unwrap();
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct MyMaterial {
    #[texture(0, dimension = "2d_array")]
    #[sampler(1)]
    texture: Handle<Image>,
}
impl Material for MyMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/my_material.wgsl".into()
    }
    fn vertex_shader() -> ShaderRef {
        "shaders/my_material.wgsl".into()
    }
    fn specialize(
        _: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
            ATTRIBUTE_TEXTURE_INDEX.at_shader_location(2),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(3),
        ])?;
        descriptor.vertex.buffers = Vec::from([vertex_layout]);
        Ok(())
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct MyMaterial2 {
    #[texture(0)]
    #[sampler(1)]
    texture: Handle<Image>,
}
impl Material for MyMaterial2 {
    fn fragment_shader() -> ShaderRef {
        "shaders/my_material2.wgsl".into()
    }
}

trait GizmosExt {
    fn block(&mut self, global: IVec3, color: Color);
    // fn aabb(&mut self, corner: Vec3, size: Vec3, color: Color);
}
impl<'w, 's, Group: GizmoConfigGroup> GizmosExt for Gizmos<'w, 's, Group> {
    fn block(&mut self, global: IVec3, color: Color) {
        self.cuboid(
            Transform::from_translation(global.as_vec3() + Vec3::splat(0.5))
                .with_scale(Vec3::splat(1.0002)),
            color,
        );
    }

    // fn aabb(&mut self, corner: Vec3, size: Vec3, color: Color) {
    //     self.cuboid(
    //         Transform {
    //             translation: corner + 0.5 * size,
    //             rotation: default(),
    //             scale: size,
    //         },
    //         color,
    //     );
    // }
}

fn axis_overlay(mut gizmos: Gizmos<AxisOverlay>, transform: Single<&Transform, With<Player>>) {
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

// #[derive(Component)]
// struct ControlledPhysically;

fn current_chunk_highlight(player: Single<&Transform, With<Player>>, mut gizmos: Gizmos) {
    let global = player.translation.floor().as_ivec3();
    let (chunk, _) = terrain::global_to_local(global);
    let center: Vec3A = terrain::chunk_center(chunk).into();
    let color = Color::srgb(0.3, 0.5, 0.7);
    let cells = UVec2::splat(32);
    let size = Vec2::splat(1.0);
    let half_chunk = terrain::CHUNK_WIDTH as f32 / 2.0;
    gizmos
        .grid(
            Isometry3d {
                rotation: default(),
                translation: center + half_chunk * -Vec3A::Z,
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
                translation: center + half_chunk * Vec3A::Z,
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
                translation: center + half_chunk * -Vec3A::X,
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
                translation: center + half_chunk * Vec3A::X,
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

fn pointed_block_show(pointed: Res<PointedBlock>, mut gizmos: Gizmos<BlockHighligh>) {
    if let Some((at, _)) = pointed.at {
        gizmos.block(at, Color::BLACK);
    }
}

fn pointed_block(
    player: Single<&Transform, With<Player>>,
    blocks: Query<&ChunkBlocks>,
    terrain: Res<ChunksIndex>,
    mut pointed: ResMut<PointedBlock>,
) {
    let ray = player.rotation * -Dir3::Z;
    let traveler = RayTraveler::new(player.translation, ray, 16.0);
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

fn player_toggle_flying(
    mut commands: Commands,
    player: Single<(Entity, Has<Velocity>), With<Player>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let (player, physics) = *player;
    if keys.just_pressed(KeyCode::KeyV) {
        if physics {
            commands.entity(player).remove::<Velocity>();
        } else {
            commands
                .entity(player)
                .insert(Velocity { linear: Vec3::ZERO });
        }
    }
}

fn player_rotate(
    mut transform: Single<&mut Transform, With<Player>>,
    time: Res<Time>,
    inputs: Res<ControllerState>,
) {
    const ROTATION_SENSITIVITY: f32 = 0.2;
    let (yaw, pitch, _) = transform.rotation.to_euler(default());
    let delta = vec2(yaw, pitch) - ROTATION_SENSITIVITY * time.delta_secs() * inputs.mouse;
    transform.rotation = Quat::from_euler(
        EulerRot::default(),
        delta.x.rem_euclid(2.0 * PI),
        delta.y.clamp(-PI / 2.0, PI / 2.0),
        0.0,
    );
}

fn player_move_flying(
    mut transform: Single<&mut Transform, (With<Player>, Without<Velocity>)>,
    inputs: Res<ControllerState>,
    time: Res<Time>,
) {
    let linear_sensi = if inputs.sprint { 40.0 } else { 8.0 };
    let (yaw, _, _) = transform.rotation.to_euler(default());
    let aligned = Quat::from_euler(EulerRot::default(), yaw, 0.0, 0.0);
    transform.translation += aligned * inputs.linear_3d * time.delta_secs() * linear_sensi;
}

fn player_move_physics(
    player: Single<(&Transform, &mut Velocity, Has<Grounded>), With<Player>>,
    inputs: Res<ControllerState>,
    time: Res<Time>,
) {
    let (transform, mut velocity, grounded) = player.into_inner();
    let linear_force: f32 = if grounded {
        if inputs.sprint { 100.0 } else { 70.0 }
    } else {
        40.0
    };

    let (yaw, _, _) = transform.rotation.to_euler(default());
    let aligned = Quat::from_euler(EulerRot::default(), yaw, 0.0, 0.0);

    if grounded && inputs.jump {
        velocity.linear.y = 12.0;
    }

    velocity.linear += aligned * linear_force * time.delta_secs() * inputs.linear_2d;
}
