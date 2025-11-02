use crate::{
    sketch::{Sketch, VertexId},
    util::SketchPair,
};

/// A loft describes how two sketches are connected.
#[derive(Debug)]
pub struct Loft {
    sections: Vec<LoftSection>,
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

        let section_index = if self.loft.sections.len() == 1 {
            // `edge_candidate_section_index` does not handle the case where a
            // single section encompasses the entirety of both sketches.
            Some(0)
        } else {
            // Make sure the edge candidate vertices lie within the same
            // section. If they don't, this is not a valid split.
            self.edge_candidate_section_index(edge_candidate_vertices)
        };

        let Some(section_index) = section_index else {
            // Invalid split.
            return;
        };

        // Split the section by removing it and inserting two new sections.
        let section = self.loft.sections.remove(section_index);

        let new_section_a = LoftSection::uninitialized_with_partial_ranges((
            (
                section.sketch_vertex_ranges.lower.0,
                edge_candidate_vertices.lower,
            ),
            (
                section.sketch_vertex_ranges.upper.0,
                edge_candidate_vertices.upper,
            ),
        ));

        let new_section_b = LoftSection::uninitialized_with_partial_ranges((
            (
                edge_candidate_vertices.lower,
                section.sketch_vertex_ranges.lower.1,
            ),
            (
                edge_candidate_vertices.upper,
                section.sketch_vertex_ranges.upper.1,
            ),
        ));

        self.loft.sections.push(new_section_a);
        self.loft.sections.push(new_section_b);
    }

    /// Returns the index in `Loft::sections` of the section containing the edge
    /// candidate vertices. If the vertices lie in two different sections, the
    /// edge candidate is invalid and `None` is returned.
    fn edge_candidate_section_index(
        &self,
        edge_candidate_vertices: SketchPair<VertexId>,
    ) -> Option<usize> {
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

    pub fn build(self) -> Loft {
        let mut loft = self.loft;

        // todo: handle case where there are no sections.

        for section in &mut loft.sections {
            section.build_loft(self.sketches);
        }

        loft
    }
}

/// A "section" of a loft connects a range of vertices from one sketch to a
/// range of vertices in another sketch.
#[derive(Debug)]
struct LoftSection {
    /// Range of vertices in the original sketches in this section (range
    /// inclusive).
    sketch_vertex_ranges: SketchPair<(VertexId, VertexId)>,
    sketch_vertex_range_type: SketchVertexRangeType,
    /// Explicit vertices used for edge connections in this section. There may
    /// be additional, generated loft vertices which are not present in the set
    /// of vertices in the original sketches.
    loft_vertices: SketchPair<Vec<LoftVertex>>,
    /// Explicit edge connections between loft vertices in this section.
    loft_edges: Vec<SketchPair<usize>>,
}

/// A range of vertices from one sketch to a range of vertices in another
/// sketch, forming a "section". This vertex range may encompass the entirety of
/// both sketches (in which case it is the only section in the loft), the range
/// may form an edge or a series of edges, or the range of vertices may contain
/// only a single vertex.
#[derive(Debug)]
enum SketchVertexRangeType {
    Entire,
    Partial,
}

impl LoftSection {
    /// Create a loft section encompassing the entirety of the sketches.
    fn uninitialized_with_entire_ranges<R>(vertex_ranges: R) -> Self
    where
        R: Into<SketchPair<(VertexId, VertexId)>>,
    {
        Self {
            sketch_vertex_ranges: vertex_ranges.into(),
            sketch_vertex_range_type: SketchVertexRangeType::Entire,
            loft_vertices: SketchPair::splat(Vec::new()),
            loft_edges: Vec::new(),
        }
    }

    /// Create a loft section encompassing a part of the sketches (akin to an
    /// n-gon).
    fn uninitialized_with_partial_ranges<R>(vertex_ranges: R) -> Self
    where
        R: Into<SketchPair<(VertexId, VertexId)>>,
    {
        Self {
            sketch_vertex_ranges: vertex_ranges.into(),
            sketch_vertex_range_type: SketchVertexRangeType::Partial,
            loft_vertices: SketchPair::splat(Vec::new()),
            loft_edges: Vec::new(),
        }
    }

    /// Initializes the "physical" loft vertices and edges from the section's
    /// vertex ranges.
    fn build_loft(&mut self, sketches: SketchPair<&Sketch>) {
        // Iterate vertices of each sketch edge in parallel.

        // Take the next vertex from either sketch with the least radial angle.
        // Add it to the loft vertex list.
        //
        // - It will either be the last vertex on the section, or unconnected (never the last vertex).
        // - If connected, create a tri or a quad from the current vertices.
        // - If unconnected, create the intermediate vertex and then create a quad.

        // Once we've reached the last vertex in each sketch edge, we're done.

        let mut current_vertices = SketchPair::new(
            self.sketch_vertex_ranges.lower.0,
            self.sketch_vertex_ranges.upper.0,
        );

        loop {

            // We need a better way to represent vertex ranges which doesn't
            // suffer from inclusive/exclusive uncertainty.
        }
    }
}

/// A vertex used to form an edge to another sketch. A loft vertex may lie along
/// the edge of a sketch, i.e. it might not be present in the original set of
/// sketch vertices.
#[derive(Clone, Copy, Debug)]
struct LoftVertex {
    /// Adjacent vertices forming an edge in the original sketch.
    edge: (VertexId, VertexId),
    /// A value in range [0, 1] which determines where the vertex lies along the
    /// sketch's edge.
    slide: f32,
}

/// Returns whether this vertex range contains the passed-in vertex.
///
/// # Note
///
/// This does **not** handle the case when the vertex range covers the entirety
/// of the sketch! This must be handled by the caller.
fn vertex_range_contains_vertex(
    vertex: VertexId,
    vertex_range: &(VertexId, VertexId),
    sketch: &Sketch,
) -> bool {
    let i_start = sketch
        .vertex_order
        .iter()
        .position(|&vid| vid == vertex_range.0)
        .expect("Sketch does not contain VertexId.");

    for &vertex_id in sketch.vertex_order.iter().cycle().skip(i_start) {
        if vertex_id == vertex {
            return true;
        }

        if vertex_id == vertex_range.1 {
            break;
        }
    }

    false
}
