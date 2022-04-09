mod keyboard_debugger;

use std::iter;
use std::time::Instant;

use egui::{FontData, FullOutput};
use egui_wgpu_backend::wgpu::Color;
use egui_wgpu_backend::{wgpu, RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use epi::*;
use winit::event::Event::*;
use winit::window::Window;
use winit::event_loop::{ControlFlow, EventLoop};



struct RepaintSignalMock;

impl epi::backend::RepaintSignal for RepaintSignalMock {
    fn request_repaint(&self) {}
}

/// A simple egui + wgpu + winit based example.
async fn run(event_loop: EventLoop<()>, window: Window) {

    let instance = wgpu::Instance::new(wgpu::Backends::all());
    let surface = unsafe { instance.create_surface(&window) };

    // WGPU 0.11+ support force fallback (if HW implementation not supported), set it to true or false (optional).
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    }))
    .unwrap();

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            features: wgpu::Features::default(),
            limits: wgpu::Limits::default(),
            label: None,
        },
        None,
    ))
    .unwrap();

    let size = window.inner_size();
    let surface_format = surface.get_preferred_format(&adapter).unwrap();
    let mut surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width as u32,
        height: size.height as u32,
        present_mode: wgpu::PresentMode::Fifo,
    };
    surface.configure(&device, &surface_config);

    let repaint_signal = std::sync::Arc::new(RepaintSignalMock);
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "Noto".to_owned(),
        FontData::from_static(include_bytes!("../NotoSansJP-Regular.otf")),
    );
    fonts.families.iter_mut().for_each(|(_, fonts)| {
        fonts.insert(0, "Noto".to_owned());
    });

    // We use the egui_winit_platform crate as the platform.
    let mut platform = Platform::new(PlatformDescriptor {
        physical_width: size.width as u32,
        physical_height: size.height as u32,
        scale_factor: window.scale_factor(),
        font_definitions: fonts,
        style: Default::default(),
    });

    // We use the egui_wgpu_backend crate as the render backend.
    let mut egui_rpass = RenderPass::new(&device, surface_format, 1);

    // Display the demo application that ships with egui.
    window.set_ime_allowed(true);
    let mut demo_app = crate::keyboard_debugger::KeyboardDebugger::new();

    let start_time = Instant::now();
    let mut previous_frame_time = None;
    event_loop.run(move |event, _, control_flow| {
        // Pass the winit events to the platform integration.
        platform.handle_event(&event);

        match event {
            RedrawRequested(..) => {
                platform.update_time(start_time.elapsed().as_secs_f64());

                let output_frame = match surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(wgpu::SurfaceError::Outdated) => {
                        // This error occurs when the app is minimized on Windows.
                        // Silently return here to prevent spamming the console with:
                        // "The underlying surface has changed, and therefore the swap chain must be updated"
                        return;
                    }
                    Err(e) => {
                        eprintln!("Dropped frame with error: {}", e);
                        return;
                    }
                };
                let output_view = output_frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                // Begin to draw the UI frame.
                let egui_start = Instant::now();
                platform.begin_frame();
                let app_output = epi::backend::AppOutput::default();

                let mut frame = epi::Frame::new(epi::backend::FrameData {
                    info: epi::IntegrationInfo {
                        name: "egui_example",
                        web_info: None,
                        cpu_usage: previous_frame_time,
                        native_pixels_per_point: Some(window.scale_factor() as _),
                        prefer_dark_mode: None,
                    },
                    output: app_output,
                    repaint_signal: repaint_signal.clone(),
                });

                // Draw the demo application.
                demo_app.update(&platform.context(), &mut frame);

                // End the UI frame. We could now handle the output and draw the UI with the backend.
                let FullOutput {
                    platform_output:_,
                    needs_repaint:_,
                    textures_delta,
                    shapes,
                } = platform.end_frame(Some(&window));
                let paint_jobs = platform.context().tessellate(shapes);

                let frame_time = (Instant::now() - egui_start).as_secs_f64() as f32;
                previous_frame_time = Some(frame_time);

                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("encoder"),
                });

                // Upload all resources for the GPU.
                let screen_descriptor = ScreenDescriptor {
                    physical_width: surface_config.width,
                    physical_height: surface_config.height,
                    scale_factor: window.scale_factor() as f32,
                };
                egui_rpass
                    .add_textures(&device, &queue, &textures_delta)
                    .unwrap();
                egui_rpass.update_buffers(&device, &queue, &paint_jobs, &screen_descriptor);
                egui_rpass
                    .execute(
                        &mut encoder,
                        &output_view,
                        &paint_jobs,
                        &screen_descriptor,
                        Some(Color::BLACK),
                    )
                    .unwrap();
                egui_rpass.remove_textures(textures_delta).unwrap();

                // Record all render passes.
                egui_rpass
                    .execute(
                        &mut encoder,
                        &output_view,
                        &paint_jobs,
                        &screen_descriptor,
                        Some(wgpu::Color::BLACK),
                    )
                    .unwrap();
                // Submit the commands.
                queue.submit(iter::once(encoder.finish()));

                // Redraw egui
                output_frame.present();

                // Suppport reactive on windows only, but not on linux.
                // if _output.needs_repaint {
                //     *control_flow = ControlFlow::Poll;
                // } else {
                //     *control_flow = ControlFlow::Wait;
                // }
            }
            MainEventsCleared => {
                window.request_redraw();
            }
            WindowEvent { event, .. } => match event {
                winit::event::WindowEvent::Resized(size) => {
                    // Resize with 0 width and height is used by winit to signal a minimize event on Windows.
                    // See: https://github.com/rust-windowing/winit/issues/208
                    // This solves an issue where the app would panic when minimizing on Windows.
                    if size.width > 0 && size.height > 0 {
                        surface_config.width = size.width;
                        surface_config.height = size.height;
                        surface.configure(&device, &surface_config);
                    }
                }
                winit::event::WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                event => match event {
                    winit::event::WindowEvent::ReceivedCharacter(_)
                    | winit::event::WindowEvent::KeyboardInput { .. }
                    | winit::event::WindowEvent::IME(_) => {
                        demo_app.feed(&event);
                    }
                    _ => (),
                },
            },
            _ => (),
        }
    });
}
fn main() {
    let event_loop = EventLoop::new();

    #[cfg(not(target_arch = "wasm32"))]
        {
            let window = winit::window::Window::new(&event_loop).unwrap();
            env_logger::init();
            // Temporarily avoid srgb formats for the swapchain on the web
            pollster::block_on(run(event_loop, window));
        }
    #[cfg(target_arch = "wasm32")]
        {
            use web_sys::HtmlCanvasElement;
            use wasm_bindgen::JsCast;
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init().expect("could not initialize logger");
            use winit::platform::web::WindowBuilderExtWebSys;
            // On wasm, append the canvas to the document body
            let canvas = web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| doc.get_element_by_id("the_canvas_id"));
            let window =winit::window::WindowBuilder::new()
                .with_canvas(canvas.and_then(|element| element.dyn_into::<HtmlCanvasElement>().ok()))
                .build(&event_loop)
                .unwrap();
            wasm_bindgen_futures::spawn_local(run(event_loop, window));
        }
}