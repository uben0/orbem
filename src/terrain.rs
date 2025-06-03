use super::octahedron;
use bevy::{
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};
use std::{
    collections::{HashMap, hash_map::Entry},
    ops::RangeInclusive,
};

pub struct TerrainPlugin;

/// An entity that causes the terrain to be loaded around it
#[derive(Component, Clone, Copy)]
pub struct TerrainLoader {
    radius: f32,
    buffer: f32,
}

#[derive(Component)]
struct Chunk {
    chunk: IVec3,
}

#[derive(Resource)]
pub struct ChunksIndex {
    chunks: HashMap<IVec3, Entity>,
}
impl ChunksIndex {
    pub fn global_to_local(&self, global: IVec3) -> Option<(Entity, IVec3)> {
        let (chunk, local) = global_to_local(global);
        Some((*self.chunks.get(&chunk)?, local))
    }
    // pub fn global_to_local_neighborhood(
    //     &self,
    //     global: IVec3,
    // ) -> Option<(Neighborhood<Entity>, IVec3)> {
    //     let (chunk, local) = global_to_local(global);
    //     Some((
    //         Neighborhood::from(chunk)
    //             .try_map(|chunk| self.chunks.get(&chunk))?
    //             .map(|chunk| *chunk),
    //         local,
    //     ))
    // }
}

/// Store terrain generation parameters
#[derive(Resource)]
struct Terrain;

#[derive(Component)]
pub struct ChunkBlocks {
    blocks: HashMap<IVec3, ()>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modify {
    Remove { at: IVec3 },
}

#[derive(Resource)]
pub struct Modifications {
    queue: Vec<Modify>,
}

#[derive(Component)]
struct MeshReload;

impl Modifications {
    pub fn push(&mut self, modify: Modify) {
        self.queue.push(modify);
    }
}

pub const CHUNK_WIDTH: i32 = 32;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    chunk_generation,
                    chunk_meshing,
                    chunk_indexer,
                    chunk_deloader,
                    apply_modifications,
                ),
            )
            .insert_resource(Terrain)
            .insert_resource(Modifications { queue: Vec::new() })
            .insert_resource(ChunksIndex {
                chunks: HashMap::new(),
            });
    }
}

impl ChunkBlocks {
    pub fn get(&self, local: IVec3) -> bool {
        self.blocks.contains_key(&local)
    }
    pub fn remove(&mut self, local: IVec3) {
        self.blocks.remove(&local);
    }
}

const NEIGHBORS: [IVec3; 6] = [
    IVec3 { x: 1, y: 0, z: 0 },
    IVec3 { x: -1, y: 0, z: 0 },
    IVec3 { y: 1, x: 0, z: 0 },
    IVec3 { y: -1, x: 0, z: 0 },
    IVec3 { z: 1, x: 0, y: 0 },
    IVec3 { z: -1, x: 0, y: 0 },
];

fn apply_modifications(
    mut queue: ResMut<Modifications>,
    index: Res<ChunksIndex>,
    mut blocks: Query<&mut ChunkBlocks>,
    mut commands: Commands,
) {
    for modify in std::mem::take(&mut queue.queue) {
        match modify {
            Modify::Remove { at } => {
                if let Some((chunk, local)) = index.global_to_local(at) {
                    if let Ok(mut blocks) = blocks.get_mut(chunk) {
                        if blocks.get(local) {
                            blocks.remove(local);

                            for neighbor in NEIGHBORS {
                                if let Some((neighbor, _)) = index.global_to_local(at + neighbor) {
                                    if neighbor != chunk {
                                        commands.entity(neighbor).insert(MeshReload);
                                    }
                                }
                            }
                            commands.entity(chunk).insert(MeshReload);
                        }
                    }
                }
            }
        }
    }
}

fn chunk_indexer(
    loaders: Query<(&Transform, &TerrainLoader)>,
    mut index: ResMut<ChunksIndex>,
    mut commands: Commands,
) {
    for (transform, loader) in &loaders {
        let (chunk, _) = global_to_local(transform.translation.as_ivec3());
        for x in loader.range() {
            for y in loader.range() {
                for z in loader.range() {
                    let chunk = chunk + IVec3 { x, y, z };
                    if let Entry::Vacant(entry) = index.chunks.entry(chunk) {
                        entry.insert(
                            commands
                                .spawn((
                                    Transform::from_translation((chunk * CHUNK_WIDTH).as_vec3()),
                                    Chunk { chunk },
                                ))
                                .id(),
                        );
                    }
                }
            }
        }
    }
}

fn chunk_deloader(
    mut commands: Commands,
    chunks: Query<(Entity, &Chunk), With<Mesh3d>>,
    loaders: Query<(&Transform, &TerrainLoader)>,
) {
    for (entity, &Chunk { chunk }) in &chunks {
        if loaders
            .iter()
            .all(|loader| loader.outside(Zone::Mesh, chunk))
        {
            commands
                .entity(entity)
                .remove::<(Mesh3d, MeshMaterial3d<StandardMaterial>)>();
        }
    }
}

fn chunk_generation(
    terrain: Res<Terrain>,
    loaders: Query<(&Transform, &TerrainLoader)>,
    chunks: Query<(Entity, &Chunk), Without<ChunkBlocks>>,
    mut commands: Commands,
) {
    for (entity, &Chunk { chunk: index }) in &chunks {
        if loaders
            .iter()
            .any(|loader| loader.inside(Zone::Blocks, index))
        {
            commands.entity(entity).insert(terrain.gen_chunk(index));
        }
    }
}

struct Neighborhood<T> {
    zero: T,
    x_pos: T,
    x_neg: T,
    y_pos: T,
    y_neg: T,
    z_pos: T,
    z_neg: T,
}
impl From<IVec3> for Neighborhood<IVec3> {
    fn from(value: IVec3) -> Self {
        Self {
            zero: value,
            x_pos: value + IVec3::X,
            x_neg: value - IVec3::X,
            y_pos: value + IVec3::Y,
            y_neg: value - IVec3::Y,
            z_pos: value + IVec3::Z,
            z_neg: value - IVec3::Z,
        }
    }
}
impl<T> Neighborhood<T> {
    // fn map<U>(self, mut f: impl FnMut(T) -> U) -> Neighborhood<U> {
    //     Neighborhood {
    //         zero: f(self.zero),
    //         x_pos: f(self.x_pos),
    //         x_neg: f(self.x_neg),
    //         y_pos: f(self.y_pos),
    //         y_neg: f(self.y_neg),
    //         z_pos: f(self.z_pos),
    //         z_neg: f(self.z_neg),
    //     }
    // }
    fn try_map<U>(self, mut f: impl FnMut(T) -> Option<U>) -> Option<Neighborhood<U>> {
        Some(Neighborhood {
            zero: f(self.zero)?,
            x_pos: f(self.x_pos)?,
            x_neg: f(self.x_neg)?,
            y_pos: f(self.y_pos)?,
            y_neg: f(self.y_neg)?,
            z_pos: f(self.z_pos)?,
            z_neg: f(self.z_neg)?,
        })
    }
}
impl<'a> Neighborhood<&'a ChunkBlocks> {
    fn get(&self, relative: IVec3) -> bool {
        const CW: i32 = CHUNK_WIDTH;
        let (chunk, at) = match relative {
            IVec3 { x: -1, y, z } => (self.x_neg, IVec3 { x: CW - 1, y, z }),
            IVec3 { x, y: -1, z } => (self.y_neg, IVec3 { x, y: CW - 1, z }),
            IVec3 { x, y, z: -1 } => (self.z_neg, IVec3 { x, y, z: CW - 1 }),
            IVec3 { x: CW, y, z } => (self.x_pos, IVec3 { x: 0, y, z }),
            IVec3 { x, y: CW, z } => (self.y_pos, IVec3 { x, y: 0, z }),
            IVec3 { x, y, z: CW } => (self.z_pos, IVec3 { x, y, z: 0 }),
            it => (self.zero, it),
        };
        debug_assert!(at.x >= 0);
        debug_assert!(at.x < CW);
        debug_assert!(at.y >= 0);
        debug_assert!(at.y < CW);
        debug_assert!(at.z >= 0);
        debug_assert!(at.z < CW);
        chunk.blocks.get(&at).is_some()
    }
}

fn chunk_meshing(
    loaders: Query<(&Transform, &TerrainLoader)>,
    not_meshed: Query<(Entity, &Chunk), Or<(Without<Mesh3d>, With<MeshReload>)>>,
    with_blocks: Query<&ChunkBlocks>,
    index: Res<ChunksIndex>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    assets: Res<MeshAssets>,
) {
    for (entity, &Chunk { chunk }) in &not_meshed {
        if loaders
            .iter()
            .any(|loader| loader.inside(Zone::Mesh, chunk))
        {
            let Some(neighborhood) = Neighborhood::from(chunk)
                .try_map(|chunk| with_blocks.get(*index.chunks.get(&chunk)?).ok())
            else {
                continue;
            };

            let mut positions = Vec::new();
            let mut normals = Vec::new();
            let mut indices = Vec::new();
            for x in 0..CHUNK_WIDTH {
                for y in 0..CHUNK_WIDTH {
                    for z in 0..CHUNK_WIDTH {
                        let local = IVec3 { x, y, z };
                        if neighborhood.get(local) {
                            make_cube_mesh(
                                local.as_vec3(),
                                !neighborhood.get(local + IVec3::X),
                                !neighborhood.get(local - IVec3::X),
                                !neighborhood.get(local + IVec3::Y),
                                !neighborhood.get(local - IVec3::Y),
                                !neighborhood.get(local + IVec3::Z),
                                !neighborhood.get(local - IVec3::Z),
                                &mut positions,
                                &mut normals,
                                &mut indices,
                            );
                        }
                    }
                }
            }
            let mesh = Mesh::new(PrimitiveTopology::TriangleList, default())
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
                .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
                .with_inserted_indices(Indices::U32(indices));
            let mesh = meshes.add(mesh);
            commands
                .entity(entity)
                .remove::<MeshReload>()
                .insert((Mesh3d(mesh), MeshMaterial3d(assets.material.clone())));
        }
    }
}

// impl<'a> (&'a Transform, &'a TerrainLoader) {}
trait TerrainLoaderExt {
    fn inside(self, zone: Zone, chunk: IVec3) -> bool;
    fn outside(self, zone: Zone, chunk: IVec3) -> bool;
}
impl<'a> TerrainLoaderExt for (&'a Transform, &'a TerrainLoader) {
    fn inside(self, zone: Zone, chunk: IVec3) -> bool {
        let (tr, loader) = self;
        zone.distance(chunk, tr.translation) <= loader.radius
    }

    fn outside(self, zone: Zone, chunk: IVec3) -> bool {
        let (tr, loader) = self;
        zone.distance(chunk, tr.translation) > loader.radius + loader.buffer
    }
}

impl TerrainLoader {
    pub fn new(radius: f32, buffer: f32) -> Self {
        assert!(radius > 1.0);
        assert!(buffer > 1.0);
        Self { radius, buffer }
    }
    // fn inside(self, transform: &Transform, zone: Zone, chunk: IVec3) -> bool {
    //     zone.distance(chunk, transform.translation) <= self.radius
    // }
    // fn outside(self, transform: &Transform, zone: Zone, chunk: IVec3) -> bool {
    //     zone.distance(chunk, transform.translation) > self.radius + self.buffer
    // }
    fn range(self) -> RangeInclusive<i32> {
        let d = self.radius / CHUNK_WIDTH as f32;
        let d = d as i32 + 2;
        -d..=d
    }
}

#[derive(Resource)]
struct MeshAssets {
    material: Handle<StandardMaterial>,
}

fn setup(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    let material = materials.add(Color::srgb(0.0, 1.0, 0.0));
    commands.insert_resource(MeshAssets { material });
    // commands.spawn((Transform::default(), Chunk { chunk: IVec3::ZERO }));
}

impl Terrain {
    fn elevation(&self, position: IVec2) -> i32 {
        (8.0 + 4.0 * noisy_bevy::simplex_noise_2d(0.05 * position.as_vec2())) as i32
    }
    fn gen_chunk(&self, chunk: IVec3) -> ChunkBlocks {
        let mut blocks = HashMap::new();
        for x in 0..CHUNK_WIDTH {
            for z in 0..CHUNK_WIDTH {
                let position = CHUNK_WIDTH * chunk.xz() + IVec2::new(x, z);
                let elevation_relative = self.elevation(position) - chunk.y * CHUNK_WIDTH;
                for y in 0..elevation_relative {
                    blocks.insert(IVec3 { x, y, z }, ());
                }
            }
        }
        ChunkBlocks { blocks }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Zone {
    Blocks,
    Mesh,
}
impl Zone {
    const fn level(self) -> i32 {
        match self {
            Zone::Blocks => 1,
            Zone::Mesh => 0,
        }
    }
    const fn reach(self) -> f32 {
        (CHUNK_WIDTH * self.level()) as f32 * 1.05
    }
    fn distance(self, chunk: IVec3, point: Vec3) -> f32 {
        octahedron::distance(chunk_center(chunk), self.reach(), point)
    }
}

pub fn chunk_center(chunk: IVec3) -> Vec3 {
    (CHUNK_WIDTH * chunk).as_vec3() + Vec3::splat(CHUNK_WIDTH as f32 / 2.0)
}

#[allow(dead_code)]
fn local_to_global(chunk: IVec3, local: IVec3) -> IVec3 {
    chunk * CHUNK_WIDTH + local
}
pub fn global_to_local(global: IVec3) -> (IVec3, IVec3) {
    let width = IVec3::splat(CHUNK_WIDTH);
    (global.div_euclid(width), global.rem_euclid(width))
}

#[inline]
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
