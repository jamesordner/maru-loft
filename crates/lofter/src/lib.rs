use std::collections::HashMap;

use crate::sketch::{Sketch, VertexId};

pub mod sketch;

pub struct LoftOptions {
    /// If `true`, the loft will use as few edges as possible. For example, when
    /// lofting One WTC, if `true`, the loft will use four connecting edges. If
    /// `false`, the loft will use eight connecting edges.
    pub minimize_edges: bool,
    pub create_intermediate_vertices: bool,
}

impl Default for LoftOptions {
    fn default() -> Self {
        Self {
            minimize_edges: true,
            create_intermediate_vertices: true,
        }
    }
}

pub struct Lofter {
    pub sketches: Vec<Sketch>,
    vertex_mapping: Vec<VertexMapping>,
}

impl Lofter {
    /// Create the loft shape.
    pub fn loft(&mut self, options: &LoftOptions) {}
}

struct Face {
    /// Vertex range of "previous" sketch.
    vertex_range_prev_sketch: (VertexId, VertexId),
    /// Vertex range of "next" sketch.
    vertex_range_next_sketch: (VertexId, VertexId),
}

struct VertexMapping {
    from_index: usize,
    to_index: usize,
}
