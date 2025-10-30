use std::collections::HashMap;

use glam::Vec3;

pub type VertexId = u32;

pub struct Sketch {
    /// The vertices of the sketch. These are stored in a HashMap so that their
    /// IDs are stable.
    vertex_map: HashMap<VertexId, Vec3>,
    /// The order of vertices, in CCW order.
    vertex_order: Vec<VertexId>,
    /// The relative offset from the previous sketch in the loft. At the moment,
    /// only vertical offset is considered (x and y axes are discarded).
    position_from_prev_sketch: Vec3,
    /// Rotation along the normal axis, in radians.
    rotation: f32,
}
