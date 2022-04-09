mod keyboard_debugger;

use instant::Instant;

use egui::{FontData, FontDefinitions, FontFamily};

use egui_glow::glow;
use egui_winit_platform::{Platform, PlatformDescriptor};
use epi::*;
use web_sys::HtmlCanvasElement;
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
    let egui_ctx = platform.context();
    //to install japanese font start frame.
    let mut fonts = egui::FontDefinitions::default();
    //install noto sans jp regular
    fonts.font_data.insert(
        "NotoSansCJK".to_string(),
        FontData::from_static(NOTO_SANS_JP_REGULAR),
    );
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0,"NotoSansCJK".to_owned());
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0,"NotoSansCJK".to_owned());
    egui_ctx.set_fonts(fonts);
    //
    use wasm_bindgen::JsCast;
    use winit::platform::web::WindowExtWebSys;
    let canvas = window.canvas();
    let gl_ctx = canvas
        .get_context("webgl")
        .expect("Failed to query about WebGL1 context");
    let gl_ctx = gl_ctx.unwrap();

    let gl_ctx = gl_ctx.dyn_into::<web_sys::WebGlRenderingContext>().unwrap();
    let glow_ctx = egui_glow::glow::Context::from_webgl1_context(gl_ctx);
    // We use the egui_glow crate as the render backend.
    let mut painter = egui_glow::Painter::new(&glow_ctx, None, "").unwrap();

    // Display the demo application
    let mut demo_app = keyboard_debugger::KeyboardDebugger::new();

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
                        web_info:None,
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
                let egui::FullOutput{
                    platform_output, needs_repaint:_, textures_delta, shapes
                } = platform.end_frame(Some(&window));
                let text_cursor_pos=platform_output.text_cursor_pos;
                if let Some(egui::Pos2 { x, y }) = text_cursor_pos {
                    window.set_ime_position(winit::dpi::LogicalPosition { x, y });
                }
                let paint_jobs = platform.context().tessellate(shapes);
                {
                    unsafe {
                        use glow::HasContext as _;
                        glow_ctx.clear_color(0.0, 0.0, 0.0, 1.0);
                        glow_ctx.clear(glow::COLOR_BUFFER_BIT);
                    }
                    textures_delta.set
                        .iter()
                        .for_each(|(k, v)| {
                            painter.set_texture(&glow_ctx, *k, v);
                        });
                    textures_delta.free
                        .iter()
                        .for_each(|k| {
                            painter.free_texture(&glow_ctx,*k);
                        });
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
    let event_loop = winit::event_loop::EventLoop::new();
    let wb = winit::window::WindowBuilder::new();

    use wasm_bindgen::JsCast;
    use winit::platform::web::WindowBuilderExtWebSys;
    // On wasm, append the canvas to the document body
    let canvas = web_sys::window()
        .and_then(|win| win.document())
        .and_then(|doc| doc.get_element_by_id("the_canvas_id"));
    let window = wb
        .with_inner_size(LogicalSize {
            width: INITIAL_WIDTH,
            height: INITIAL_HEIGHT,
        })
        .with_canvas(canvas.and_then(|element| element.dyn_into::<HtmlCanvasElement>().ok()))
        .build(&event_loop)
        .unwrap();
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        wasm_bindgen_futures::spawn_local(run(event_loop, window));
    }
}
#[cfg(target_arch = "wasm32")]
mod wasm {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(start)]
    pub fn run() {
        super::main();
    }
}
