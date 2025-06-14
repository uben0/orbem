use std::ops::{Index, IndexMut};

use bevy::math::{Dir3, IVec3, Vec3, Vec3Swizzles};

#[derive(Debug, Clone, Copy)]
pub enum Dim3 {
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum Swizzle3 {
    XYZ,
    XZY,
    YXZ,
    YZX,
    ZXY,
    ZYX,
}

pub trait Dim3Selector {
    type Scalar;
    fn split(self, dim: Dim3) -> (Self::Scalar, [Self::Scalar; 2]);
    fn compose(dim: Dim3, it: Self::Scalar, others: [Self::Scalar; 2]) -> Self;
    // fn others(self, dim: Dim3) -> (Self::Scalar, Self::Scalar);
    // fn others_mut(&mut self, dim: Dim3) -> (&mut Self::Scalar, &mut Self::Scalar);
}

impl Dim3Selector for Vec3 {
    type Scalar = f32;

    fn split(self, dim: Dim3) -> (Self::Scalar, [Self::Scalar; 2]) {
        match dim {
            Dim3::X => (self.x, [self.y, self.z]),
            Dim3::Y => (self.y, [self.x, self.z]),
            Dim3::Z => (self.z, [self.x, self.y]),
        }
    }

    fn compose(dim: Dim3, it: Self::Scalar, [u, v]: [Self::Scalar; 2]) -> Self {
        match dim {
            Dim3::X => Self { x: it, y: u, z: v },
            Dim3::Y => Self { x: u, y: it, z: v },
            Dim3::Z => Self { x: u, y: v, z: it },
        }
    }
}
impl Dim3Selector for IVec3 {
    type Scalar = i32;

    fn split(self, dim: Dim3) -> (Self::Scalar, [Self::Scalar; 2]) {
        match dim {
            Dim3::X => (self.x, [self.y, self.z]),
            Dim3::Y => (self.y, [self.x, self.z]),
            Dim3::Z => (self.z, [self.x, self.y]),
        }
    }

    fn compose(dim: Dim3, it: Self::Scalar, [u, v]: [Self::Scalar; 2]) -> Self {
        match dim {
            Dim3::X => Self { x: it, y: u, z: v },
            Dim3::Y => Self { x: u, y: it, z: v },
            Dim3::Z => Self { x: u, y: v, z: it },
        }
    }
}

// impl Swizzle3 {
//     pub fn swap(lhs: Dim3, rhs: Dim3) -> Self {
//         match (lhs, rhs) {
//             (Dim3::X, Dim3::X) => Self::XYZ,
//             (Dim3::X, Dim3::Y) => Self::YXZ,
//             (Dim3::X, Dim3::Z) => Self::ZYX,
//             (Dim3::Y, Dim3::X) => Self::YXZ,
//             (Dim3::Y, Dim3::Y) => Self::XYZ,
//             (Dim3::Y, Dim3::Z) => Self::XZY,
//             (Dim3::Z, Dim3::X) => Self::ZYX,
//             (Dim3::Z, Dim3::Y) => Self::XZY,
//             (Dim3::Z, Dim3::Z) => Self::XYZ,
//         }
//     }
// }

impl Index<Dim3> for Vec3 {
    type Output = f32;

    fn index(&self, index: Dim3) -> &Self::Output {
        match index {
            Dim3::X => &self.x,
            Dim3::Y => &self.y,
            Dim3::Z => &self.z,
        }
    }
}
impl IndexMut<Dim3> for Vec3 {
    fn index_mut(&mut self, index: Dim3) -> &mut Self::Output {
        match index {
            Dim3::X => &mut self.x,
            Dim3::Y => &mut self.y,
            Dim3::Z => &mut self.z,
        }
    }
}
impl Index<Dim3> for IVec3 {
    type Output = i32;

    fn index(&self, index: Dim3) -> &Self::Output {
        match index {
            Dim3::X => &self.x,
            Dim3::Y => &self.y,
            Dim3::Z => &self.z,
        }
    }
}
impl IndexMut<Dim3> for IVec3 {
    fn index_mut(&mut self, index: Dim3) -> &mut Self::Output {
        match index {
            Dim3::X => &mut self.x,
            Dim3::Y => &mut self.y,
            Dim3::Z => &mut self.z,
        }
    }
}
impl Index<Dim3> for Dir3 {
    type Output = f32;

    fn index(&self, index: Dim3) -> &Self::Output {
        match index {
            Dim3::X => &self.x,
            Dim3::Y => &self.y,
            Dim3::Z => &self.z,
        }
    }
}

impl std::ops::Mul<Vec3> for Swizzle3 {
    type Output = Vec3;

    fn mul(self, vec: Vec3) -> Self::Output {
        match self {
            Swizzle3::XYZ => vec.xyz(),
            Swizzle3::XZY => vec.xzy(),
            Swizzle3::YXZ => vec.yxz(),
            Swizzle3::YZX => vec.yzx(),
            Swizzle3::ZXY => vec.zxy(),
            Swizzle3::ZYX => vec.zyx(),
        }
    }
}
impl std::ops::Mul<IVec3> for Swizzle3 {
    type Output = IVec3;

    fn mul(self, vec: IVec3) -> Self::Output {
        match self {
            Swizzle3::XYZ => vec.xyz(),
            Swizzle3::XZY => vec.xzy(),
            Swizzle3::YXZ => vec.yxz(),
            Swizzle3::YZX => vec.yzx(),
            Swizzle3::ZXY => vec.zxy(),
            Swizzle3::ZYX => vec.zyx(),
        }
    }
}
impl std::ops::Mul<Dir3> for Swizzle3 {
    type Output = Dir3;

    fn mul(self, dir: Dir3) -> Self::Output {
        match self {
            Swizzle3::XYZ => dir.xyz(),
            Swizzle3::XZY => dir.xzy(),
            Swizzle3::YXZ => dir.yxz(),
            Swizzle3::YZX => dir.yzx(),
            Swizzle3::ZXY => dir.zxy(),
            Swizzle3::ZYX => dir.zyx(),
        }
        .try_into()
        .unwrap()
    }
}
