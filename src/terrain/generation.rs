use std::fs::File;

use bevy::{
    ecs::resource::Resource,
    math::{IVec2, IVec3, Vec2, Vec3Swizzles},
    platform::collections::HashMap,
};
use serde::Deserialize;

use crate::terrain::{Block, CHUNK_WIDTH, local_to_global};

fn harmonic_noise(harmonic: &[(f32, f32)], at: Vec2) -> f32 {
    let mut value = 0.0;
    let mut span = 0.0;
    for &(frequency, amplitude) in harmonic {
        value += amplitude * noisy_bevy::simplex_noise_2d(at / frequency);
        span += amplitude;
    }
    value / span
}

fn sigmoid(x: f32) -> f32 {
    x.exp() / ((-x).exp() + x.exp())
}
fn logistic(x: f32) -> f32 {
    (1.0 + x.exp()).ln()
}

#[derive(Resource, Deserialize)]
pub struct TerrainGenerator {
    bedrock_harmonics: Vec<(f32, f32)>,
    relief_harmonics: Vec<(f32, f32)>,
}

struct Profile {
    bedrock: i32,
    // relief: i32,
    sediment: i32,
}
// impl Default for TerrainGenerator {
//     fn default() -> Self {
//         Self {
//             bedrock_harmonics: Vec::from([(20.0, 0.5), (100.0, 4.0), (1000.0, 16.0)]),
//             relief_harmonics: Vec::from([(50.0, 1.0), (80.0, 1.0)]),
//         }
//     }
// }
impl TerrainGenerator {
    pub fn load_from_file() -> Self {
        serde_json::from_reader(File::open("assets/generation.json").unwrap()).unwrap()
    }
    fn sample(&self, coord: IVec2) -> Profile {
        let bedrock = harmonic_noise(&self.bedrock_harmonics, coord.as_vec2());
        let dry = logistic(bedrock * 4.0 - 5.0);
        let bedrock = sigmoid(bedrock * 2.4 + 0.4);
        let bedrock = bedrock * 40.0 - 32.0;
        // TODO: change noise to be between 0 and 1
        let relief = harmonic_noise(&self.relief_harmonics, coord.as_vec2()) + 1.0;
        let relief = relief.powi(2);
        let relief = relief * dry;
        let relief = relief * 80.0;
        // Profile {
        //     bedrock: (relief * 20.0) as i32,
        //     relief: 0,
        //     sediment: 0,
        // }
        Profile {
            bedrock: (bedrock + relief) as i32,
            sediment: 3,
        }
    }
    pub fn generate(&self, chunk: IVec3) -> HashMap<IVec3, Block> {
        let mut blocks = HashMap::new();
        for x in 0..CHUNK_WIDTH {
            for z in 0..CHUNK_WIDTH {
                let global = local_to_global(chunk, IVec3 { x, y: 0, z });
                let profile = self.sample(global.xz());
                let elevation = profile.bedrock;
                let elevation_relative = elevation - chunk.y * CHUNK_WIDTH;
                for y in 0..elevation_relative.min(CHUNK_WIDTH) {
                    blocks.insert(IVec3 { x, y, z }, Block::Stone);
                }
                for (y, i) in (elevation_relative..)
                    .zip(0..profile.sediment)
                    .filter(|&(y, _)| y >= 0 && y < CHUNK_WIDTH)
                {
                    let block = if elevation < 2 {
                        Block::Sand
                    } else if i + 1 == profile.sediment {
                        Block::Grass
                    } else {
                        Block::Dirt
                    };
                    blocks.insert(IVec3 { x, y, z }, block);
                }
            }
        }
        blocks
    }
}
