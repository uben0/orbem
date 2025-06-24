use super::octahedron;
use bevy::{
    asset::RenderAssetUsages,
    image::{CompressedImageFormats, ImageSampler},
    input::common_conditions::input_just_pressed,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::{
        mesh::{
            Indices, MeshVertexAttribute, MeshVertexBufferLayoutRef, PrimitiveTopology,
            VertexFormat,
        },
        render_resource::{
            AsBindGroup, RenderPipelineDescriptor, ShaderRef, SpecializedMeshPipelineError,
        },
    },
};
use std::{
    collections::{HashMap, hash_map::Entry},
    ops::RangeInclusive,
};

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
            .add_systems(Startup, setup)
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
                ),
            )
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

                for neighbor in NEIGHBORS {
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

                for neighbor in NEIGHBORS {
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
    fn as_array(&self) -> [&T; 7] {
        [
            &self.zero,
            &self.x_pos,
            &self.x_neg,
            &self.y_pos,
            &self.y_neg,
            &self.z_pos,
            &self.z_neg,
        ]
    }
    fn all(&self, f: impl FnMut(&T) -> bool) -> bool {
        self.as_array().into_iter().all(f)
    }
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

fn chunk_meshing(
    not_meshed: Query<(Entity, &Chunk), With<MeshReload>>,
    with_blocks: Query<&ChunkBlocks>,
    index: Res<ChunksIndex>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    assets: Res<MeshAssets>,
) {
    for (entity, &Chunk { chunk }) in &not_meshed {
        let Some(neighborhood) = Neighborhood::from(chunk)
            .try_map(|chunk| with_blocks.get(*index.chunks.get(&chunk)?).ok())
        else {
            continue;
        };
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut indices = Vec::new();
        let mut texture_uvs = Vec::new();
        let mut texture_indices = Vec::new();
        for (&local, &_) in &neighborhood.zero.blocks {
            assert!(local.x >= 0);
            assert!(local.x < CHUNK_WIDTH);
            assert!(local.y >= 0);
            assert!(local.y < CHUNK_WIDTH);
            assert!(local.z >= 0);
            assert!(local.z < CHUNK_WIDTH);
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
                &mut texture_uvs,
                &mut texture_indices,
            );
        }
        assert_eq!(positions.len(), normals.len());
        assert_eq!(positions.len(), texture_uvs.len());
        assert_eq!(positions.len(), texture_indices.len());
        assert_eq!(indices.len() % 6, 0);
        assert_eq!(positions.len() % 4, 0);
        assert_eq!(positions.len() / 4, indices.len() / 6);
        let mesh = Mesh::new(PrimitiveTopology::TriangleList, default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, texture_uvs)
            .with_inserted_attribute(ATTRIBUTE_TEXTURE_INDEX, texture_indices)
            .with_inserted_indices(Indices::U32(indices));
        let mesh = meshes.add(mesh);
        commands
            .entity(entity)
            .remove::<MeshReload>()
            .insert((Mesh3d(mesh), MeshMaterial3d(assets.material.clone())));
    }
}

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
    fn range(self) -> RangeInclusive<i32> {
        let d = self.radius / CHUNK_WIDTH as f32;
        let d = d as i32 + 2;
        -d..=d
    }
}

#[derive(Resource)]
struct MeshAssets {
    material: Handle<TerrainMaterial>,
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
) {
    commands.insert_resource(MeshAssets {
        material: materials.add(TerrainMaterial {
            texture: images.add(load_texture_atlas()),
        }),
    });
}
fn load_texture_atlas() -> Image {
    let bytes = std::fs::read("assets/textures/blocks.png").unwrap();
    let mut textures = Image::from_buffer(
        &bytes,
        bevy::image::ImageType::Format(ImageFormat::Png),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::nearest(),
        RenderAssetUsages::default(),
    )
    .unwrap();
    textures.reinterpret_stacked_2d_as_array(2);
    textures
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
                for y in 0..elevation_relative.min(CHUNK_WIDTH) {
                    blocks.insert(IVec3 { x, y, z }, Block::Grass);
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

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct TerrainMaterial {
    #[texture(0, dimension = "2d_array")]
    #[sampler(1)]
    texture: Handle<Image>,
}
const ATTRIBUTE_TEXTURE_INDEX: MeshVertexAttribute =
    MeshVertexAttribute::new("TextureIndex", 2760892297209218923, VertexFormat::Uint32);

impl Material for TerrainMaterial {
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
