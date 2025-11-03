use std::time::Instant;

use glam::{Vec2, Vec3Swizzles};
use imgui::{Condition, FontSource, MouseCursor};
use imgui_wgpu::RendererConfig;
use imgui_winit_support::WinitPlatform;
use lofter::Lofter;
use winit::event::Event;

use crate::render::Renderer;

pub struct ImguiState {
    context: imgui::Context,
    platform: WinitPlatform,
    renderer: imgui_wgpu::Renderer,
    last_frame: Instant,
    last_cursor: Option<MouseCursor>,
    pub loft_state: LoftState,
}

pub struct LoftState {
    pub reloft: bool,
    pub max_angle: f32,
    pub rotation: f32,
}

impl Default for LoftState {
    fn default() -> Self {
        Self {
            reloft: false,
            max_angle: 30.,
            rotation: 0.,
        }
    }
}

impl ImguiState {
    pub fn new(renderer: &Renderer, hidpi_factor: f32) -> Self {
        let mut context = imgui::Context::create();
        let mut platform = imgui_winit_support::WinitPlatform::new(&mut context);
        platform.attach_window(
            context.io_mut(),
            &renderer.window,
            imgui_winit_support::HiDpiMode::Default,
        );
        context.set_ini_filename(None);

        let font_size = 13.0 * hidpi_factor;
        context.io_mut().font_global_scale = 1.0 / hidpi_factor;

        context.fonts().add_font(&[FontSource::DefaultFontData {
            config: Some(imgui::FontConfig {
                oversample_h: 1,
                pixel_snap_h: true,
                size_pixels: font_size,
                ..Default::default()
            }),
        }]);

        let renderer_config = RendererConfig {
            texture_format: renderer.surface_config.format,
            ..Default::default()
        };

        let renderer = imgui_wgpu::Renderer::new(
            &mut context,
            &renderer.device,
            &renderer.queue,
            renderer_config,
        );

        let last_frame = Instant::now();
        let last_cursor = None;

        Self {
            context,
            platform,
            renderer,
            last_frame,
            last_cursor,
            loft_state: Default::default(),
        }
    }

    pub fn handle_event(&mut self, renderer: &Renderer, event: &Event<()>) {
        self.platform
            .handle_event::<()>(self.context.io_mut(), &renderer.window, event);
    }

    pub fn draw(&mut self, renderer: &Renderer, view: &wgpu::TextureView, lofter: &mut Lofter) {
        let now = Instant::now();
        self.context
            .io_mut()
            .update_delta_time(now - self.last_frame);
        self.last_frame = now;

        self.platform
            .prepare_frame(self.context.io_mut(), &renderer.window)
            .unwrap();

        let ui = self.context.frame();

        ui.window("Lofter")
            .size([200.0, 500.0], Condition::FirstUseEver)
            .build(|| {
                ui.separator();

                ui.child_window("Upper sketch")
                    .size([200.; _])
                    .movable(false)
                    .build(|| {
                        let window_pos = Vec2::from_array(ui.window_pos());
                        let window_size = Vec2::from_array(ui.window_size());
                        let window_center = window_pos + window_size / 2.;

                        let draw_list = ui.get_window_draw_list();
                        let points: Vec<_> = lofter
                            .vertices(1)
                            .unwrap()
                            .map(|(_, pos)| (window_center + pos.xy() * 50.).to_array())
                            .collect();

                        draw_list
                            .add_polyline(points.clone(), [1., 0., 0.])
                            .filled(true)
                            .build();

                        for point in &points {
                            draw_list
                                .add_rect(
                                    (Vec2::from_array(*point) - Vec2::splat(5.)).to_array(),
                                    (Vec2::from_array(*point) + Vec2::splat(5.)).to_array(),
                                    [1., 1., 1.],
                                )
                                .build();
                        }
                    });

                ui.separator();

                ui.child_window("Lower sketch")
                    .size([200.; _])
                    .movable(false)
                    .build(|| {
                        let window_pos = Vec2::from_array(ui.window_pos());
                        let window_size = Vec2::from_array(ui.window_size());
                        let window_center = window_pos + window_size / 2.;

                        let draw_list = ui.get_window_draw_list();
                        let points: Vec<_> = lofter
                            .vertices(0)
                            .unwrap()
                            .map(|(_, pos)| (window_center + pos.xy() * 50.).to_array())
                            .collect();

                        draw_list
                            .add_polyline(points.clone(), [1., 0., 0.])
                            .filled(true)
                            .build();

                        for point in &points {
                            draw_list
                                .add_rect(
                                    (Vec2::from_array(*point) - Vec2::splat(5.)).to_array(),
                                    (Vec2::from_array(*point) + Vec2::splat(5.)).to_array(),
                                    [1., 1., 1.],
                                )
                                .build();
                        }
                    });

                ui.separator();

                ui.slider("Max angle", 0.1, 60., &mut self.loft_state.max_angle);
                ui.slider("Rotation", -180., 180., &mut self.loft_state.rotation);
                if ui.button("Loft") {
                    self.loft_state.reloft = true;
                }
            });

        ui.window("Vertices").build(|| {
            let mut i = 0;

            lofter.vertices_mut(1, |(_, vert)| {
                let label = i.to_string();
                i += 1;
                ui.input_float3(&label, vert.as_mut()).build();
            });

            ui.separator();

            lofter.vertices_mut(0, |(_, vert)| {
                let label = i.to_string();
                i += 1;
                ui.input_float3(&label, vert.as_mut()).build();
            });
        });

        let mut encoder: wgpu::CommandEncoder = renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        if self.last_cursor != ui.mouse_cursor() {
            self.last_cursor = ui.mouse_cursor();
            self.platform.prepare_render(ui, &renderer.window);
        }

        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        self.renderer
            .render(
                self.context.render(),
                &renderer.queue,
                &renderer.device,
                &mut rpass,
            )
            .expect("Rendering failed");

        drop(rpass);

        renderer.queue.submit(Some(encoder.finish()));
    }
}
