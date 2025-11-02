use glam::{Vec3, Vec3Swizzles};

pub use crate::sketch::SketchDescriptor;
use crate::{
    loft::{Loft, LoftBuilder},
    sketch::{Sketch, VertexId},
    util::SketchPair,
};

mod loft;
mod sketch;
mod util;

pub struct LoftOptions {
    /// In degrees.
    pub max_radial_edge_angle: f32,
}

impl Default for LoftOptions {
    fn default() -> Self {
        Self {
            max_radial_edge_angle: 50.,
        }
    }
}

#[derive(Default)]
pub struct Lofter {
    sketches: Vec<Sketch>,
    /// Mappings for each pair of sketches. There will always be one-fewer
    /// mappings than the number of sketches.
    loft_maps: Vec<Loft>,
}

impl Lofter {
    pub fn push_sketch(&mut self, sketch: &SketchDescriptor) {
        self.insert_sketch(self.sketches.len(), sketch);
    }

    pub fn insert_sketch(&mut self, sketch_index: usize, sketch: &SketchDescriptor) {
        self.sketches.insert(sketch_index, sketch.into());
    }

    pub fn remove_sketch(&mut self, sketch_index: usize) {
        self.sketches.remove(sketch_index);
    }

    pub fn sketch_rotation(&self, sketch_index: usize) -> Option<&Vec3> {
        let sketch = self.sketches.get(sketch_index)?;

        Some(&sketch.rotation)
    }

    pub fn set_sketch_rotation(&mut self, sketch_index: usize, rotation: &Vec3) {
        let Some(sketch) = self.sketches.get_mut(sketch_index) else {
            return;
        };

        sketch.rotation = *rotation;
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
    pub fn loft(&mut self, options: &LoftOptions) {
        self.loft_maps = self
            .sketches
            .windows(2)
            .map(|sketches| loft_sketches(SketchPair::new(&sketches[0], &sketches[1]), options))
            .collect();

        dbg!(&self.loft_maps);
    }
}

fn loft_sketches(sketches: SketchPair<&Sketch>, options: &LoftOptions) -> Loft {
    let mut loft_map_builder = LoftBuilder::new(sketches);

    // Get edge candidates, which are all combinations of vertices between
    // sketches.
    let mut edge_candidates = edge_candidates(sketches);

    // Sort edge candidates by increasing radial error.
    edge_candidates.sort_unstable_by(|a, b| a.radial_error.total_cmp(&b.radial_error));

    dbg!(&edge_candidates);

    let max_radial_error = options.max_radial_edge_angle.to_radians();

    // Iterate edge candidates, taking edges as long as they are valid, until
    // radial error > max error.
    for edge_candidate in edge_candidates {
        if edge_candidate.radial_error > max_radial_error {
            break;
        }

        loft_map_builder.try_split_section(edge_candidate.vertices);
    }

    // resolve sections

    loft_map_builder.build()
}

#[derive(Debug)]
struct EdgeCandidate {
    /// The radial angle difference, from the xy origin, of the two vertices.
    radial_error: f32,
    vertices: SketchPair<VertexId>,
}

/// Returns a vector of all combinations of vertices between two sketches.
fn edge_candidates(sketches: SketchPair<&Sketch>) -> Vec<EdgeCandidate> {
    let a = sketches.lower;
    let b = sketches.upper;

    a.vertex_map
        .iter()
        .flat_map(|v_a| b.vertex_map.iter().map(move |v_b| (v_a, v_b)))
        .map(|(v_a, v_b)| EdgeCandidate {
            radial_error: radial_error(v_a.1, v_b.1),
            vertices: SketchPair::new(*v_a.0, *v_b.0),
        })
        .collect()
}

/// Returns the radial difference of two points along the z axis, in radians.
fn radial_error(a: &Vec3, b: &Vec3) -> f32 {
    let a = a.xy().normalize_or_zero();
    let b = b.xy().normalize_or_zero();

    a.dot(b).acos()
}
