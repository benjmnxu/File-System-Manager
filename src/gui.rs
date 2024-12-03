use eframe::egui;
use std::cell::RefCell;
use std::rc::Rc;
use tokio::sync::mpsc;

use crate::threads::*;

pub fn run_app(to_backend: mpsc::Sender<Command>, from_backend: mpsc::Receiver<BackendResponse>) -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Encapsulated Load Page",
        options,
        Box::new(|_cc| Box::new(MyApp::new(to_backend, from_backend))),
    )
}

#[derive(Clone)]
enum PageState {
    Home,
    Load { directory: String, display_text: String },
    Files { directory: String, display_text: String, input_text: String, response_text: String, ai_input: String, ai_output: String},
}

struct AppState {
    current_page: PageState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_page: PageState::Home,
        }
    }
}

struct MyApp {
    state: Rc<RefCell<AppState>>,
    to_backend: mpsc::Sender<Command>,
    from_backend: mpsc::Receiver<BackendResponse>,
}

impl MyApp {
    fn new(to_backend: mpsc::Sender<Command>, from_backend: mpsc::Receiver<BackendResponse>) -> Self {
        Self {
            state: Rc::new(RefCell::new(AppState::default())),
            to_backend,
            from_backend,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_backend_responses();
        
        egui::CentralPanel::default().show(ctx, |ui| {
            let current_page = self.state.borrow().current_page.clone();
            match current_page {
                PageState::Home => self.show_home_page(ui),
                PageState::Load { .. } => self.show_load_page(ui),
                PageState::Files { .. } => self.show_files_page(ui),
            }
        });
        ctx.request_repaint();
    }
}

impl MyApp {
    fn show_home_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("Home Page");
        if ui.button("Go to Load Page").clicked() {
            self.state.borrow_mut().current_page = PageState::Load {
                directory: String::new(),
                display_text: String::new(),
            };
        }
        if ui.button("Go to Files Page").clicked() {
            self.state.borrow_mut().current_page = PageState::Files {
                directory: String::new(),
                display_text: String::new(),
                input_text: String::new(),
                response_text: String::new(),
                ai_input: String::new(),
                ai_output: String::new(),
            };
        }
    }

    fn show_load_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("Load Page");
    
        let (directory, display_text) = {
            let mut state = self.state.borrow_mut();
            if let PageState::Load { directory, display_text } = &mut state.current_page {
                (directory.clone(), display_text.clone())
            } else {
                return; // Exit if not on the Load page
            }
        };
    
        // Work with extracted values
        let mut updated_directory = directory;
        if ui.text_edit_singleline(&mut updated_directory).changed() {
            self.state.borrow_mut().current_page = PageState::Load {
                directory: updated_directory.clone(),
                display_text: display_text.clone(),
            };
        }
    
        if ui.button("Load").clicked() {
            let trimmed_dir = updated_directory.trim();
            if trimmed_dir.is_empty() {
                let mut state = self.state.borrow_mut();
                if let PageState::Load { display_text, .. } = &mut state.current_page {
                    *display_text = "Please enter a valid directory.".to_string();
                }
            } else {
                let command = Command::LoadDirectory(trimmed_dir.to_string());
                if let Err(err) = self.to_backend.try_send(command) {
                    let mut state = self.state.borrow_mut();
                    if let PageState::Load { display_text, .. } = &mut state.current_page {
                        *display_text = format!("Failed to send command: {}", err);
                    }
                } else {
                    let mut state = self.state.borrow_mut();
                    state.current_page = PageState::Files {
                        directory: trimmed_dir.to_string(),
                        display_text: String::new(),
                        input_text: String::new(),
                        response_text: String::new(),
                        ai_input: String::new(),
                        ai_output: String::new()
                    };
                }
            }
        }
    
        if !display_text.is_empty() {
            self.display_message(ui, &display_text);
        }
    
        if ui.button("Back to Home").clicked() {
            let mut state = self.state.borrow_mut();
            state.current_page = PageState::Home;
        }
    }
        
    fn show_files_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("Files Page");
    
        if let PageState::Files {
            display_text,
            input_text,
            response_text,
            ai_input,
            ai_output,
            ..
        } = &mut self.state.borrow_mut().current_page
        {
            // Display file content
            egui::ScrollArea::vertical()
                .max_height(ui.available_height() * 0.9)
                .max_width(ui.available_width() * 0.7)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.label(display_text.clone());
                });
    
            let input_id = ui.make_persistent_id("command_input");

            // Render the input field and check for Enter key
            let response = egui::TextEdit::singleline(input_text)
                .id(input_id)
                .show(ui);
            
            // Check if the Enter key was pressed and handle input
            if response.response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let trimmed_input = input_text.trim();
                if !trimmed_input.is_empty() {
                    let command = parse_command(trimmed_input);
                    if let Err(err) = self.to_backend.try_send(command) {
                        *response_text = format!("Error sending command: {}", err);
                    }
                    input_text.clear();
                }

                // Reapply focus to the input field
                ui.memory_mut(|mem| mem.request_focus(input_id));
            }

            egui::Area::new("top_right_box")
            .fixed_pos(egui::Pos2::new(ui.available_width() * (1.0 - 0.28), 0.0))
            .show(ui.ctx(), |ui| {
                ui.heading("ai");
                egui::ScrollArea::vertical()
                    .max_height(ui.available_height() * 0.9)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.label(ai_output.clone());
                    });
                let ai_input_id = ui.make_persistent_id("ai command_input");

                // Render the input field and check for Enter key
                let ai_response = egui::TextEdit::singleline(ai_input)
                    .id(ai_input_id)
                    .show(ui);
                
                // Check if the Enter key was pressed and handle input
                if ai_response.response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let trimmed_input = ai_input.trim();
                    if !trimmed_input.is_empty() {
                        let command = Command::AISuggestion(trimmed_input.to_string());
                        if let Err(err) = self.to_backend.try_send(command) {
                            *response_text = format!("Error sending command: {}", err);
                        }
                        ai_input.clear();
                    }

                    // Reapply focus to the input field
                    ui.memory_mut(|mem| mem.request_focus(ai_input_id));
                }

                if ui.button("apply suggestions").clicked() {
                    let command = Command::AIConfirm;
                    if let Err(err) = self.to_backend.try_send(command) {
                        *response_text = format!("Error sending command: {}", err);
                    }
                }

            });
        }

        if ui.button("Back to Home").clicked() {
            self.state.borrow_mut().current_page = PageState::Home;
        }
    }
    

    fn display_message(&self, ui: &mut egui::Ui, message: &str) {
        if message.contains("Error") {
            ui.colored_label(egui::Color32::RED, message);
        } else {
            ui.colored_label(egui::Color32::GREEN, message);
        }
    }
    fn handle_backend_responses(&mut self) {
        while let Ok(response) = self.from_backend.try_recv() {
            match response {
                BackendResponse::Response(data) => {
                    // Handle successful response, update state or UI
                    if let PageState::Files { display_text, .. } = &mut self.state.borrow_mut().current_page {
                        display_text.push('\n');
                        display_text.push_str(&data);
                    }
                }
                BackendResponse::AIResponse(message) => {
                    if let PageState::Files { ai_output, .. } = &mut self.state.borrow_mut().current_page {
                        ai_output.push('\n');
                        ai_output.push_str(&message);
                    }
                }
                BackendResponse::Error(error) => {
                    // Handle backend error, update state or UI
                    if let PageState::Load { display_text, .. } = &mut self.state.borrow_mut().current_page {
                        *display_text = format!("Error: {}", error);
                    }
                }
            }
        }
    }
}
