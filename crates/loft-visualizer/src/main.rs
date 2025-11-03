use glam::Vec3;
use lofter::{LoftOptions, Lofter};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::Window,
};

use crate::{render::Renderer, ui::ImguiState};

mod render;
mod ui;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut App::default()).unwrap();
}

struct AppWindow {
    renderer: Renderer,
    window: Arc<Window>,
    hidpi_factor: f32,
    camera_rotation: f32,
    imgui: Option<ImguiState>,
}

#[derive(Default)]
struct App {
    lofter: Lofter,
    app_window: Option<AppWindow>,
}

impl AppWindow {
    fn setup_gpu(event_loop: &ActiveEventLoop) -> Self {
        let window = {
            let size = LogicalSize::new(1280.0, 720.0);

            let attributes = Window::default_attributes()
                .with_inner_size(size)
                .with_title("Lofter");
            Arc::new(event_loop.create_window(attributes).unwrap())
        };

        let hidpi_factor = window.scale_factor() as f32;
        let renderer = Renderer::new(window.clone());

        Self {
            renderer,
            window,
            hidpi_factor,
            camera_rotation: 0.,
            imgui: None,
        }
    }

    fn new(lofter: &Lofter, event_loop: &ActiveEventLoop) -> Self {
        let mut app_window = Self::setup_gpu(event_loop);
        app_window.imgui = ImguiState::new(&app_window.renderer, app_window.hidpi_factor).into();

        let vb = lofter.vertex_buffer();

        app_window.renderer.set_loft_vertex_buffer(&vb);
        app_window.renderer.set_camera_rotation(0.);

        app_window
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.app_window = Some(AppWindow::new(&self.lofter, event_loop));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let app_window = self.app_window.as_mut().unwrap();
        let imgui = app_window.imgui.as_mut().unwrap();

        match &event {
            WindowEvent::Resized(size) => {
                app_window.renderer.resize(size.width, size.height);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let delta = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => *y,
                    winit::event::MouseScrollDelta::PixelDelta(physical_position) => {
                        physical_position.y as f32
                    }
                };

                app_window.camera_rotation += delta * 0.01;
                app_window
                    .renderer
                    .set_camera_rotation(app_window.camera_rotation);
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                let Some(surface) = app_window.renderer.frame_surface_texture() else {
                    return;
                };

                let view = surface
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                app_window.renderer.draw(&view);
                imgui.draw(&app_window.renderer, &view, &mut self.lofter);

                // Check UI changes.

                if imgui.loft_state.reloft {
                    imgui.loft_state.reloft = false;

                    self.lofter.loft(&LoftOptions {
                        max_radial_edge_angle: imgui.loft_state.max_angle,
                    });
                }

                self.lofter
                    .set_sketch_rotation(1, &Vec3::new(0., 0., imgui.loft_state.rotation));

                // Todo: don't do this every frame.
                let vertex_buffer = self.lofter.vertex_buffer();
                app_window.renderer.set_loft_vertex_buffer(&vertex_buffer);

                surface.present();
            }
            _ => (),
        }

        imgui.handle_event(
            &app_window.renderer,
            &Event::WindowEvent { window_id, event },
        );
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: ()) {
        let app_window = self.app_window.as_mut().unwrap();
        let imgui = app_window.imgui.as_mut().unwrap();
        imgui.handle_event(&app_window.renderer, &Event::UserEvent(event));
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        let app_window = self.app_window.as_mut().unwrap();
        let imgui = app_window.imgui.as_mut().unwrap();
        imgui.handle_event(
            &app_window.renderer,
            &Event::DeviceEvent { device_id, event },
        );
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let app_window = self.app_window.as_mut().unwrap();
        let imgui = app_window.imgui.as_mut().unwrap();
        app_window.window.request_redraw();
        imgui.handle_event(&app_window.renderer, &Event::AboutToWait);
    }
}
