use glam::{Vec3, Vec3Swizzles};

use crate::{
    sketch::{Sketch, VertexId},
    util::{SketchPair, radial_error},
};

/// A loft describes how two sketches are connected.
#[derive(Debug)]
pub struct Loft {
    sections: Vec<LoftSection>,
}

impl Loft {
    /// Generates a renderable, non-indexed vertex buffer.
    pub fn append_vertex_buffer(
        &self,
        vertex_buffer: &mut Vec<[Vec3; 3]>,
        sketches: SketchPair<&Sketch>,
    ) {
        let mut prev_loft_edge = self.sections.last().unwrap().loft_edges.last().unwrap();

        for loft_edge in self
            .sections
            .iter()
            .map(|section| &section.loft_edges)
            .flatten()
            .chain(&[self.sections[0].loft_edges[0]])
        {
            if prev_loft_edge.lower == loft_edge.lower {
                // Tri.
                vertex_buffer.push([
                    prev_loft_edge.upper.to_pos(sketches.upper),
                    loft_edge.lower.to_pos(sketches.lower),
                    loft_edge.upper.to_pos(sketches.upper),
                ]);
            } else if prev_loft_edge.upper == loft_edge.upper {
                // Tri.
                vertex_buffer.push([
                    prev_loft_edge.upper.to_pos(sketches.upper),
                    prev_loft_edge.lower.to_pos(sketches.lower),
                    loft_edge.lower.to_pos(sketches.lower),
                ]);
            } else {
                // Quad.
                vertex_buffer.push([
                    prev_loft_edge.upper.to_pos(sketches.upper),
                    prev_loft_edge.lower.to_pos(sketches.lower),
                    loft_edge.lower.to_pos(sketches.lower),
                ]);
                vertex_buffer.push([
                    prev_loft_edge.upper.to_pos(sketches.upper),
                    loft_edge.lower.to_pos(sketches.lower),
                    loft_edge.upper.to_pos(sketches.upper),
                ]);
            }

            prev_loft_edge = loft_edge;
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
                (edge_candidate_vertices.lower, edge_candidate_vertices.lower),
                (edge_candidate_vertices.upper, edge_candidate_vertices.upper),
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

        let new_section_a = LoftSection::uninitialized_with_partial_ranges((
            (
                section.sketch_vertex_ranges.lower.range.0,
                edge_candidate_vertices.lower,
            ),
            (
                section.sketch_vertex_ranges.upper.range.0,
                edge_candidate_vertices.upper,
            ),
        ));

        let new_section_b = LoftSection::uninitialized_with_partial_ranges((
            (
                edge_candidate_vertices.lower,
                section.sketch_vertex_ranges.lower.range.1,
            ),
            (
                edge_candidate_vertices.upper,
                section.sketch_vertex_ranges.upper.range.1,
            ),
        ));

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

        // todo: handle case where there are no sections.

        for section in &mut loft.sections {
            println!("Building sketch section loft...");
            section.build_loft(self.sketches, max_radial_error);
        }

        loft
    }
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
        R: Into<SketchPair<(VertexId, VertexId)>>,
    {
        let sketch_vertex_ranges = vertex_ranges.into().map(SketchVertexRange::entire);

        Self {
            sketch_vertex_ranges,
            loft_edges: Vec::new(),
        }
    }

    /// Create a loft section encompassing a part of the sketches (akin to an
    /// n-gon).
    fn uninitialized_with_partial_ranges<R>(vertex_ranges: R) -> Self
    where
        R: Into<SketchPair<(VertexId, VertexId)>>,
    {
        let sketch_vertex_ranges = vertex_ranges.into().map(SketchVertexRange::partial);

        Self {
            sketch_vertex_ranges,
            loft_edges: Vec::new(),
        }
    }

    /// Initializes the "physical" loft vertices and edges from the section's
    /// vertex ranges.
    fn build_loft(&mut self, sketches: SketchPair<&Sketch>, max_radial_error: f32) {
        // Iterate vertices of each sketch edge in parallel.

        // Take the next vertex from either sketch with the least radial angle.
        // Add it to the loft vertex list.
        //
        // - It will either be the last vertex on the section, or unconnected (never the last vertex).
        // - If connected, create a tri or a quad from the current vertices.
        // - If unconnected, create the intermediate vertex and then create a quad.

        // Once we've reached the last vertex in each sketch edge, we're done.

        // Iterate the sketch edges in parallel.
        let mut sketch_vertex_iters = self
            .sketch_vertex_ranges
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
                println!("Forming valid edge at {:?}", current_vertex_positions);

                self.loft_edges
                    .push(current_vertex_ids.map(|id| LoftVertex::SketchVertex(id)));
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

                let loft_vertex_vertex = LoftVertex::SketchVertex(vertex_id);
                let loft_vertex_edge = LoftVertex::SketchEdge {
                    edge: edge_vertex_ids,
                    slide: 0.,
                };

                let loft_edge = if pair_vertex_index == 0 {
                    SketchPair::new(loft_vertex_vertex, loft_vertex_edge)
                } else {
                    SketchPair::new(loft_vertex_edge, loft_vertex_vertex)
                };

                self.loft_edges.push(loft_edge);
            }

            // Increment the vertex iterator for one of the sketches.

            let next_vertex_positions = sketch_vertex_iters
                .as_mut()
                .zip(sketches)
                .map(|(iter, sketch)| iter.peek().map(|id| sketch.vertex_map[id]));

            if next_vertex_positions.lower.is_some() && next_vertex_positions.upper.is_none() {
                current_vertex_ids.lower = sketch_vertex_iters.lower.next().unwrap();
            } else if next_vertex_positions.lower.is_none() && next_vertex_positions.upper.is_some()
            {
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
    }
}

/// A range of vertices from one sketch to a range of vertices in another
/// sketch, forming a "section". This vertex range may encompass the entirety of
/// both sketches (in which case it is the only section in the loft), the range
/// may form an edge or a series of edges, or the range of vertices may contain
/// only a single vertex.
#[derive(Debug)]
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
    fn entire(range: (VertexId, VertexId)) -> Self {
        Self {
            range,
            covers_entire_sketch: true,
        }
    }

    fn partial(range: (VertexId, VertexId)) -> Self {
        Self {
            range,
            covers_entire_sketch: false,
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
        slide: f32,
    },
}

impl LoftVertex {
    fn to_pos(&self, sketch: &Sketch) -> Vec3 {
        let relative_pos = match self {
            LoftVertex::SketchVertex(id) => sketch.vertex_map[id],
            LoftVertex::SketchEdge { edge, slide } => {
                let a = sketch.vertex_map[&edge.0];
                let b = sketch.vertex_map[&edge.1];

                a.lerp(b, *slide)
            }
        };

        relative_pos + sketch.relative_position
    }
}
