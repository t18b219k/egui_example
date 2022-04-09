use egui::Context;
use epi::Frame;

pub struct KeyboardDebugger {
    text_buffer: String,
    event_buffer: Vec<String>,
}

impl KeyboardDebugger {
    pub fn new() -> KeyboardDebugger {
        Self {
            text_buffer: String::new(),
            event_buffer: vec![],
        }
    }
    pub fn feed(&mut self, event: &winit::event::WindowEvent) {
        self.event_buffer.push(format!("{:?}", event));
    }
    pub fn clear(&mut self) {
        self.event_buffer.clear()
    }
}
impl epi::App for KeyboardDebugger {
    fn update(&mut self, ctx: &Context, frame: &Frame) {
        egui::Window::new("Keyboard debugger").show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("please input here");
                    ui.text_edit_singleline(&mut self.text_buffer);
                    if ui.button("clear logs").clicked() {
                        self.clear()
                    }
                });
                let scroll = egui::containers::ScrollArea::both();
                scroll.stick_to_bottom().show(ui, |ui| {
                    ui.vertical(|ui| {
                        self.event_buffer.iter().for_each(|log| {
                            ui.label(log);
                        });
                    })
                });
            });
        });
        frame.request_repaint()
    }

    fn name(&self) -> &str {
        "Keyboard debugger"
    }
}
