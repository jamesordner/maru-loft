use std::iter::zip;

use glam::Vec3;

pub use crate::sketch::SketchDescriptor;
use crate::{
    loft::{Loft, LoftBuilder},
    sketch::{Sketch, VertexId},
    util::{SketchPair, radial_error},
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

pub struct Lofter {
    sketches: Vec<Sketch>,
    /// Mappings for each pair of sketches. There will always be one-fewer
    /// mappings than the number of sketches.
    loft_maps: Vec<Loft>,
}

impl Default for Lofter {
    fn default() -> Self {
        let mut lofter = Self {
            sketches: Default::default(),
            loft_maps: Default::default(),
        };

        let vertices = vec![
            Vec3::new(1., 0., 0.),
            Vec3::new(0., 1., 0.),
            Vec3::new(-1., 0., 0.),
            Vec3::new(0., -1., 0.),
        ];

        lofter.push_sketch(&SketchDescriptor {
            vertices: vertices.clone(),
            relative_position: Vec3::ZERO,
            rotation: Vec3::ZERO,
        });

        lofter.push_sketch(&SketchDescriptor {
            vertices,
            relative_position: Vec3::new(0., 0., 3.),
            rotation: Vec3::ZERO,
        });

        lofter.loft(&Default::default());

        lofter
    }
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

    /// Iterates over all vertices in a sketch, in CCW order.
    pub fn vertices_mut<F>(&mut self, sketch_index: usize, mut f: F)
    where
        F: FnMut((VertexId, &mut Vec3)),
    {
        let sketch = &mut self.sketches[sketch_index];

        for id in &sketch.vertex_order {
            f((*id, &mut sketch.vertex_map.get_mut(id).unwrap()));
        }
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
    }

    /// Returns a vertex buffer containing interleaved vertex positions and
    /// colors.
    ///
    /// `[Vec3; 2] == vertex [position, color]`
    /// `[[Vec3; 2]; 3] == triangle with three vertices`
    pub fn vertex_buffer(&self) -> Vec<[[Vec3; 2]; 3]> {
        let mut vertex_buffer = Vec::new();

        let sketches = self.sketches.windows(2);

        for (loft_map, sketches) in zip(&self.loft_maps, sketches) {
            let sketches = SketchPair::new(&sketches[0], &sketches[1]);
            loft_map.append_vertex_buffer(&mut vertex_buffer, sketches);
        }

        vertex_buffer
    }
}

fn loft_sketches(sketches: SketchPair<&Sketch>, options: &LoftOptions) -> Loft {
    let mut loft_map_builder = LoftBuilder::new(sketches);

    // Get edge candidates, which are all combinations of vertices between
    // sketches.
    let mut edge_candidates = edge_candidates(sketches);

    // Sort edge candidates by increasing radial error.
    edge_candidates.sort_unstable_by(|a, b| a.radial_error.total_cmp(&b.radial_error));

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

    loft_map_builder.build(max_radial_error)
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

    a.vertex_order
        .iter()
        .map(|&id| (id, a.vertex_rotated(id)))
        .flat_map(|v_a| {
            b.vertex_order
                .iter()
                .map(move |&id| (v_a, (id, b.vertex_rotated(id))))
        })
        .map(|(v_a, v_b)| EdgeCandidate {
            radial_error: radial_error(&v_a.1, &v_b.1),
            vertices: SketchPair::new(v_a.0, v_b.0),
        })
        .collect()
}
