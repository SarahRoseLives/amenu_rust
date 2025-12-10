use eframe::egui;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::env;

// --- Logic for Loading Prompts ---
fn load_prompts<P: AsRef<Path> + std::fmt::Debug>(filename: P) -> HashMap<String, String> {
    let mut prompts = HashMap::new();

    if let Ok(cwd) = env::current_dir() {
        println!("DEBUG: Current working dir: {:?}", cwd);
    }
    println!("DEBUG: Attempting to open file: {:?}", filename);

    match File::open(&filename) {
        Ok(file) => {
            println!("DEBUG: File opened successfully.");
            let lines = io::BufReader::new(file).lines();
            for (i, line) in lines.flatten().enumerate() {
                if i < 3 {
                    println!("DEBUG: Reading line {}: '{}'", i, line);
                }
                if let Some((title, prompt)) = line.split_once(':') {
                    prompts.insert(title.trim().to_string(), prompt.trim().to_string());
                }
            }
        }
        Err(e) => {
            eprintln!("ERROR: Could not open file: {}", e);
        }
    }

    println!("DEBUG: Total loaded prompts: {}", prompts.len());
    prompts
}

struct Amenu {
    prompts: HashMap<String, String>,
    query: String,
    all_titles: Vec<String>,
    filtered_suggestions: Vec<String>,
    selected_index: usize,
    clipboard: Option<arboard::Clipboard>,
    // CHANGED: We use a counter instead of a boolean
    startup_counter: u8,
}

impl Amenu {
    fn new(cc: &eframe::CreationContext, filename: String) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(egui::Color32::WHITE);
        visuals.panel_fill = egui::Color32::from_rgb(35, 36, 41);
        cc.egui_ctx.set_visuals(visuals);

        let mut style = (*cc.egui_ctx.style()).clone();
        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::new(14.0, egui::FontFamily::Monospace),
        );
        cc.egui_ctx.set_style(style);

        let prompts = load_prompts(filename);
        let all_titles: Vec<String> = prompts.keys().cloned().collect();

        Self {
            prompts,
            all_titles,
            query: String::new(),
            filtered_suggestions: Vec::new(),
            selected_index: 0,
            clipboard: arboard::Clipboard::new().ok(),
            startup_counter: 0, // Start at 0
        }
    }

    fn copy_and_quit(&mut self, ctx: &egui::Context) {
        if !self.filtered_suggestions.is_empty() {
            if let Some(title) = self.filtered_suggestions.get(self.selected_index) {
                if let Some(content) = self.prompts.get(title) {
                    if let Some(cb) = &mut self.clipboard {
                        if let Err(e) = cb.set_text(content.clone()) {
                            eprintln!("Failed to copy to clipboard: {}", e);
                        } else {
                            std::thread::sleep(std::time::Duration::from_millis(200));
                        }
                    } else {
                         eprintln!("Clipboard interface unavailable.");
                    }
                }
            }
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn update_suggestions(&mut self) {
        if self.query.is_empty() {
            self.filtered_suggestions.clear();
        } else {
            let q_lower = self.query.to_lowercase();
            self.filtered_suggestions = self.all_titles
                .iter()
                .filter(|t| t.to_lowercase().contains(&q_lower))
                .cloned()
                .collect();
        }
        self.selected_index = 0;
    }
}

impl eframe::App for Amenu {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- AGGRESSIVE POSITIONING FIX ---
        // Force the window to (0,0) for the first 5 frames.
        // This fights the Window Manager trying to center it.
        if self.startup_counter < 5 {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(0.0, 0.0)));
            self.startup_counter += 1;
            // Request a repaint immediately to ensure the next frame happens instantly
            ctx.request_repaint();
        }
        // ----------------------------------

        let esc_pressed = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));
        let tab_pressed = ctx.input(|i| i.key_pressed(egui::Key::Tab));

        if esc_pressed {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        if tab_pressed && !self.filtered_suggestions.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.filtered_suggestions.len();
        }

        if enter_pressed {
            self.copy_and_quit(ctx);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.style_mut().spacing.item_spacing = egui::vec2(8.0, 0.0);

            ui.horizontal(|ui| {
                let font_id = egui::FontId::new(14.0, egui::FontFamily::Monospace);
                let text_width = ui.fonts(|f| {
                    f.layout_no_wrap(self.query.clone(), font_id, egui::Color32::WHITE).rect.width()
                });

                let box_width = (text_width + 20.0).max(50.0);

                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.query)
                        .hint_text("Type...")
                        .frame(false)
                        .desired_width(box_width)
                );

                response.request_focus();

                if response.changed() {
                    self.update_suggestions();
                }

                for (i, suggestion) in self.filtered_suggestions.iter().enumerate() {
                    let is_selected = i == self.selected_index;

                    let bg_color = if is_selected {
                        egui::Color32::from_rgb(217, 70, 239)
                    } else {
                        egui::Color32::from_rgb(35, 36, 41)
                    };

                    let fg_color = if is_selected {
                        egui::Color32::WHITE
                    } else if i == 1 {
                        egui::Color32::from_rgb(229, 192, 123)
                    } else {
                        egui::Color32::from_rgb(171, 178, 191)
                    };

                    let font_id = egui::FontId::new(14.0, egui::FontFamily::Monospace);
                    let galley = ui.painter().layout_no_wrap(suggestion.clone(), font_id.clone(), fg_color);

                    let padding = egui::vec2(12.0, 6.0);
                    let rect_size = galley.size() + padding;

                    let (rect, _resp) = ui.allocate_at_least(rect_size, egui::Sense::hover());

                    ui.painter().rect_filled(rect, 0.0, bg_color);
                    let text_pos = rect.min + egui::vec2(6.0, (rect.height() - galley.size().y) / 2.0);
                    ui.painter().galley(text_pos, galley, egui::Color32::PLACEHOLDER);
                }
            });
        });
    }
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = env::args().collect();
    let filename = if args.len() > 1 {
        args[1].clone()
    } else {
        "prompts".to_string()
    };

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_always_on_top()
            .with_inner_size([1920.0, 36.0])
            .with_position(egui::pos2(0.0, 0.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Amenu",
        native_options,
        Box::new(move |cc| Ok(Box::new(Amenu::new(cc, filename)))),
    )
}