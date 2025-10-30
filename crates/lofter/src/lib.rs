use glam::Vec3;

pub use crate::sketch::SketchDescriptor;
use crate::sketch::{Sketch, VertexId};

mod sketch;

pub struct LoftOptions {
    /// Edge greediness is the likelihood that the lofting algorithm will create
    /// additional edges between sketches. It is in the range [0, 1]. For
    /// example, when recreating One WTC, a greediness of 0 will create four
    /// connecting edges, resulting in a twisted shape. A greediness of 1 would
    /// result in the desired shape with eight connecting edges.
    pub edge_greediness: f32,
    pub create_intermediate_vertices: bool,
}

impl Default for LoftOptions {
    fn default() -> Self {
        Self {
            edge_greediness: 0.,
            create_intermediate_vertices: true,
        }
    }
}

pub struct Lofter {
    sketches: Vec<Sketch>,
    /// Mappings for each pair of sketches. There will always be one-fewer
    /// mappings than the number of sketches.
    loft_maps: Vec<LoftMap>,
}

impl Lofter {
    pub fn push_sketch(&mut self, sketch: &SketchDescriptor) {
        self.insert_sketch(self.sketches.len(), sketch);
    }

    pub fn insert_sketch(&mut self, sketch_index: usize, sketch: &SketchDescriptor) {}

    pub fn remove_sketch(&mut self, sketch_index: usize) {}

    pub fn sketch_rotation(&self, sketch_index: usize) -> Option<f32> {
        let sketch = self.sketches.get(sketch_index)?;

        Some(sketch.rotation)
    }

    pub fn set_sketch_rotation(&mut self, sketch_index: usize, rotation: f32) {
        let Some(sketch) = self.sketches.get_mut(sketch_index) else {
            return;
        };

        sketch.rotation = rotation;
    }

    pub fn sketch_relative_position(&self, sketch_index: usize) -> Option<&Vec3> {
        let sketch = self.sketches.get(sketch_index)?;

        Some(&sketch.relative_position)
    }

    pub fn set_sketch_relative_position(&mut self, sketch_index: usize, relative_position: &Vec3) {
        let Some(sketch) = self.sketches.get_mut(sketch_index) else {
            return;
        };

        sketch.relative_position = *relative_position;
    }

    pub fn insert_vertex(&mut self, sketch_index: usize, between_vertices: (VertexId, VertexId)) {}

    pub fn remove_vertex(&mut self, sketch_index: usize, vertex_id: VertexId) {}

    /// Returns an iterator over all vertices in a sketch, in CCW order.
    pub fn vertices(&self, sketch_index: usize) -> Option<impl Iterator<Item = (VertexId, &Vec3)>> {
        let sketch = self.sketches.get(sketch_index)?;

        Some(
            sketch
                .vertex_order
                .iter()
                .map(|id| (*id, &sketch.vertex_map[id])),
        )
    }

    pub fn get_vertex(&self, sketch_index: usize, vertex_id: VertexId) -> Option<&Vec3> {
        self.sketches.get(sketch_index)?.vertex_map.get(&vertex_id)
    }

    pub fn get_vertex_mut(
        &mut self,
        sketch_index: usize,
        vertex_id: VertexId,
    ) -> Option<&mut Vec3> {
        self.sketches
            .get_mut(sketch_index)?
            .vertex_map
            .get_mut(&vertex_id)
    }

    /// Create (or recreate) the loft shape.
    pub fn loft(&mut self, options: &LoftOptions) {}
}

/// Metadata describing how two sketches are lofted.
struct LoftMap {
    sections: Vec<LoftSection>,
}

/// A "section" of a loft, connecting a range of vertices from one sketch to a
/// range of vertices in another sketch. This vertex range may form an edge or a
/// series of edges, or the range of vertices may contain only a single vertex.
struct LoftSection {
    /// Range of vertices in the original sketches in this section (range
    /// inclusive).
    vertex_ranges: LoftPair<(VertexId, VertexId)>,
    /// Explicit vertices used for edge connections in this section. There may
    /// be additional, generated loft vertices which are not present in the set
    /// of vertices in the original sketches.
    loft_vertices: LoftPair<Vec<LoftVertex>>,
    /// Explicit edge connections between loft vertices in this section.
    edges: Vec<LoftPair<usize>>,
}

/// A vertex used to form an edge to another sketch. A loft vertex may lie along
/// the edge of a sketch, i.e. it might not be present in the original set of
/// sketch vertices.
struct LoftVertex {
    edge: (VertexId, VertexId),
    /// A value in range [0, 1] which determines where the vertex lies along the
    /// edge.
    slide: f32,
}

/// Paired values. Used for i.e. loft edge connections or paired vertex ranges
/// between sketches.
struct LoftPair<T> {
    lower_sketch: T,
    upper_sketch: T,
}
