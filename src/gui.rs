use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use eframe::egui;
use egui::{Color32, FontId, RichText, TextEdit};

#[derive(PartialEq, Clone, Copy)]
enum TextFormat {
    Normal,
    Bold,
    Underlined,
    BoldUnderlined,
}

struct TextEditorApp {
    content: String,
    current_format: TextFormat,
    save_filename: String,
    save_dialog_open: bool,
    output_path: String,
    status_message: Option<String>,
}

impl Default for TextEditorApp {
    fn default() -> Self {
        Self {
            content: String::new(),
            current_format: TextFormat::Normal,
            save_filename: String::new(),
            save_dialog_open: false,
            output_path: "output".to_string(),
            status_message: None,
        }
    }
}

impl eframe::App for TextEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Text Editor");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Save").clicked() {
                        self.enter_save_mode();
                    }
                    
                    // Use current format by value
                    let format = self.current_format;
                    
                    // Create button for bold formatting
                    let bold_text = if is_bold(format) {
                        RichText::new("B").color(Color32::YELLOW)
                    } else {
                        RichText::new("B")
                    };
                    
                    if ui.button(bold_text).clicked() {
                        self.toggle_bold();
                    }
                    
                    // Create button for underline formatting
                    let underline_text = if is_underlined(format) {
                        RichText::new("U").color(Color32::YELLOW)
                    } else {
                        RichText::new("U")
                    };
                    
                    if ui.button(underline_text).clicked() {
                        self.toggle_underline();
                    }
                });
            });
            
            ui.separator();
            
            // Store the current format before mutable borrow
            let font = self.get_font();
            
            // Main text area
            let text_edit = TextEdit::multiline(&mut self.content)
                .font(font)
                .desired_width(f32::INFINITY)
                .desired_rows(20);
            
            ui.add(text_edit);
            
            if let Some(message) = &self.status_message {
                ui.separator();
                ui.colored_label(Color32::GREEN, message);
            }
        });
        
        // Handle save dialog separately to avoid borrowing issues
        self.handle_save_dialog(ctx);
    }
}

// Helper functions for format detection
fn is_bold(format: TextFormat) -> bool {
    matches!(format, TextFormat::Bold | TextFormat::BoldUnderlined)
}

fn is_underlined(format: TextFormat) -> bool {
    matches!(format, TextFormat::Underlined | TextFormat::BoldUnderlined)
}

impl TextEditorApp {
    fn get_font(&self) -> FontId {
        let size = 16.0;
        match self.current_format {
            TextFormat::Normal => FontId::proportional(size),
            TextFormat::Bold => FontId::proportional(size),
            TextFormat::Underlined => FontId::proportional(size),
            TextFormat::BoldUnderlined => FontId::proportional(size),
        }
    }
    
    fn toggle_bold(&mut self) {
        self.current_format = match self.current_format {
            TextFormat::Normal => TextFormat::Bold,
            TextFormat::Bold => TextFormat::Normal,
            TextFormat::Underlined => TextFormat::BoldUnderlined,
            TextFormat::BoldUnderlined => TextFormat::Underlined,
        };
    }
    
    fn toggle_underline(&mut self) {
        self.current_format = match self.current_format {
            TextFormat::Normal => TextFormat::Underlined,
            TextFormat::Bold => TextFormat::BoldUnderlined,
            TextFormat::Underlined => TextFormat::Normal,
            TextFormat::BoldUnderlined => TextFormat::Bold,
        };
    }
    
    fn enter_save_mode(&mut self) {
        self.save_dialog_open = true;
        
        // Generate default filename from first three words
        let first_three_words = self.get_first_three_words();
        let default_filename = if first_three_words.is_empty() {
            "untitled.txt".to_string()
        } else {
            format!("{}.txt", first_three_words)
        };
        
        self.save_filename = default_filename;
    }
    
    fn get_first_three_words(&self) -> String {
        let words: Vec<&str> = self.content
            .split_whitespace()
            .take(3)
            .collect();
        
        words.join("_")
    }
    
    fn handle_save_dialog(&mut self, ctx: &egui::Context) {
        if !self.save_dialog_open {
            return;
        }
        
        let mut open = self.save_dialog_open;
        let mut output_path = self.output_path.clone();
        let mut save_filename = self.save_filename.clone();
        let mut save_clicked = false;
        let mut cancel_clicked = false;
        
        egui::Window::new("Save File")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Output folder:");
                    ui.text_edit_singleline(&mut output_path);
                });
                
                ui.horizontal(|ui| {
                    ui.label("Filename:");
                    ui.text_edit_singleline(&mut save_filename);
                });
                
                ui.separator();
                
                ui.horizontal(|ui| {
                    save_clicked = ui.button("Save").clicked();
                    cancel_clicked = ui.button("Cancel").clicked();
                });
            });
        
        // Update values after dialog closes
        self.output_path = output_path;
        self.save_filename = save_filename;
        
        if save_clicked {
            match self.save_file() {
                Ok(_) => {
                    let path = Path::new(&self.output_path).join(&self.save_filename);
                    self.status_message = Some(format!("File saved to: {}", path.display()));
                },
                Err(err) => {
                    self.status_message = Some(format!("Error saving file: {}", err));
                }
            }
            open = false;
        }
        
        if cancel_clicked {
            open = false;
        }
        
        self.save_dialog_open = open;
    }
    
    fn save_file(&self) -> std::io::Result<()> {
        // Create output directory if it doesn't exist
        let output_dir = Path::new(&self.output_path);
        if !output_dir.exists() {
            fs::create_dir_all(output_dir)?;
        }
        
        let filepath = output_dir.join(&self.save_filename);
        let mut file = File::create(&filepath)?;
        
        // Write content to file
        file.write_all(self.content.as_bytes())?;
        
        Ok(())
    }
}

fn main() -> Result<(), eframe::Error> {
    // Use default options to avoid version compatibility issues
    let options = eframe::NativeOptions::default();
    
    eframe::run_native(
        "Text Editor",
        options,
        Box::new(|_cc| Ok(Box::new(TextEditorApp::default()))),
    )
} 