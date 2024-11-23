use eframe::egui;

pub fn run_app() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Encapsulated Load Page",
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    )
}

#[derive(Default)]
struct MyApp {
    current_page: Page, // Tracks the current page
    load_page: LoadPage, // The LoadPage struct encapsulates its own state
}

enum Page {
    Home,
    Load,
}

impl Default for Page {
    fn default() -> Self {
        Page::Home
    }
}


struct LoadPage {
    directory: String,   // Stores the directory input
    display_text: String, // Stores the text to display after clicking "Load"
}

impl Default for LoadPage {
    fn default() -> Self {
        LoadPage {
            directory: String::new(),
            display_text: String::new(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_page {
                Page::Home => self.show_home_page(ui),
                Page::Load => self.load_page.show(ui, &mut self.current_page),
            }
        });
    }
}

impl MyApp {
    fn show_home_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("Home Page");
        if ui.button("Go to Load Page").clicked() {
            self.current_page = Page::Load;
        }
    }
}

impl LoadPage {
    fn show(&mut self, ui: &mut egui::Ui, current_page: &mut Page) {
        ui.heading("Load");
        ui.label("Load in the folder you want to clean (For entire system, input '/').");

        // Input for directory
        ui.text_edit_singleline(&mut self.directory);

        // Button to display the directory
        if ui.button("Load").clicked() {
            self.display_text = format!("Directory loaded: {}", self.directory);
        }

        // Display the directory if it has been loaded
        if !self.display_text.is_empty() {
            ui.label(&self.display_text);
        }

        // Button to go back to the Home page
        if ui.button("Back to Home").clicked() {
            *current_page = Page::Home;
        }
    }
}

