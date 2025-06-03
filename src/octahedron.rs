use bevy::math::{Vec3, Vec3Swizzles};

#[derive(Debug, Clone, Copy)]
pub enum Zone {
    Pillar,
    PyramidX,
    PyramidY,
    PyramidZ,
    CornerX,
    CornerY,
    CornerZ,
}

const fn zone(point: Vec3) -> Zone {
    debug_assert!(point.x >= 0.0);
    debug_assert!(point.y >= 0.0);
    debug_assert!(point.z >= 0.0);
    match (
        point.x + point.y - point.z * 2.0,
        point.y + point.z - point.x * 2.0,
        point.z + point.x - point.y * 2.0,
        point.x - point.y,
        point.y - point.z,
        point.z - point.x,
    ) {
        (..=1.0, ..=1.0, ..=1.0, _, _, _) => Zone::Pillar,
        (1.0.., _, _, -1.0..=1.0, _, _) => Zone::CornerZ,
        (_, 1.0.., _, _, -1.0..=1.0, _) => Zone::CornerX,
        (_, _, 1.0.., _, _, -1.0..=1.0) => Zone::CornerY,
        (_, _, _, ..=-1.0, 1.0.., _) => Zone::PyramidY,
        (_, _, _, _, ..=-1.0, 1.0..) => Zone::PyramidZ,
        (_, _, _, 1.0.., _, ..=-1.0) => Zone::PyramidX,
        _ => panic!(),
    }
}

// fn nearest(point: Vec3) -> Vec3 {
//     if point.element_sum() <= 1.0 {
//         return point;
//     }
//     match zone(point) {
//         Zone::Pillar => point - Vec3::splat((point.element_sum() - 1.0) / 3.0),
//         Zone::PyramidX => Vec3::X,
//         Zone::PyramidY => Vec3::Y,
//         Zone::PyramidZ => Vec3::Z,
//         Zone::CornerX => {
//             let d = (point.yz().element_sum() - 1.0) / 2.0;
//             Vec3 {
//                 x: 0.0,
//                 y: point.y - d,
//                 z: point.z - d,
//             }
//         }
//         Zone::CornerY => {
//             let d = (point.xz().element_sum() - 1.0) / 2.0;
//             Vec3 {
//                 x: point.x - d,
//                 y: 0.0,
//                 z: point.z - d,
//             }
//         }
//         Zone::CornerZ => {
//             let d = (point.xy().element_sum() - 1.0) / 2.0;
//             Vec3 {
//                 x: point.x - d,
//                 y: point.y - d,
//                 z: 0.0,
//             }
//         }
//     }
// }

// fn nearest_negatives(point: Vec3) -> Vec3 {
//     point.signum() * nearest(point.abs())
// }

// pub fn nearest_any(center: Vec3, radius: f32, point: Vec3) -> Vec3 {
//     if radius > 0.0 {
//         nearest_negatives((point - center) / radius) * radius + center
//     } else {
//         center
//     }
// }

pub fn distance(center: Vec3, radius: f32, point: Vec3) -> f32 {
    if radius > 0.0 {
        let point = ((point - center) / radius).abs();
        if point.element_sum() <= 1.0 {
            return 0.0;
        }
        let nearest = match zone(point) {
            Zone::Pillar => point - Vec3::splat((point.element_sum() - 1.0) / 3.0),
            Zone::PyramidX => Vec3::X,
            Zone::PyramidY => Vec3::Y,
            Zone::PyramidZ => Vec3::Z,
            Zone::CornerX => {
                let d = (point.yz().element_sum() - 1.0) / 2.0;
                Vec3 {
                    x: 0.0,
                    y: point.y - d,
                    z: point.z - d,
                }
            }
            Zone::CornerY => {
                let d = (point.xz().element_sum() - 1.0) / 2.0;
                Vec3 {
                    x: point.x - d,
                    y: 0.0,
                    z: point.z - d,
                }
            }
            Zone::CornerZ => {
                let d = (point.xy().element_sum() - 1.0) / 2.0;
                Vec3 {
                    x: point.x - d,
                    y: point.y - d,
                    z: 0.0,
                }
            }
        };
        point.distance(nearest) * radius
    } else {
        point.distance(center)
    }
}
