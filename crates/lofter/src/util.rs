use std::ops::{Index, IndexMut};

use glam::{Vec3, Vec3Swizzles};

/// Paired values. Used for i.e. edge connections or paired vertex ranges
/// between sketches.
#[derive(Copy, Clone, Debug)]
pub struct SketchPair<T> {
    pub lower: T,
    pub upper: T,
}

impl<T> SketchPair<T> {
    pub fn new(lower: T, upper: T) -> Self {
        Self { lower, upper }
    }

    /// Duplicate `val` to both values in the pair.
    pub fn splat(val: T) -> Self
    where
        T: Clone,
    {
        Self {
            lower: val.clone(),
            upper: val,
        }
    }

    pub fn as_ref(&self) -> SketchPair<&T> {
        SketchPair {
            lower: &self.lower,
            upper: &self.upper,
        }
    }

    pub fn as_mut(&mut self) -> SketchPair<&mut T> {
        SketchPair {
            lower: &mut self.lower,
            upper: &mut self.upper,
        }
    }

    pub fn map<F, U>(self, mut f: F) -> SketchPair<U>
    where
        F: FnMut(T) -> U,
    {
        SketchPair {
            lower: f(self.lower),
            upper: f(self.upper),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        [&self.lower, &self.upper].into_iter()
    }

    pub fn zip<U>(self, other: SketchPair<U>) -> SketchPair<(T, U)> {
        SketchPair {
            lower: (self.lower, other.lower),
            upper: (self.upper, other.upper),
        }
    }
}

impl<T> Index<usize> for SketchPair<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.lower,
            1 => &self.upper,
            _ => panic!("SketchPair index out of bounds! Must be 0 or 1."),
        }
    }
}

impl<T> IndexMut<usize> for SketchPair<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0 => &mut self.lower,
            1 => &mut self.upper,
            _ => panic!("SketchPair index out of bounds! Must be 0 or 1."),
        }
    }
}

impl<T> From<(T, T)> for SketchPair<T> {
    fn from(value: (T, T)) -> Self {
        Self::new(value.0, value.1)
    }
}

/// Returns the radial difference of two points in polar coordinates from the
/// origin along the z axis, in radians.
pub fn radial_error(a: &Vec3, b: &Vec3) -> f32 {
    a.xy().angle_to(b.xy()).abs()
}
