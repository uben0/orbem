use crate::{
    spacial::{Neighborhood, Side, Sides},
    terrain::{CHUNK_WIDTH, Chunk, ChunkBlocks, ChunksIndex, MeshReload},
};
use bevy::{
    asset::RenderAssetUsages,
    image::{CompressedImageFormats, ImageSampler},
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
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

#[derive(Resource)]
pub struct MeshAssets {
    material: Handle<TerrainMaterial>,
}

pub fn setup_render(
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
    textures.reinterpret_stacked_2d_as_array(textures.height() / 16);
    textures
}

pub fn chunk_meshing(
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
        for (&local, &block) in &neighborhood.zero.blocks {
            let Some(textures) = block.textures() else {
                continue;
            };
            assert!(local.x >= 0);
            assert!(local.x < CHUNK_WIDTH);
            assert!(local.y >= 0);
            assert!(local.y < CHUNK_WIDTH);
            assert!(local.z >= 0);
            assert!(local.z < CHUNK_WIDTH);
            make_cube_mesh(
                local.as_vec3(),
                Sides::AXIS.map(|dir| !neighborhood.get(local + dir)),
                textures,
                // !neighborhood.get(local + IVec3::X),
                // !neighborhood.get(local - IVec3::X),
                // !neighborhood.get(local + IVec3::Y),
                // !neighborhood.get(local - IVec3::Y),
                // !neighborhood.get(local + IVec3::Z),
                // !neighborhood.get(local - IVec3::Z),
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
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct TerrainMaterial {
    #[texture(0, dimension = "2d_array")]
    #[sampler(1)]
    texture: Handle<Image>,
}
const ATTRIBUTE_TEXTURE_INDEX: MeshVertexAttribute =
    MeshVertexAttribute::new("TextureIndex", 2760892297209218923, VertexFormat::Uint32);

impl Material for TerrainMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/terrain_material.wgsl".into()
    }
    fn vertex_shader() -> ShaderRef {
        "shaders/terrain_material.wgsl".into()
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
    visible: Sides<bool>,
    texture: Sides<u32>,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    texture_uvs: &mut Vec<[f32; 2]>,
    texture_indices: &mut Vec<u32>,
) {
    let position = Sides {
        x_pos: [
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 0.0],
            [1.0, 1.0, 1.0],
        ],
        x_neg: [
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 1.0],
        ],
        y_pos: [
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 1.0],
            [1.0, 1.0, 0.0],
            [1.0, 1.0, 1.0],
        ],
        y_neg: [
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
        ],
        z_pos: [
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
        ],
        z_neg: [
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
        ],
    }
    .map(|block| block.map(|[x, y, z]| [x + tr.x, y + tr.y, z + tr.z]));
    let normal: Sides<[f32; 3]> = Sides::AXIS.map(|v| v.as_vec3().into());
    let uv = Sides {
        x_pos: [[0.0, 1.0], [1.0, 1.0], [0.0, 0.0], [1.0, 0.0]],
        x_neg: [[0.0, 1.0], [1.0, 1.0], [0.0, 0.0], [1.0, 0.0]],
        y_pos: [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]],
        y_neg: [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]],
        z_pos: [[1.0, 1.0], [1.0, 0.0], [0.0, 1.0], [0.0, 0.0]],
        z_neg: [[1.0, 1.0], [1.0, 0.0], [0.0, 1.0], [0.0, 0.0]],
    };
    let vertex = Sides {
        x_pos: [0, 2, 1, 1, 2, 3],
        x_neg: [0, 1, 2, 1, 3, 2],
        y_pos: [0, 1, 2, 1, 3, 2],
        y_neg: [0, 2, 1, 1, 2, 3],
        z_pos: [0, 2, 1, 1, 2, 3],
        z_neg: [0, 1, 2, 1, 3, 2],
    };

    for side in Side::ALL {
        if visible[side] {
            let index = positions.len() as u32;
            positions.extend(position[side]);
            normals.extend([normal[side]; 4]);
            indices.extend(vertex[side].map(|vertex| index + vertex));
            texture_uvs.extend(uv[side]);
            texture_indices.extend([texture[side]; 4]);
        }
    }
}
