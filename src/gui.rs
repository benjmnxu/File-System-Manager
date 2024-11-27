use eframe::egui;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::{mpsc, Arc, Mutex};
use std::cell::RefCell;

use crate::system::FileSystemNode;
use crate::kernel::Kernel;
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
    Load {
        directory: String,
        display_text: String,
    },
    Files {
        directory: String,
        display_text: String,
        input_text: String,
        response_text: String,
    },
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
    load_page: LoadPage,
    files_page: FilesPage,
    to_backend: mpsc::Sender<Command>,
    from_backend: mpsc::Receiver<BackendResponse>,
}

#[derive(Default)]
struct LoadPage;

#[derive(Default)]
struct FilesPage;

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let current_page = self.state.borrow().current_page.clone();
        egui::CentralPanel::default().show(ctx, |ui| match current_page {
            PageState::Home => self.show_home_page(ui),
            PageState::Load { .. } => self.load_page.show(ui, Rc::clone(&self.state), &self.to_backend, &self.from_backend),
            PageState::Files { .. } => self.files_page.show(ui, Rc::clone(&self.state), &self.to_backend, &self.from_backend),
        });
    }
}


impl MyApp {
    fn new(to_backend: mpsc::Sender<Command>, from_backend: mpsc::Receiver<BackendResponse>) -> Self {
        Self {
            state: Rc::new(RefCell::new(AppState::default())),
            load_page: LoadPage::default(),
            files_page: FilesPage::default(),
            to_backend,
            from_backend
        }
    }
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
            };
        }
    }
}

impl LoadPage {
    fn show(
        &self,
        ui: &mut egui::Ui,
        state: Rc<RefCell<AppState>>,
        to_backend: &mpsc::Sender<Command>,
        from_backend: &mpsc::Receiver<BackendResponse>,
    ) {
        // Render UI using the `directory` and `display_text` fields of the page state
        ui.heading("Load");

        {
            let mut current_state = state.borrow_mut();
            if let PageState::Load { directory, .. } = &mut current_state.current_page {
                // Allow user to edit the `directory` directly
                ui.text_edit_singleline(directory);
            }
        }

        let load_clicked = ui.button("Load").clicked();
        let back_to_home_clicked = ui.button("Back to Home").clicked();

        // Handle "Load" button click
        if load_clicked {
            let mut current_state = state.borrow_mut();
            if let PageState::Load { directory, display_text } = &mut current_state.current_page {
                if directory.is_empty() {
                    *display_text = "Please enter a valid directory.".to_string();
                } else {
                    if let Err(err) = to_backend.send(Command::LoadDirectory(directory.clone())) {
                        *display_text = format!("Failed to send command to backend: {}", err);
                    }
                }
            }
        }

        // Handle backend response
        if let Ok(response) = from_backend.try_recv() {
            let mut current_state = state.borrow_mut();
            if let PageState::Load { directory, display_text } = &mut current_state.current_page {
                match response {
                    BackendResponse::Response(_) => {
                        current_state.current_page = PageState::Files {
                            directory: directory.clone(),
                            display_text: String::new(),
                            input_text: String::new(),
                            response_text: String::new(),
                        };
                    }
                    BackendResponse::Error(error) => {
                        *display_text = format!("Error: {}", error);
                    }
                }
            }
        }

        // Render feedback or error messages
        {
            let current_state = state.borrow();
            if let PageState::Load { display_text, .. } = &current_state.current_page {
                if !display_text.is_empty() {
                    if display_text.contains("Error") {
                        ui.colored_label(egui::Color32::RED, display_text);
                    } else {
                        ui.colored_label(egui::Color32::GREEN, display_text);
                    }
                }
            }
        }

        // Handle "Back to Home" button click
        if back_to_home_clicked {
            let mut current_state = state.borrow_mut();
            current_state.current_page = PageState::Home;
        }
    }
}


impl FilesPage {
    fn show(
        &self,
        ui: &mut egui::Ui,
        state: Rc<RefCell<AppState>>,
        to_backend: &mpsc::Sender<Command>,
        from_backend: &mpsc::Receiver<BackendResponse>,
    ) {
        // Access and modify the current page state
        if let PageState::Files {
            directory,
            display_text,
            input_text,
            response_text,
        } = &mut state.borrow_mut().current_page
        {
            ui.heading("Files");

            // Send a request to the backend if `display_text` is empty
            if display_text.is_empty() {
                if let Err(err) = to_backend.send(Command::Display) {
                    *display_text = format!("Failed to send command to backend: {}", err);
                }
            }

            // Check for backend response
            while let Ok(response) = from_backend.try_recv() {
                match response {
                    BackendResponse::Response(text) => {
                        display_text.push_str(&format!("{}\n", text));
                    }
                    BackendResponse::Error(error) => {
                        *display_text = format!("Error: {}", error);
                    }
                }
            }

            // Create a scrollable area for `display_text`
            let max_height = ui.available_height() * 0.8;
            let max_width = ui.available_width() * 0.7; // Capture available width

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .max_height(max_height)
                .max_width(max_width)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.label(display_text.clone());
                });

            // Add a taller input box for user commands with the same width as scroll area
            let input_response = ui.add(
                egui::TextEdit::singleline(input_text)
                    .desired_width(max_width)
                    .hint_text("Enter command...")
                    .margin(egui::vec2(4.0, 10.0)), // Taller padding
            );

            // Handle "Enter" key to send commands to the backend
            if input_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if !input_text.trim().is_empty() {
                    let command = parse_command(input_text.trim());
                    to_backend.send(command).unwrap();
                    input_text.clear(); // Clear input after sending the command
                }
            }

            // Display response or error messages
            if !response_text.is_empty() {
                ui.label(egui::RichText::new(response_text.clone()).italics());
            }
        }

        // Back to home button
        if ui.button("Back to Home").clicked() {
            state.borrow_mut().current_page = PageState::Home;
        }
    }
}
