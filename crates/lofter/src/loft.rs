use std::{array::from_fn, f32::consts::PI};

use glam::{Vec3, Vec3Swizzles};
use rand::{Rng, thread_rng};

use crate::{
    sketch::{Sketch, VertexId},
    util::{SketchPair, radial_error},
};

/// A loft describes how two sketches are connected.
#[derive(Debug)]
pub struct Loft {
    sections: Vec<LoftSection>,

    /// This loft map only exists when no sections could be formed (which
    /// normally contain individual loft maps).
    sectionless_loft_map: Option<Vec<SketchPair<LoftVertex>>>,
}

impl Loft {
    /// Generates a renderable, non-indexed vertex buffer.
    pub fn append_vertex_buffer(
        &self,
        vertex_buffer: &mut Vec<[[Vec3; 2]; 3]>,
        sketches: SketchPair<&Sketch>,
    ) {
        if let Some(loft_map) = &self.sectionless_loft_map {
            let prev_loft_edge = loft_map.last().unwrap();
            let first_loft_edge = [loft_map[0]];
            let loft_edges = loft_map.iter().chain(&first_loft_edge);

            append_iterator(vertex_buffer, sketches, prev_loft_edge, loft_edges);
        } else {
            let prev_loft_edge = self.sections.last().unwrap().loft_edges.last().unwrap();
            let first_loft_edge = [self.sections[0].loft_edges[0]];
            let loft_edges = self
                .sections
                .iter()
                .map(|section| &section.loft_edges)
                .flatten()
                .chain(&first_loft_edge);

            append_iterator(vertex_buffer, sketches, prev_loft_edge, loft_edges);
        };

        fn append_iterator<'a>(
            vertex_buffer: &mut Vec<[[Vec3; 2]; 3]>,
            sketches: SketchPair<&Sketch>,
            mut prev_loft_edge: &'a SketchPair<LoftVertex>,
            loft_edges: impl Iterator<Item = &'a SketchPair<LoftVertex>>,
        ) {
            let mut rng = rand::rng();

            for loft_edge in loft_edges {
                // Color each face a different random color.
                let color = Vec3::from(from_fn(|_| rng.random()));

                if prev_loft_edge.lower == loft_edge.lower {
                    // Tri.
                    vertex_buffer.push([
                        [prev_loft_edge.upper.to_pos(sketches.upper), color],
                        [loft_edge.lower.to_pos(sketches.lower), color],
                        [loft_edge.upper.to_pos(sketches.upper), color],
                    ]);
                } else if prev_loft_edge.upper == loft_edge.upper {
                    // Tri.
                    vertex_buffer.push([
                        [prev_loft_edge.upper.to_pos(sketches.upper), color],
                        [prev_loft_edge.lower.to_pos(sketches.lower), color],
                        [loft_edge.lower.to_pos(sketches.lower), color],
                    ]);
                } else {
                    // Quad.
                    vertex_buffer.push([
                        [prev_loft_edge.upper.to_pos(sketches.upper), color],
                        [prev_loft_edge.lower.to_pos(sketches.lower), color],
                        [loft_edge.lower.to_pos(sketches.lower), color],
                    ]);
                    vertex_buffer.push([
                        [prev_loft_edge.upper.to_pos(sketches.upper), color],
                        [loft_edge.lower.to_pos(sketches.lower), color],
                        [loft_edge.upper.to_pos(sketches.upper), color],
                    ]);
                }

                prev_loft_edge = loft_edge;
            }
        }
    }
}

pub struct LoftBuilder<'a> {
    loft: Loft,
    sketches: SketchPair<&'a Sketch>,
}

impl<'a> LoftBuilder<'a> {
    pub fn new(sketches: SketchPair<&'a Sketch>) -> Self {
        let loft = Loft {
            sections: Vec::new(),
            sectionless_loft_map: None,
        };

        Self { loft, sketches }
    }

    /// Splits, or subdivides, a section into two different sections, creating
    /// an edge formed by the two passed-in vertices. If no sections exist
    /// yet, this creates a single section encompassing the entirety of both
    /// sketches.
    ///
    /// If the passed-in vertices lie in two different sections, the split is
    /// invalid, and the function returns without modifying the loft.
    pub fn try_split_section(&mut self, edge_candidate_vertices: SketchPair<VertexId>) {
        // Check if this is the first split.
        if self.loft.sections.is_empty() {
            // Create an initial section encompassing the entirety of the
            // sketches.
            let initial_section = LoftSection::uninitialized_with_entire_ranges((
                edge_candidate_vertices.lower,
                edge_candidate_vertices.upper,
            ));

            self.loft.sections.push(initial_section);

            return;
        }

        // Make sure the edge candidate vertices lie within the same section. If
        // they don't, this is not a valid split.
        let Some(section_index) = self.edge_candidate_section_index(edge_candidate_vertices) else {
            // Invalid split.
            return;
        };

        // Split the section by removing it and inserting two new sections.
        let section = self.loft.sections.remove(section_index);

        let mut split_ranges = section
            .sketch_vertex_ranges
            .zip(edge_candidate_vertices)
            .map(|(range, vert)| range.split_at(vert));

        // If the lower range split still covers the whole sketch, we need
        // to check if the lower ranges need to be swapped to match the upper
        // splits.
        if split_ranges.lower.0.covers_entire_sketch
            && split_ranges.upper.0.iter(self.sketches.upper).count() == 2
        {
            std::mem::swap(&mut split_ranges.lower.0, &mut split_ranges.lower.1);
        }
        // Same check for the upper splits.
        else if split_ranges.upper.0.covers_entire_sketch
            && split_ranges.lower.0.iter(self.sketches.lower).count() == 2
        {
            std::mem::swap(&mut split_ranges.upper.0, &mut split_ranges.upper.1);
        }

        let new_section_a =
            LoftSection::uninitialized_with_ranges((split_ranges.lower.0, split_ranges.upper.0));

        let new_section_b =
            LoftSection::uninitialized_with_ranges((split_ranges.lower.1, split_ranges.upper.1));

        // Use `insert` instead of `push` so that sections remain sorted in CCW
        // order.
        self.loft.sections.insert(section_index, new_section_a);
        self.loft.sections.insert(section_index + 1, new_section_b);
    }

    /// Returns the index in `Loft::sections` of the section containing the edge
    /// candidate vertices. If the vertices lie in two different sections, the
    /// edge candidate is invalid and `None` is returned.
    fn edge_candidate_section_index(
        &self,
        edge_candidate_vertices: SketchPair<VertexId>,
    ) -> Option<usize> {
        fn vertex_range_contains_vertex(
            vertex: VertexId,
            vertex_range: &SketchVertexRange,
            sketch: &Sketch,
        ) -> bool {
            vertex_range.iter(sketch).any(|id| vertex == id)
        }

        self.loft.sections.iter().position(|section| {
            vertex_range_contains_vertex(
                edge_candidate_vertices.lower,
                &section.sketch_vertex_ranges.lower,
                self.sketches.lower,
            ) && vertex_range_contains_vertex(
                edge_candidate_vertices.upper,
                &section.sketch_vertex_ranges.upper,
                self.sketches.upper,
            )
        })
    }

    pub fn build(self, max_radial_error: f32) -> Loft {
        let mut loft = self.loft;

        if loft.sections.is_empty() {
            let sketch_vertex_ranges = self
                .sketches
                .map(|sketch| sketch.vertex_order[0])
                .map(|id| SketchVertexRange::entire(id));

            let loft_edges =
                build_loft_edges(sketch_vertex_ranges, self.sketches, max_radial_error);

            loft.sectionless_loft_map = Some(loft_edges);
        } else {
            for section in &mut loft.sections {
                section.build_loft(self.sketches, max_radial_error);
            }
        }

        loft
    }
}

enum LoftType {
    Whole {
        loft_egdes: Vec<SketchPair<LoftVertex>>,
    },
    Sectioned {
        sections: Vec<LoftSection>,
    },
}

/// A "section" of a loft connects a range of vertices from one sketch to a
/// range of vertices in another sketch.
#[derive(Debug)]
struct LoftSection {
    /// Ranges of vertices that this section covers in the original sketches.
    sketch_vertex_ranges: SketchPair<SketchVertexRange>,
    /// All edges between sketches in this section, sorted in CCW order.
    loft_edges: Vec<SketchPair<LoftVertex>>,
}

impl LoftSection {
    /// Create a loft section encompassing the entirety of the sketches.
    fn uninitialized_with_entire_ranges<R>(vertex_ranges: R) -> Self
    where
        R: Into<SketchPair<VertexId>>,
    {
        let sketch_vertex_ranges = vertex_ranges.into().map(SketchVertexRange::entire);

        Self {
            sketch_vertex_ranges,
            loft_edges: Vec::new(),
        }
    }

    fn uninitialized_with_ranges<R>(vertex_ranges: R) -> Self
    where
        R: Into<SketchPair<SketchVertexRange>>,
    {
        Self {
            sketch_vertex_ranges: vertex_ranges.into(),
            loft_edges: Vec::new(),
        }
    }

    /// Initializes the "physical" loft vertices and edges from the section's
    /// vertex ranges.
    fn build_loft(&mut self, sketches: SketchPair<&Sketch>, max_radial_error: f32) {
        self.loft_edges = build_loft_edges(self.sketch_vertex_ranges, sketches, max_radial_error);
    }
}

/// A range of vertices from one sketch to a range of vertices in another
/// sketch, forming a "section". This vertex range may encompass the entirety of
/// both sketches (in which case it is the only section in the loft), the range
/// may form an edge or a series of edges, or the range of vertices may contain
/// only a single vertex.
#[derive(Clone, Copy, Debug)]
struct SketchVertexRange {
    /// Range of vertices in the original sketches in this section (range
    /// inclusive).
    range: (VertexId, VertexId),
    /// This field disambiguates the case where the range values are the same.
    /// When they are the same, it could mean that the range covers only a
    /// single vertex, or the range covers the entirety of the sketch.
    covers_entire_sketch: bool,
}

impl SketchVertexRange {
    fn entire(range_start_and_end: VertexId) -> Self {
        Self {
            range: (range_start_and_end, range_start_and_end),
            covers_entire_sketch: true,
        }
    }

    fn split_at(self, vertex: VertexId) -> (Self, Self) {
        if self.covers_entire_sketch && self.range.0 == self.range.1 && self.range.1 == vertex {
            // This is an edge case we need to handle, where one range still
            // covers the whole sketch, but the other only encompasses a single
            // vertex.
            let mut other = self;
            other.covers_entire_sketch = false;

            // Always return the full range as tuple 0, for easier checks.
            (self, other)
        } else {
            (
                Self {
                    range: (self.range.0, vertex),
                    covers_entire_sketch: false,
                },
                Self {
                    range: (vertex, self.range.1),
                    covers_entire_sketch: false,
                },
            )
        }
    }

    fn iter<'a>(&'a self, sketch: &'a Sketch) -> SketchVertexRangeIter<'a> {
        let next_index = sketch
            .vertex_order
            .iter()
            .position(|&id| self.range.0 == id);

        SketchVertexRangeIter {
            next_index,
            has_visited_first_vertex: false,
            range: self,
            sketch,
        }
    }
}

struct SketchVertexRangeIter<'a> {
    next_index: Option<usize>,
    has_visited_first_vertex: bool,
    range: &'a SketchVertexRange,
    sketch: &'a Sketch,
}

impl Iterator for SketchVertexRangeIter<'_> {
    type Item = VertexId;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.next_index.take()?;
        let vertex_id = self.sketch.vertex_order[index];

        // Only set `self.next_index` if we're not done iterating after
        // returning this vertex id.
        if vertex_id != self.range.range.1
            || (self.range.covers_entire_sketch && !self.has_visited_first_vertex)
        {
            let next_index = (index + 1) % self.sketch.vertex_order.len();
            self.next_index = Some(next_index);
        }

        if vertex_id == self.range.range.0 {
            self.has_visited_first_vertex = true;
        }

        Some(vertex_id)
    }
}

/// A vertex used to form the loft mesh. A loft vertex may lie along the edge
/// of a sketch, i.e. it might not be present in the original set of sketch
/// vertices.
#[derive(Clone, Copy, Debug, PartialEq)]
enum LoftVertex {
    /// A loft vertex at the same position as a sketch's vertex.
    SketchVertex(VertexId),
    /// A loft vertex which lies along a sketch's edge.
    SketchEdge {
        /// Adjacent vertices forming an edge in the original sketch.
        edge: (VertexId, VertexId),
        /// A value in range [0, 1] which determines where the vertex lies along the
        /// sketch's edge.
        edge_length: f32,
    },
}

impl LoftVertex {
    fn to_pos(&self, sketch: &Sketch) -> Vec3 {
        let relative_pos = match self {
            LoftVertex::SketchVertex(id) => sketch.vertex_map[id],
            LoftVertex::SketchEdge { edge, edge_length } => {
                let a = sketch.vertex_map[&edge.0];
                let b = sketch.vertex_map[&edge.1];

                a + (b - a).normalize() * edge_length
            }
        };

        relative_pos + sketch.relative_position
    }
}

/// Initializes the "physical" loft vertices and edges from a section's vertex
/// ranges.
fn build_loft_edges(
    sketch_vertex_ranges: SketchPair<SketchVertexRange>,
    sketches: SketchPair<&Sketch>,
    max_radial_error: f32,
) -> Vec<SketchPair<LoftVertex>> {
    let mut loft_edges = Vec::new();

    // Iterate vertices of each sketch edge in parallel.
    let mut sketch_vertex_iters = sketch_vertex_ranges
        .as_ref()
        .zip(sketches)
        .map(|(range, sketch)| range.iter(sketch).peekable());

    let mut current_vertex_ids = sketch_vertex_iters
        .as_mut()
        .map(|iter| iter.next().unwrap());

    // Iterate until the current vertices are the last ones in the section.
    while sketch_vertex_iters
        .as_mut()
        .map(|iter| iter.peek())
        .iter()
        .any(|next| next.is_some())
    {
        let current_vertex_positions = current_vertex_ids
            .zip(sketches)
            .map(|(id, sketch)| sketch.vertex_map[&id]);

        // If the current vertices can form a valid edge (i.e it is within
        // the allowed radial error), create the edge.
        if radial_error(
            &current_vertex_positions.lower,
            &current_vertex_positions.upper,
        ) <= max_radial_error
        {
            loft_edges.push(current_vertex_ids.map(|id| LoftVertex::SketchVertex(id)));
        } else {
            // Form an intermediate edge for the CCW-most current vertex.

            // Take the CCW-most current vertex (the vertex to form
            // an edge from) by comparing the angle between the two
            // current vertices. The "pair index" is the index into the
            // `SketchPair`, as a programmatic way of accessing the lower or
            // upper sketch.
            let pair_vertex_index = if current_vertex_positions
                .lower
                .xy()
                .angle_to(current_vertex_positions.upper.xy())
                < 0.
            {
                0
            } else {
                1
            };

            // The pair index of the edge to split (just the opposite of
            // `pair_vertex_index`).
            let pair_edge_index = (pair_vertex_index + 1) % 2;

            let vertex_id = current_vertex_ids[pair_vertex_index];
            let edge_vertex_ids = (
                current_vertex_ids[pair_edge_index],
                *sketch_vertex_iters[pair_edge_index].peek().unwrap(),
            );

            let vertex_position = sketches[pair_vertex_index].vertex_map[&vertex_id];
            let edge_vertex_positions = {
                let sketch = &sketches[pair_edge_index];
                (
                    &sketch.vertex_map[&edge_vertex_ids.0],
                    &sketch.vertex_map[&edge_vertex_ids.1],
                )
            };

            let edge_length = edge_length(&vertex_position, edge_vertex_positions);

            let loft_vertex_vertex = LoftVertex::SketchVertex(vertex_id);
            let loft_vertex_edge = LoftVertex::SketchEdge {
                edge: edge_vertex_ids,
                edge_length,
            };

            let loft_edge = if pair_vertex_index == 0 {
                SketchPair::new(loft_vertex_vertex, loft_vertex_edge)
            } else {
                SketchPair::new(loft_vertex_edge, loft_vertex_vertex)
            };

            loft_edges.push(loft_edge);
        }

        // Increment the vertex iterator for one of the sketches.

        let next_vertex_positions = sketch_vertex_iters
            .as_mut()
            .zip(sketches)
            .map(|(iter, sketch)| iter.peek().map(|id| sketch.vertex_map[id]));

        if next_vertex_positions.lower.is_some() && next_vertex_positions.upper.is_none() {
            current_vertex_ids.lower = sketch_vertex_iters.lower.next().unwrap();
        } else if next_vertex_positions.lower.is_none() && next_vertex_positions.upper.is_some() {
            current_vertex_ids.upper = sketch_vertex_iters.upper.next().unwrap();
        } else {
            // There are still vertices to iterate on both sketches. In this
            // case, check the positions of both of the next vertices, and
            // only increment the CW-most of the next two vertices.

            let next_vertex_positions = next_vertex_positions.map(Option::unwrap);

            let angle = next_vertex_positions
                .lower
                .xy()
                .angle_to(next_vertex_positions.upper.xy());

            if angle.abs() <= max_radial_error {
                // If the next two vertices can form a valid edge, we've
                // reached the end of the section.
                break;
            }

            if angle > 0. {
                current_vertex_ids.lower = sketch_vertex_iters.lower.next().unwrap();
            } else {
                current_vertex_ids.upper = sketch_vertex_iters.upper.next().unwrap();
            }
        }
    }

    loft_edges
}

fn edge_length(vertex_position: &Vec3, edge_vertex_positions: (&Vec3, &Vec3)) -> f32 {
    // Variable names reference graphic here:
    // <https://www.mathsisfun.com/algebra/trig-sine-law.html>.

    // First solve for the edge vertices, to get B.
    let angle_b = {
        let angle_a = edge_vertex_positions
            .0
            .xy()
            .angle_to(edge_vertex_positions.1.xy());
        let edge_a = edge_vertex_positions
            .0
            .xy()
            .distance(edge_vertex_positions.1.xy());
        let edge_b = edge_vertex_positions.1.length();

        (edge_b * angle_a.sin() / edge_a).asin()
    };

    // Now solve for a.
    let angle_a = edge_vertex_positions.0.xy().angle_to(vertex_position.xy());
    let edge_c = edge_vertex_positions.0.xy().length();
    let angle_c = PI - angle_a - angle_b;

    edge_c * angle_a.sin() / angle_c.sin()
}
