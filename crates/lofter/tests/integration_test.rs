use glam::Vec3;
use lofter::{LoftOptions, Lofter, SketchDescriptor};

#[test]
fn integration() {
    let mut lofter = Lofter::default();

    lofter.push_sketch(&SketchDescriptor {
        vertices: vec![
            Vec3::new(1., 0., 0.),
            Vec3::new(0., 1., 0.),
            Vec3::new(-1., -1., 0.),
        ],
        relative_position: Vec3::ZERO,
        rotation: Vec3::ZERO,
    });

    lofter.push_sketch(&SketchDescriptor {
        vertices: vec![
            Vec3::new(1., 0., 0.),
            Vec3::new(0., 1., 0.),
            Vec3::new(-1., 0., 0.),
            Vec3::new(0., -1., 0.),
        ],
        relative_position: Vec3::new(0., 0., 1.),
        rotation: Vec3::ZERO,
    });

    lofter.loft(&LoftOptions {
        max_radial_edge_angle: 5.,
    });

    let vb = lofter.vertex_buffer();

    let mut obj_string = String::new();
    let mut i = 1;

    for tri in &vb {
        for vert in tri {
            obj_string.push_str("v ");
            for axis in vert[0].to_array() {
                obj_string.push_str(&axis.to_string());
                obj_string.push(' ');
            }
            obj_string.push('\n');
        }

        obj_string.push_str("f ");
        for _ in 0..3 {
            obj_string.push_str(&i.to_string());
            i += 1;
            obj_string.push(' ');
        }
        obj_string.push('\n');
    }

    dbg!(obj_string);
}
