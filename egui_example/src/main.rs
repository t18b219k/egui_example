use instant::Instant;
use std::iter;

use egui::{FontData, FontDefinitions};

use egui_glow::glow;
use egui_winit_platform::{Platform, PlatformDescriptor};
use epi::*;
use std::borrow::Cow;
use winit::dpi::LogicalSize;
use winit::event::Event::*;
use winit::event_loop::ControlFlow;

const INITIAL_WIDTH: u32 = 1280;
const INITIAL_HEIGHT: u32 = 720;


struct RepaintSignalMock;

impl epi::backend::RepaintSignal for RepaintSignalMock {
    fn request_repaint(&self) {}
}

static NOTO_SANS_JP_REGULAR: &[u8] = include_bytes!("../NotoSansJP-Regular.otf");
/// A simple egui + wgpu + winit based example.
async fn run(event_loop: winit::event_loop::EventLoop<()>, window: winit::window::Window) {
    let size = window.inner_size();

    let repaint_signal = std::sync::Arc::new(RepaintSignalMock);

    // We use the egui_winit_platform crate as the platform.
    let mut platform = Platform::new(PlatformDescriptor {
        physical_width: size.width as u32,
        physical_height: size.height as u32,
        scale_factor: window.scale_factor(),
        font_definitions: FontDefinitions::default(),
        style: Default::default(),
    });
    let mut egui_ctx = platform.context();
    //to install japanese font start frame.
    egui_ctx.begin_frame(egui::RawInput::default());
    let mut fonts = egui_ctx.fonts().definitions().clone();
    //install noto sans jp regular
    fonts.font_data.insert(
        "NotoSansCJK".to_string(),
        FontData::from_static(NOTO_SANS_JP_REGULAR),
    );
    fonts
        .fonts_for_family
        .values_mut()
        .for_each(|x| x.push("NotoSansCJK".to_string()));
    egui_ctx.set_fonts(fonts);
    egui_ctx.end_frame();
    //
    use wasm_bindgen::JsCast;
    use winit::platform::web::WindowExtWebSys;
    let canvas = window.canvas();
    let gl2_ctx = canvas
        .get_context("webgl2")
        .expect("Failed to query about WebGL2 context");
    let gl2_ctx = gl2_ctx.unwrap();

    let gl2_ctx = gl2_ctx
        .dyn_into::<web_sys::WebGl2RenderingContext>()
        .unwrap();
    let glow_ctx = egui_glow::glow::Context::from_webgl2_context(gl2_ctx);
    // We use the egui_wgpu_backend crate as the render backend.
    let mut painter = egui_glow::Painter::new(
        &glow_ctx,
        Some([INITIAL_WIDTH as i32, INITIAL_HEIGHT as i32]),
        "",
    )
    .unwrap();

    // Display the demo application that ships with egui.
    let mut demo_app = egui_demo_lib::WrapApp::default();

    let start_time = Instant::now();
    let mut previous_frame_time = None;
    event_loop.run(move |event, _, control_flow| {
        // Pass the winit events to the platform integration.
        platform.handle_event(&event);

        match event {
            RedrawRequested(..) => {
                platform.update_time(start_time.elapsed().as_secs_f64());
                // Begin to draw the UI frame.
                let egui_start = Instant::now();
                platform.begin_frame();
                let app_output = epi::backend::AppOutput::default();

                let mut frame = epi::Frame::new(epi::backend::FrameData {
                    info: epi::IntegrationInfo {
                        name: "egui_example",
                        web_info: Some(WebInfo {
                            web_location_hash: "".to_string(),
                        }),
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
                let (output, paint_commands) = platform.end_frame(Some(&window));
                let egui::Output {
                    text_cursor_pos, ..
                } = output;
                if let Some(egui::Pos2 { x, y }) = text_cursor_pos {
                    window.set_ime_position(winit::dpi::LogicalPosition { x, y });
                }
                let paint_jobs = platform.context().tessellate(paint_commands);
                {
                    unsafe {
                        use glow::HasContext as _;
                        glow_ctx.clear_color(0.0, 0.0, 0.0, 1.0);
                        glow_ctx.clear(glow::COLOR_BUFFER_BIT);
                    }
                    painter.upload_egui_texture(&glow_ctx, &platform.context().font_image());
                    // draw things behind egui here
                    painter.paint_meshes(
                        &glow_ctx,
                        [window.inner_size().width, window.inner_size().height],
                        window.scale_factor() as f32,
                        paint_jobs,
                    );
                }
                let frame_time = (Instant::now() - egui_start).as_secs_f64() as f32;
                previous_frame_time = Some(frame_time);
            }
            MainEventsCleared | UserEvent(_) => {
                window.request_redraw();
            }
            WindowEvent { event, .. } => match event {
                winit::event::WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                winit::event::WindowEvent::Resized(x) => {
                    window.set_inner_size(x);
                }
                _ => {}
            },
            _ => (),
        }
    });
}

fn main() {
    let event_loop = winit::event_loop::EventLoop::new();
    let wb = winit::window::WindowBuilder::new();

    let window = wb
        .with_inner_size(LogicalSize {
            width: INITIAL_WIDTH,
            height: INITIAL_HEIGHT,
        })
        .build(&event_loop)
        .unwrap();
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
        // Temporarily avoid srgb formats for the swapchain on the web
        run(event_loop, window).await;
    }
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init().expect("could not initialize logger");
        use winit::platform::web::WindowExtWebSys;
        // On wasm, append the canvas to the document body
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            })
            .expect("couldn't append canvas to document body");
        wasm_bindgen_futures::spawn_local(run(event_loop, window));
    }
}
