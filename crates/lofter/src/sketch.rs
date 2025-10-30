use std::collections::HashMap;

use glam::Vec3;

pub type VertexId = u32;

/// A simplified sketch data structure, used by callers to insert initial
/// sketches.
pub struct SketchDescriptor {
    pub vertices: Vec<Vec3>,
    pub relative_position: Vec3,
    pub rotation: f32,
}

pub struct Sketch {
    /// The vertices of the sketch. These are stored in a HashMap so that their
    /// IDs are stable on insertion/removal (as opposed to indices in a `Vec`).
    pub vertex_map: HashMap<VertexId, Vec3>,
    /// The order of vertices, in CCW order.
    pub vertex_order: Vec<VertexId>,
    /// The relative offset from the previous sketch in the loft, or from the
    /// origin if this is the bottommost sketch.
    pub relative_position: Vec3,
    /// Rotation along the normal axis, in radians.
    pub rotation: f32,
}
