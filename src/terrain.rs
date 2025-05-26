use std::{collections::HashMap, ops::RangeInclusive};

use bevy::{
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};

pub struct TerrainPlugin;

/// An entity that causes the terrain to be loaded around it
#[derive(Component, Clone, Copy)]
pub struct TerrainLoader {
    near: f32,
    far: f32,
}

#[derive(Component)]
pub struct TerrainChunk;

const CHUNK_WIDTH: i32 = 32;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        app.add_systems(Update, chunk_loader);
        app.insert_resource(Terrain {
            chunks: HashMap::new(),
        });
    }
}

impl TerrainLoader {
    pub fn new(radius: f32, intermediate: f32) -> Self {
        assert!(radius > 1.0);
        assert!(intermediate > 1.0);
        Self {
            near: radius,
            far: radius + intermediate,
        }
    }
    fn zone(self, transform: &Transform, chunk: IVec3) -> Zone {
        match transform.translation.distance(chunk_center(chunk)) {
            d if d < self.near => Zone::Load,
            d if d < self.far => Zone::Keep,
            _ => Zone::Clear,
        }
    }
    fn range(self) -> RangeInclusive<i32> {
        let d = self.far / CHUNK_WIDTH as f32;
        let d = d as i32 + 1;
        -d..=d
    }
}

#[derive(Resource)]
struct Terrain {
    chunks: HashMap<IVec3, Entity>,
}

#[derive(Resource)]
struct MeshAssets {
    material: Handle<StandardMaterial>,
}

fn setup(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    let material = materials.add(Color::srgb(0.0, 1.0, 0.0));
    commands.insert_resource(MeshAssets { material });
}

impl Terrain {
    fn get(&self, position: IVec3) -> bool {
        let level = noisy_bevy::simplex_noise_2d(0.05 * position.xz().as_vec2());
        ((4.0 * (level + 2.0)) as i32) > position.y
    }
    fn chunk_mesh(
        &self,
        chunk: IVec3,
        positions: &mut Vec<[f32; 3]>,
        normals: &mut Vec<[f32; 3]>,
        indices: &mut Vec<u32>,
    ) {
        for x in 0..CHUNK_WIDTH {
            for y in 0..CHUNK_WIDTH {
                for z in 0..CHUNK_WIDTH {
                    let position_local = IVec3 { x, y, z };
                    let position_global = position_local + CHUNK_WIDTH * chunk;
                    if self.get(position_global) {
                        make_cube_mesh(
                            position_local.as_vec3(),
                            !self.get(position_global + IVec3::X),
                            !self.get(position_global - IVec3::X),
                            !self.get(position_global + IVec3::Y),
                            !self.get(position_global - IVec3::Y),
                            !self.get(position_global + IVec3::Z),
                            !self.get(position_global - IVec3::Z),
                            positions,
                            normals,
                            indices,
                        );
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Zone {
    Load,
    #[allow(dead_code)]
    Keep,
    Clear,
}

fn in_chunk(transform: &Transform) -> IVec3 {
    (transform.translation / CHUNK_WIDTH as f32)
        .floor()
        .as_ivec3()
}
fn chunk_center(chunk: IVec3) -> Vec3 {
    (CHUNK_WIDTH * chunk).as_vec3() + Vec3::splat(CHUNK_WIDTH as f32 / 2.0)
}

fn chunk_loader(
    mut commands: Commands,
    mut terrain: ResMut<Terrain>,
    chunks: Query<&Transform, (With<TerrainChunk>, With<Mesh3d>)>,
    loaders: Query<(&TerrainLoader, &Transform), With<TerrainLoader>>,
    mut meshes: ResMut<Assets<Mesh>>,
    assets: Res<MeshAssets>,
) {
    for (loader, tr) in &loaders {
        let chunk = in_chunk(tr);
        for x in loader.range() {
            for y in loader.range() {
                for z in loader.range() {
                    let chunk = chunk + IVec3 { x, y, z };
                    if loader.zone(tr, chunk) == Zone::Load && !terrain.chunks.contains_key(&chunk)
                    {
                        let mut positions = Vec::new();
                        let mut normals = Vec::new();
                        let mut indices = Vec::new();
                        terrain.chunk_mesh(chunk, &mut positions, &mut normals, &mut indices);
                        let mesh = Mesh::new(PrimitiveTopology::TriangleList, default())
                            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
                            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
                            .with_inserted_indices(Indices::U32(indices));
                        let entity = commands
                            .spawn((
                                Transform::from_translation((chunk * CHUNK_WIDTH).as_vec3()),
                                Mesh3d(meshes.add(mesh)),
                                MeshMaterial3d(assets.material.clone()),
                                TerrainChunk,
                            ))
                            .id();
                        terrain.chunks.insert(chunk, entity);
                    }
                }
            }
        }
    }
    for chunk in &chunks {
        let chunk = in_chunk(chunk);
        if let Some(&entity) = terrain.chunks.get(&chunk) {
            if loaders
                .iter()
                .all(|(loader, tr)| loader.zone(tr, chunk) == Zone::Clear)
            {
                commands.entity(entity).despawn();
                terrain.chunks.remove(&chunk);
            }
        }
    }
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
    }
}
