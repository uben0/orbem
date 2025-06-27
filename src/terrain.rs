mod generation;
mod render;

use crate::{
    spacial::{Neighborhood, Sides},
    terrain::{
        generation::TerrainGenerator,
        render::{TerrainMaterial, chunk_meshing, setup_render},
    },
};

use super::octahedron;
use bevy::{
    input::common_conditions::input_just_pressed,
    platform::collections::{HashMap, hash_map::Entry},
    prelude::*,
};
use std::ops::RangeInclusive;

pub const CHUNK_WIDTH: i32 = 32;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Block {
    Air,
    Grass,
    Stone,
    Dirt,
    Sand,
}

/// Store terrain generation parameters
#[derive(Resource)]
struct Terrain;

#[derive(Component)]
pub struct ChunkBlocks {
    blocks: HashMap<IVec3, Block>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modify {
    Remove { at: IVec3 },
    Place { at: IVec3 },
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

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<TerrainMaterial>::default())
            .add_systems(Startup, setup_render)
            .add_systems(
                Update,
                (
                    chunk_indexer,
                    chunk_deloader,
                    chunk_generation.before(chunk_meshing),
                    chunk_need_mesh.before(chunk_meshing),
                    apply_modifications.before(chunk_meshing),
                    chunk_meshing,
                    remove_meshes.run_if(input_just_pressed(KeyCode::KeyU)),
                    reload_generation_parameters.run_if(input_just_pressed(KeyCode::KeyI)),
                ),
            )
            .insert_resource(TerrainGenerator::load_from_file())
            .insert_resource(Terrain)
            .insert_resource(Modifications { queue: Vec::new() })
            .insert_resource(ChunksIndex {
                chunks: HashMap::new(),
            });
    }
}
impl ChunksIndex {
    pub fn global_to_local(&self, global: IVec3) -> Option<(Entity, IVec3)> {
        let (chunk, local) = global_to_local(global);
        Some((*self.chunks.get(&chunk)?, local))
    }
    pub fn get(&self, blocks: Query<&ChunkBlocks>, global: IVec3) -> bool {
        let Some((chunk, local)) = self.global_to_local(global) else {
            return false;
        };
        let Ok(blocks) = blocks.get(chunk) else {
            return false;
        };
        blocks.get(local)
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

impl ChunkBlocks {
    pub fn get(&self, local: IVec3) -> bool {
        self.blocks.contains_key(&local)
    }
    pub fn remove(&mut self, local: IVec3) {
        self.blocks.remove(&local);
    }
    pub fn place(&mut self, local: IVec3, block: Block) {
        if block == Block::Air {
            self.blocks.remove(&local);
        } else {
            self.blocks.insert(local, block);
        }
    }
}

fn reload_generation_parameters(
    mut commands: Commands,
    mut parameters: ResMut<TerrainGenerator>,
    chunks: Query<Entity, With<Chunk>>,
) {
    *parameters = TerrainGenerator::load_from_file();
    for chunk in &chunks {
        commands
            .entity(chunk)
            .remove::<(ChunkBlocks, Mesh3d, MeshMaterial3d<TerrainMaterial>)>();
    }
}

fn apply_modifications(
    mut queue: ResMut<Modifications>,
    index: Res<ChunksIndex>,
    mut chunks_blocks: Query<&mut ChunkBlocks>,
    mut commands: Commands,
) {
    for modify in std::mem::take(&mut queue.queue) {
        match modify {
            Modify::Remove { at } => {
                let Some((chunk, local)) = index.global_to_local(at) else {
                    continue;
                };
                let Ok(mut blocks) = chunks_blocks.get_mut(chunk) else {
                    continue;
                };
                if !blocks.get(local) {
                    continue;
                }
                blocks.remove(local);

                for neighbor in Sides::AXIS {
                    let Some((neighbor, local)) = index.global_to_local(at + neighbor) else {
                        continue;
                    };
                    if neighbor == chunk {
                        continue;
                    }
                    let Ok(blocks) = chunks_blocks.get(neighbor) else {
                        continue;
                    };
                    if !blocks.get(local) {
                        continue;
                    }
                    commands.entity(neighbor).insert(MeshReload);
                }
                commands.entity(chunk).insert(MeshReload);
            }
            Modify::Place { at } => {
                let Some((chunk, local)) = index.global_to_local(at) else {
                    continue;
                };
                let Ok(mut blocks) = chunks_blocks.get_mut(chunk) else {
                    continue;
                };
                if blocks.get(local) {
                    continue;
                }
                blocks.place(local, Block::Stone);

                for neighbor in Sides::AXIS {
                    let Some((neighbor, local)) = index.global_to_local(at + neighbor) else {
                        continue;
                    };
                    if neighbor == chunk {
                        continue;
                    }
                    let Ok(blocks) = chunks_blocks.get(neighbor) else {
                        continue;
                    };
                    if !blocks.get(local) {
                        continue;
                    }
                    commands.entity(neighbor).insert(MeshReload);
                }
                commands.entity(chunk).insert(MeshReload);
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
                .remove::<(Mesh3d, MeshMaterial3d<TerrainMaterial>)>();
        }
    }
}

fn chunk_generation(
    loaders: Query<(&Transform, &TerrainLoader)>,
    chunks: Query<(Entity, &Chunk), Without<ChunkBlocks>>,
    mut commands: Commands,
    generator: Res<TerrainGenerator>,
) {
    // TODO: use the chunk wrapper for IVec3
    let priority = |chunk| {
        loaders
            .iter()
            .filter_map(|loader| loader.inside_priority(Zone::Blocks, chunk))
            .min()
    };
    if let Some((entity, chunk, _)) = chunks
        .iter()
        .filter_map(|(entity, &Chunk { chunk })| Some((entity, chunk, priority(chunk)?)))
        .min_by_key(|&(_, _, p)| p)
    {
        commands.entity(entity).insert(ChunkBlocks {
            blocks: generator.generate(chunk),
        });
    }
    // for (entity, &Chunk { chunk: index }) in &chunks {
    //     if loaders
    //         .iter()
    //         .any(|loader| loader.inside(Zone::Blocks, index))
    //     {
    //         commands.entity(entity).insert(ChunkBlocks {
    //             blocks: generate(index),
    //         });
    //     }
    // }
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

fn chunk_need_mesh(
    loaders: Query<(&Transform, &TerrainLoader)>,
    not_meshed: Query<(Entity, &Chunk), Without<Mesh3d>>,
    with_blocks: Query<&ChunkBlocks>,
    index: Res<ChunksIndex>,
    mut commands: Commands,
) {
    for (entity, &Chunk { chunk }) in &not_meshed {
        if loaders
            .iter()
            .any(|loader| loader.inside(Zone::Mesh, chunk))
        {
            let Some(neighborhood) =
                Neighborhood::from(chunk).try_map(|chunk| index.chunks.get(&chunk).copied())
            else {
                continue;
            };
            if neighborhood.all(|&chunk| with_blocks.contains(chunk)) {
                commands.entity(entity).insert(MeshReload);
            }
        }
    }
}

fn remove_meshes(mut commands: Commands, meshed: Query<Entity, (With<Chunk>, With<Mesh3d>)>) {
    for chunk in meshed {
        commands
            .entity(chunk)
            .remove::<(Mesh3d, MeshMaterial3d<TerrainMaterial>)>();
    }
}

trait TerrainLoaderExt {
    fn inside(self, zone: Zone, chunk: IVec3) -> bool;
    fn inside_priority(self, zone: Zone, chunk: IVec3) -> Option<u32>;
    fn outside(self, zone: Zone, chunk: IVec3) -> bool;
}
impl<'a> TerrainLoaderExt for (&'a Transform, &'a TerrainLoader) {
    fn inside(self, zone: Zone, chunk: IVec3) -> bool {
        let (tr, loader) = self;
        zone.distance(chunk, tr.translation) <= loader.radius
    }
    fn inside_priority(self, zone: Zone, chunk: IVec3) -> Option<u32> {
        let (tr, loader) = self;
        let distance = zone.distance(chunk, tr.translation);
        if distance <= loader.radius {
            Some(distance as u32)
        } else {
            None
        }
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
    fn range(self) -> RangeInclusive<i32> {
        let d = self.radius / CHUNK_WIDTH as f32;
        let d = d as i32 + 2;
        -d..=d
    }
}

/// Area around a player where the terrain should be loaded
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Zone {
    /// In this zone, a chunk should have its blocks generated and loaded
    Blocks,
    /// In the zone, a chunk should render
    Mesh,
}
impl Zone {
    /// Zones are ordered by dependency
    ///
    /// A chunk in the Mesh zone should already be in the Blocks zone
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

impl Block {
    fn textures(self) -> Option<Sides<u32>> {
        // 0 stone
        // 1 dirt
        // 2 grass side
        // 3 grass top
        // 4 sand
        match self {
            Block::Air => None,
            Block::Grass => Some(Sides {
                x_pos: 2,
                x_neg: 2,
                y_pos: 3,
                y_neg: 1,
                z_pos: 2,
                z_neg: 2,
            }),
            Block::Stone => Some(Sides {
                x_pos: 0,
                x_neg: 0,
                y_pos: 0,
                y_neg: 0,
                z_pos: 0,
                z_neg: 0,
            }),
            Block::Dirt => Some(Sides {
                x_pos: 1,
                x_neg: 1,
                y_pos: 1,
                y_neg: 1,
                z_pos: 1,
                z_neg: 1,
            }),
            Block::Sand => Some(Sides {
                x_pos: 4,
                x_neg: 4,
                y_pos: 4,
                y_neg: 4,
                z_pos: 4,
                z_neg: 4,
            }),
        }
    }
}
