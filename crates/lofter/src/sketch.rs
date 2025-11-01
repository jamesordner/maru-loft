use std::collections::HashMap;

use glam::Vec3;

pub type VertexId = u32;

/// A simplified sketch data structure, used by callers to insert initial
/// sketches.
pub struct SketchDescriptor {
    pub vertices: Vec<Vec3>,
    pub relative_position: Vec3,
    pub rotation: Vec3,
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
    /// Rotation, in radians.
    pub rotation: Vec3,
}

impl From<&SketchDescriptor> for Sketch {
    fn from(value: &SketchDescriptor) -> Self {
        let mut vertex_map = HashMap::with_capacity(value.vertices.len());
        let mut vertex_order = Vec::with_capacity(value.vertices.len());

        for (i, &vertex) in value.vertices.iter().enumerate() {
            let vertex_id = i as VertexId;

            vertex_map.insert(vertex_id, vertex);
            vertex_order.push(vertex_id);
        }

        Self {
            vertex_map,
            vertex_order,
            relative_position: value.relative_position,
            rotation: value.rotation,
        }
    }
}
