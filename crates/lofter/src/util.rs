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
}

impl<T> From<(T, T)> for SketchPair<T> {
    fn from(value: (T, T)) -> Self {
        Self::new(value.0, value.1)
    }
}
