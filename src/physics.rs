use bevy::{math::NormedVectorSpace, prelude::*};

use crate::{
    // GizmosExt,
    ray_travel::RayTraveler,
    swizzle::{Dim3, Dim3Selector},
    terrain::{ChunkBlocks, ChunksIndex},
};

#[derive(SystemSet, Clone, PartialEq, Eq, Debug, Hash)]
pub struct ApplyPhysics;
pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (apply_gravity, damp_velocity, apply_velocity)
                .chain()
                .in_set(ApplyPhysics),
        );
    }
}

#[derive(Component)]
pub struct Collider {
    pub size: Vec3,
    pub anchor: Vec3,
}

#[derive(Component)]
pub struct Velocity {
    pub linear: Vec3,
}

#[derive(Component)]
pub struct Grounded;

fn damp_velocity(collider: Query<(&mut Velocity, Has<Grounded>), With<Collider>>, time: Res<Time>) {
    for (mut velocity, grounded) in collider {
        let rate: f32 = if grounded { 0.7 } else { 0.9 };
        velocity.linear.x *= rate.powf(time.delta_secs() + 1.0);
        velocity.linear.z *= rate.powf(time.delta_secs() + 1.0);
    }
}

fn apply_gravity(velocity: Query<&mut Velocity>, time: Res<Time>) {
    for mut velocity in velocity {
        velocity.linear += Vec3::NEG_Y * 40.0 * time.delta_secs();
    }
}

fn apply_velocity(
    // mut gizmos: Gizmos,
    chunks: Res<ChunksIndex>,
    blocks: Query<&ChunkBlocks>,
    collider: Query<(Entity, &mut Transform, &Collider, &mut Velocity)>,
    time: Res<Time>,
    mut commands: Commands,
) {
    for (entity, mut tr, cl, mut vl) in collider {
        // which side of the collider is advancing
        let corner_select = Vec3 {
            x: if vl.linear.x < 0.0 { 0.0 } else { cl.size.x },
            y: if vl.linear.y < 0.0 { 0.0 } else { cl.size.y },
            z: if vl.linear.z < 0.0 { 0.0 } else { cl.size.z },
        };
        let corner_active = tr.translation - cl.anchor + corner_select;

        // the current translation
        let mut shift = vl.linear * time.delta_secs();

        // let keyframe = time.elapsed_secs().rem_euclid(1.0);
        // gizmos.aabb(corner_low, cl.size, Color::srgb(1.0, 0.3, 0.2));
        // gizmos.aabb(
        //     corner_low + shift * keyframe,
        //     cl.size,
        //     Color::srgb(1.0, 0.5, 0.0),
        // );
        // gizmos.line(
        //     corner_active,
        //     corner_active + shift,
        //     Color::srgb(1.0, 0.5, 0.0),
        // );

        let mut grounded = false;

        'search: while let Ok(dir) = shift.try_into() {
            let length = shift.norm();
            for step in RayTraveler::new(corner_active, dir, length) {
                // to avoid code duplication, each symetric situation through dimension permutation is made identic by a reversible swizzle
                let dim = match step.dir {
                    IVec3::X | IVec3::NEG_X => Dim3::X,
                    IVec3::Y | IVec3::NEG_Y => Dim3::Y,
                    IVec3::Z | IVec3::NEG_Z => Dim3::Z,
                    _ => unreachable!(),
                };

                let (_, [plane_u, plane_v]) = (step.at - corner_select).split(dim);
                let (_, [size_u, size_v]) = cl.size.split(dim);

                // on the UV plane, we select all voxels covered by the side of the collider
                for u in plane_u.floor() as i32..=(plane_u + size_u).floor() as i32 {
                    for v in plane_v.floor() as i32..=(plane_v + size_v).floor() as i32 {
                        // we find the global coordinate of each voxel
                        let selected = IVec3::compose(dim, step.to[dim], [u, v]);

                        // if a block is present, a collision occur
                        if chunks.get(blocks, selected) {
                            // we correct the vector component to stop at the collision
                            shift[dim] *= step.time / length;
                            // we stop slightly before the collision
                            shift[dim] -= dir[dim].signum() * 1e-4;
                            // the collision absorbs all kinetic energy
                            vl.linear[dim] = 0.0;

                            if step.dir == IVec3::NEG_Y {
                                grounded = true;
                            }

                            // we restart the collision search with the corrected shift
                            continue 'search;
                        }
                    }
                }
            }
            // no more collisions are detected
            break 'search;
        }

        // gizmos.aabb(
        //     corner_low + shift * keyframe,
        //     cl.size,
        //     Color::srgb(0.8, 0.0, 1.0),
        // );
        // gizmos.line(
        //     corner_active,
        //     corner_active + shift,
        //     Color::srgb(0.8, 0.0, 1.0),
        // );

        if grounded {
            commands.entity(entity).insert_if_new(Grounded);
        } else {
            commands.entity(entity).remove::<Grounded>();
        }

        if chunks.get(blocks, (corner_active + shift).floor().as_ivec3()) {
            println!("collider tunneling");
            println!(" - pos    {:.10}", corner_active);
            println!(" - shift* {:.10}", shift);
            println!(" - pos*   {:.10}", corner_active + shift);
            println!();
            commands.entity(entity).remove::<Velocity>();
            return;
        }

        tr.translation += shift;
    }
}
