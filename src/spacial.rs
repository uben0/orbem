use std::ops::Index;

use bevy::math::IVec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    XPos,
    XNeg,
    YPos,
    YNeg,
    ZPos,
    ZNeg,
}
impl Side {
    pub const ALL: [Self; 6] = [
        Self::XPos,
        Self::XNeg,
        Self::YPos,
        Self::YNeg,
        Self::ZPos,
        Self::ZNeg,
    ];
}
pub struct Sides<T> {
    pub x_pos: T,
    pub x_neg: T,
    pub y_pos: T,
    pub y_neg: T,
    pub z_pos: T,
    pub z_neg: T,
}
impl Sides<IVec3> {
    pub const AXIS: Self = Sides {
        x_pos: IVec3::X,
        x_neg: IVec3::NEG_X,
        y_pos: IVec3::Y,
        y_neg: IVec3::NEG_Y,
        z_pos: IVec3::Z,
        z_neg: IVec3::NEG_Z,
    };
}
impl<T> Sides<T> {
    pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> Sides<U> {
        Sides {
            x_pos: f(self.x_pos),
            x_neg: f(self.x_neg),
            y_pos: f(self.y_pos),
            y_neg: f(self.y_neg),
            z_pos: f(self.z_pos),
            z_neg: f(self.z_neg),
        }
    }
}
impl<T> Index<Side> for Sides<T> {
    type Output = T;

    fn index(&self, index: Side) -> &Self::Output {
        match index {
            Side::XPos => &self.x_pos,
            Side::XNeg => &self.x_neg,
            Side::YPos => &self.y_pos,
            Side::YNeg => &self.y_neg,
            Side::ZPos => &self.z_pos,
            Side::ZNeg => &self.z_neg,
        }
    }
}
impl<T> From<Sides<T>> for [T; 6] {
    fn from(
        Sides {
            x_pos,
            x_neg,
            y_pos,
            y_neg,
            z_pos,
            z_neg,
        }: Sides<T>,
    ) -> Self {
        [x_pos, x_neg, y_pos, y_neg, z_pos, z_neg]
    }
}
impl<'a, T> From<&'a Sides<T>> for [&'a T; 6] {
    fn from(
        Sides {
            x_pos,
            x_neg,
            y_pos,
            y_neg,
            z_pos,
            z_neg,
        }: &'a Sides<T>,
    ) -> Self {
        [x_pos, x_neg, y_pos, y_neg, z_pos, z_neg]
    }
}
impl<T> IntoIterator for Sides<T> {
    type Item = T;

    type IntoIter = <[T; 6] as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        <[T; 6]>::from(self).into_iter()
    }
}
impl<'a, T> IntoIterator for &'a Sides<T> {
    type Item = &'a T;

    type IntoIter = <[&'a T; 6] as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        <[&'a T; 6]>::from(self).into_iter()
    }
}

pub struct Neighborhood<T> {
    pub zero: T,
    pub x_pos: T,
    pub x_neg: T,
    pub y_pos: T,
    pub y_neg: T,
    pub z_pos: T,
    pub z_neg: T,
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
    pub fn as_array(&self) -> [&T; 7] {
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
    pub fn all(&self, f: impl FnMut(&T) -> bool) -> bool {
        self.as_array().into_iter().all(f)
    }
    pub fn try_map<U>(self, mut f: impl FnMut(T) -> Option<U>) -> Option<Neighborhood<U>> {
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
