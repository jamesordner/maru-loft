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
        max_radial_edge_angle: 1.,
    });
}
