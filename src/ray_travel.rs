use std::cmp::Ordering;

use arrayvec::ArrayVec;
use bevy::math::{Dir3, IVec3, Vec3};

struct AxisTraveler {
    next: f32,
    step: f32,
    dir: IVec3,
}

pub struct RayTraveler {
    axis_travelers: ArrayVec<AxisTraveler, 3>,
    time: f32,
    limit: f32,
    origin: Vec3,
    ray: Dir3,
}

impl RayTraveler {
    pub fn new(origin: Vec3, ray: Dir3, limit: f32) -> Self {
        Self {
            axis_travelers: [
                (origin.x, ray.x, IVec3::X),
                (origin.y, ray.y, IVec3::Y),
                (origin.z, ray.z, IVec3::Z),
            ]
            .into_iter()
            .filter_map(|(origin, ray, dir)| match ray.partial_cmp(&0.0)? {
                Ordering::Less => Some(AxisTraveler {
                    next: (origin - origin.floor()) / ray.abs(),
                    step: 1.0 / ray.abs(),
                    dir,
                }),
                Ordering::Equal => None,
                Ordering::Greater => Some(AxisTraveler {
                    next: (origin.ceil() - origin) / ray.abs(),
                    step: 1.0 / ray.abs(),
                    dir: -dir,
                }),
            })
            .collect(),
            time: 0.0,
            limit,
            origin,
            ray,
        }
    }
}

impl Iterator for RayTraveler {
    type Item = (IVec3, IVec3);

    fn next(&mut self) -> Option<Self::Item> {
        const EPSILON: f32 = 1e-4;
        if self.time > self.limit {
            return None;
        }
        let axis_traveler = self
            .axis_travelers
            .iter_mut()
            .min_by(|lhs, rhs| lhs.next.partial_cmp(&rhs.next).unwrap())?;
        self.time = axis_traveler.next;
        axis_traveler.next += axis_traveler.step;
        Some((
            (self.origin + (self.time + EPSILON) * self.ray).as_ivec3(),
            axis_traveler.dir,
        ))
    }
}
