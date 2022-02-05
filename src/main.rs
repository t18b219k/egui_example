mod keyboard_debugger;

use instant::Instant;

use egui::{FontData, FontDefinitions};

use egui_glow::glow;
use egui_winit_platform::{Platform, PlatformDescriptor};
use epi::*;
use winit::dpi::LogicalSize;
use winit::event::Event::*;
use winit::event_loop::ControlFlow;

const INITIAL_WIDTH: u32 = 1280;
const INITIAL_HEIGHT: u32 = 720;


static NOTO_SANS_JP_REGULAR: &[u8] = include_bytes!("../NotoSansJP-Regular.otf");

async fn run(event_loop: winit::event_loop::EventLoop<()>, window: winit::window::Window) {
    let size = window.inner_size();


    // We use the egui_winit_platform crate as the platform.
    let mut platform = Platform::new(PlatformDescriptor {
        physical_width: size.width as u32,
        physical_height: size.height as u32,
        scale_factor: window.scale_factor(),
        font_definitions: FontDefinitions::default(),
        style: Default::default(),
    });
    let  egui_ctx = platform.context();
    //install japanese
    let mut fonts = FontDefinitions::default();

    //install noto sans jp regular
    fonts.font_data.insert(
        "NotoSansCJK".to_string(),
        FontData::from_static(NOTO_SANS_JP_REGULAR),
    );
    fonts
        .families
        .values_mut()
        .for_each(|x| x.push("NotoSansCJK".to_string()));
    egui_ctx.set_fonts(fonts);
    //
    use wasm_bindgen::JsCast;
    use winit::platform::web::WindowExtWebSys;
    let canvas = window.canvas();
    let gl_ctx = canvas
        .get_context("webgl")
        .expect("Failed to query about WebGL1 context");
    let gl_ctx = gl_ctx.unwrap();

    let gl_ctx = gl_ctx
        .dyn_into::<web_sys::WebGlRenderingContext>()
        .unwrap();
    let glow_ctx = egui_glow::glow::Context::from_webgl1_context(gl_ctx);
    // We use the egui_glow crate as the render backend.
    let mut painter = egui_glow::Painter::new(
        &glow_ctx,
        None,
        "",
    )
    .unwrap();

    // Display the demo application that ships with egui.

    let mut debugger = keyboard_debugger::KeyboardDebugger::new();

    let start_time = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        // Pass the winit events to the platform integration.
        platform.handle_event(&event);

        match event {
            RedrawRequested(..) => {
                platform.update_time(start_time.elapsed().as_secs_f64());

                platform.begin_frame();


                // Draw the demo application.
                egui::containers::CentralPanel::default().show(&platform.context(),|ui|{
                   debugger.ui(ui);
                });
                // End the UI frame. We could now handle the output and draw the UI with the backend.
                let (output, paint_commands) = platform.end_frame(Some(&window));
                let paint_jobs = platform.context().tessellate(paint_commands);
                let mut tex_delta =platform.context().tex_manager().write().take_delta();
                tex_delta.set.iter().for_each(|(k,v)|{
                    painter.set_texture(&glow_ctx,*k,v)
                });


                let egui::Output {
                    text_cursor_pos, ..
                } = output;
                if let Some(egui::Pos2 { x, y }) = text_cursor_pos {
                    window.set_ime_position(winit::dpi::LogicalPosition { x, y });
                }

                {
                    unsafe {
                        use glow::HasContext as _;
                        glow_ctx.clear_color(0.0, 0.0, 0.0, 1.0);
                        glow_ctx.clear(glow::COLOR_BUFFER_BIT);
                    }
                    // draw things behind egui here
                    painter.paint_meshes(
                        &glow_ctx,
                        [window.inner_size().width, window.inner_size().height],
                        window.scale_factor() as f32,
                        paint_jobs,
                    );
                }
                tex_delta.free.drain(..).for_each(|k|{
                    painter.free_texture(&glow_ctx,k)
                });

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
                event => {
                    debugger.feed(&event);
                }
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
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));

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
#[cfg(target_arch = "wasm32")]
mod wasm {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(start)]
    pub fn run() {
        super::main();
    }
}