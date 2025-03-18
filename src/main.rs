use std::io::{self, stdout, Write};
use std::time::Duration;
use std::fs::{self, File};
use std::path::Path;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Line},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};

#[derive(Clone)]
enum TextFormat {
    Normal,
    Bold,
    Underlined,
}

enum EditorMode {
    Normal,
    SaveAs,
}

struct TextEditor {
    text: Vec<Vec<(String, TextFormat)>>,
    cursor_x: usize,
    cursor_y: usize,
    current_format: TextFormat,
    mode: EditorMode,
    save_filename: String,
    status_message: Option<String>,
}

impl TextEditor {
    fn new() -> Self {
        TextEditor {
            text: vec![vec![("".to_string(), TextFormat::Normal)]],
            cursor_x: 0,
            cursor_y: 0,
            current_format: TextFormat::Normal,
            mode: EditorMode::Normal,
            save_filename: String::new(),
            status_message: None,
        }
    }

    fn insert_char(&mut self, c: char) {
        if self.text[self.cursor_y].is_empty() {
            self.text[self.cursor_y].push((c.to_string(), self.current_format.clone()));
        } else {
            let segment_idx = self.find_segment_at_cursor();
            let format_is_same = match &self.text[self.cursor_y][segment_idx].1 {
                f => std::mem::discriminant(f) == std::mem::discriminant(&self.current_format),
            };
            
            // If the cursor is at the end of the segment
            let segment_start_x = self.get_segment_start_x(segment_idx);
            let text_len = self.text[self.cursor_y][segment_idx].0.len();
            if self.cursor_x == segment_start_x + text_len {
                // If the format matches, append to current segment
                if format_is_same {
                    self.text[self.cursor_y][segment_idx].0.push(c);
                } else {
                    // Create a new segment with the current format
                    self.text[self.cursor_y].insert(
                        segment_idx + 1,
                        (c.to_string(), self.current_format.clone()),
                    );
                }
            } else {
                // If cursor is in the middle of a segment
                let cursor_in_segment = self.cursor_x - segment_start_x;
                
                // Get the current text
                let current_text = std::mem::take(&mut self.text[self.cursor_y][segment_idx].0);
                let current_format = self.text[self.cursor_y][segment_idx].1.clone();
                
                // Split the text at cursor position
                let (left, right) = current_text.split_at(cursor_in_segment);
                
                // Update the current segment with left part + new char
                self.text[self.cursor_y][segment_idx] = (format!("{}{}", left, c), current_format.clone());
                
                // Insert the right part as a new segment if not empty
                if !right.is_empty() {
                    self.text[self.cursor_y].insert(
                        segment_idx + 1,
                        (right.to_string(), current_format),
                    );
                }
            }
        }
        self.cursor_x += 1;
    }

    fn backspace(&mut self) {
        if self.cursor_x > 0 {
            let segment_idx = self.find_segment_at_cursor();
            
            // Calculate cursor position within the segment
            let start_x = self.get_segment_start_x(segment_idx);
            let cursor_in_segment = self.cursor_x - start_x;
            
            if cursor_in_segment > 0 {
                // Remove character from current segment
                let text = &mut self.text[self.cursor_y][segment_idx].0;
                text.remove(cursor_in_segment - 1);
                
                // If segment is now empty, remove it
                if text.is_empty() {
                    self.text[self.cursor_y].remove(segment_idx);
                }
            } else {
                // We're at the beginning of a segment, need to merge with previous
                if segment_idx > 0 {
                    let prev_text = self.text[self.cursor_y][segment_idx - 1].0.clone();
                    let prev_len = prev_text.len();
                    
                    // Remove last character from previous segment
                    self.text[self.cursor_y][segment_idx - 1].0 = prev_text[..prev_len - 1].to_string();
                    
                    // If previous segment is now empty, remove it
                    if self.text[self.cursor_y][segment_idx - 1].0.is_empty() {
                        self.text[self.cursor_y].remove(segment_idx - 1);
                    }
                }
            }
            
            self.cursor_x -= 1;
            
            // If line is now empty, add an empty segment with normal formatting
            if self.text[self.cursor_y].is_empty() {
                self.text[self.cursor_y].push(("".to_string(), TextFormat::Normal));
            }
        } else if self.cursor_y > 0 {
            // Handle backspace at beginning of line
            let current_line = self.text.remove(self.cursor_y);
            self.cursor_y -= 1;
            self.cursor_x = self.line_length(self.cursor_y);
            
            // Append current line to previous line
            self.text[self.cursor_y].extend(current_line);
        }
    }

    fn new_line(&mut self) {
        let segment_idx = self.find_segment_at_cursor();
        let mut new_line = Vec::new();
        
        if segment_idx < self.text[self.cursor_y].len() {
            let cursor_in_segment = self.cursor_x - self.get_segment_start_x(segment_idx);
            
            // Get current segment
            let current_text = std::mem::take(&mut self.text[self.cursor_y][segment_idx].0);
            let current_format = self.text[self.cursor_y][segment_idx].1.clone();
            
            if cursor_in_segment < current_text.len() {
                // Split current segment at cursor
                let (left, right) = current_text.split_at(cursor_in_segment);
                
                // Update current segment with left part
                self.text[self.cursor_y][segment_idx].0 = left.to_string();
                
                // Add right part to new line with same format
                new_line.push((right.to_string(), current_format));
                
                // Add remaining segments to new line
                let remaining = self.text[self.cursor_y].split_off(segment_idx + 1);
                new_line.extend(remaining);
            } else {
                // Cursor at end of segment, restore the text
                self.text[self.cursor_y][segment_idx].0 = current_text;
                
                // Move remaining segments to new line
                let remaining = self.text[self.cursor_y].split_off(segment_idx + 1);
                new_line.extend(remaining);
            }
        }
        
        // If new line is empty, add an empty segment with current format
        if new_line.is_empty() {
            new_line.push(("".to_string(), self.current_format.clone()));
        }
        
        // Insert new line after current line
        self.text.insert(self.cursor_y + 1, new_line);
        self.cursor_y += 1;
        self.cursor_x = 0;
    }

    fn move_cursor_left(&mut self) {
        if self.cursor_x > 0 {
            self.cursor_x -= 1;
        } else if self.cursor_y > 0 {
            self.cursor_y -= 1;
            self.cursor_x = self.line_length(self.cursor_y);
        }
    }

    fn move_cursor_right(&mut self) {
        let line_len = self.line_length(self.cursor_y);
        if self.cursor_x < line_len {
            self.cursor_x += 1;
        } else if self.cursor_y < self.text.len() - 1 {
            self.cursor_y += 1;
            self.cursor_x = 0;
        }
    }

    fn move_cursor_up(&mut self) {
        if self.cursor_y > 0 {
            self.cursor_y -= 1;
            let line_len = self.line_length(self.cursor_y);
            if self.cursor_x > line_len {
                self.cursor_x = line_len;
            }
        }
    }

    fn move_cursor_down(&mut self) {
        if self.cursor_y < self.text.len() - 1 {
            self.cursor_y += 1;
            let line_len = self.line_length(self.cursor_y);
            if self.cursor_x > line_len {
                self.cursor_x = line_len;
            }
        }
    }

    fn toggle_bold(&mut self) {
        self.current_format = match self.current_format {
            TextFormat::Normal => TextFormat::Bold,
            TextFormat::Bold => TextFormat::Normal,
            TextFormat::Underlined => TextFormat::Bold,
        };
    }

    fn toggle_underline(&mut self) {
        self.current_format = match self.current_format {
            TextFormat::Normal => TextFormat::Underlined,
            TextFormat::Bold => TextFormat::Underlined,
            TextFormat::Underlined => TextFormat::Normal,
        };
    }

    fn find_segment_at_cursor(&self) -> usize {
        let mut current_x = 0;
        for (idx, (text, _)) in self.text[self.cursor_y].iter().enumerate() {
            let segment_len = text.len();
            if current_x <= self.cursor_x && self.cursor_x <= current_x + segment_len {
                return idx;
            }
            current_x += segment_len;
        }
        self.text[self.cursor_y].len().saturating_sub(1)
    }

    fn get_segment_start_x(&self, segment_idx: usize) -> usize {
        let mut start_x = 0;
        for i in 0..segment_idx {
            start_x += self.text[self.cursor_y][i].0.len();
        }
        start_x
    }

    fn line_length(&self, line_idx: usize) -> usize {
        self.text[line_idx].iter().map(|(text, _)| text.len()).sum()
    }

    fn render<'a>(&self) -> Vec<Line<'a>> {
        let mut rendered_lines = Vec::new();
        
        for line in &self.text {
            let mut spans = Vec::new();
            
            for (text, format) in line {
                let style = match format {
                    TextFormat::Normal => Style::default(),
                    TextFormat::Bold => Style::default().add_modifier(Modifier::BOLD),
                    TextFormat::Underlined => Style::default().add_modifier(Modifier::UNDERLINED),
                };
                
                spans.push(Span::styled(text.clone(), style));
            }
            
            rendered_lines.push(Line::from(spans));
        }
        
        rendered_lines
    }

    fn get_format_name(&self) -> &'static str {
        match self.current_format {
            TextFormat::Normal => "Normal",
            TextFormat::Bold => "Bold",
            TextFormat::Underlined => "Underlined",
        }
    }

    fn enter_save_mode(&mut self) {
        self.mode = EditorMode::SaveAs;
        
        // Generate default filename based on first three words
        let first_three_words = self.get_first_three_words();
        let default_filename = if first_three_words.is_empty() {
            "untitled.txt".to_string()
        } else {
            format!("{}.txt", first_three_words.replace(" ", "_"))
        };
        
        self.save_filename = default_filename;
    }
    
    fn exit_save_mode(&mut self) {
        self.mode = EditorMode::Normal;
        self.save_filename.clear();
    }
    
    fn get_first_three_words(&self) -> String {
        let mut words = Vec::new();
        
        for line in &self.text {
            if !line.is_empty() {
                for (text, _) in line {
                    for word in text.split_whitespace() {
                        if !word.is_empty() {
                            words.push(word);
                            if words.len() >= 3 {
                                return words.join("_");
                            }
                        }
                    }
                }
            }
            
            if words.len() >= 3 {
                break;
            }
        }
        
        words.join("_")
    }
    
    fn save_file(&mut self) -> io::Result<()> {
        // Create output directory if it doesn't exist
        let output_dir = Path::new("output");
        if !output_dir.exists() {
            fs::create_dir_all(output_dir)?;
        }
        
        let filepath = output_dir.join(&self.save_filename);
        let mut file = File::create(&filepath)?;
        
        // Write content to file
        for (i, line) in self.text.iter().enumerate() {
            for (text, _) in line {
                file.write_all(text.as_bytes())?;
            }
            
            // Add newline except for the last line
            if i < self.text.len() - 1 {
                file.write_all(b"\n")?;
            }
        }
        
        self.status_message = Some(format!("File saved to: {}", filepath.display()));
        self.mode = EditorMode::Normal;
        
        Ok(())
    }
    
    fn update_save_filename(&mut self, c: char) {
        if c == '\n' {
            // Submit filename
            return;
        } else if c == '\u{8}' || c == '\u{7f}' {  // Backspace or Delete
            self.save_filename.pop();
        } else {
            self.save_filename.push(c);
        }
    }
}

fn main() -> Result<(), io::Error> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut editor = TextEditor::new();
    let mut running = true;

    while running {
        // Draw UI
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(f.size());

            // Text editor area
            let paragraph = Paragraph::new(editor.render())
                .block(Block::default().borders(Borders::ALL).title("Text Editor"))
                .wrap(Wrap { trim: false });
            f.render_widget(paragraph, chunks[0]);

            // Status bar
            let status_bar_content = match editor.mode {
                EditorMode::Normal => {
                    if let Some(message) = &editor.status_message {
                        Line::from(vec![
                            Span::styled(message, Style::default().fg(Color::Green)),
                        ])
                    } else {
                        Line::from(vec![
                            Span::raw("Format: "),
                            Span::styled(
                                editor.get_format_name(),
                                Style::default().fg(Color::Yellow),
                            ),
                            Span::raw(" | Cursor: "),
                            Span::styled(
                                format!("({}, {})", editor.cursor_x, editor.cursor_y),
                                Style::default().fg(Color::Yellow),
                            ),
                            Span::raw(" | Ctrl+B: Bold | Ctrl+U: Underline | Ctrl+S: Save | Ctrl+X: Quit"),
                        ])
                    }
                },
                EditorMode::SaveAs => {
                    Line::from(vec![
                        Span::raw("Save as: "),
                        Span::styled(
                            &editor.save_filename,
                            Style::default().fg(Color::Yellow),
                        ),
                        Span::raw(" (Press Enter to save, Esc to cancel)"),
                    ])
                }
            };
            
            let status_bar = Paragraph::new(status_bar_content)
                .style(Style::default().bg(Color::Blue));
            f.render_widget(status_bar, chunks[1]);
        })?;

        // Handle input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match editor.mode {
                    EditorMode::Normal => {
                        match key {
                            KeyEvent {
                                code: KeyCode::Char('x'),
                                modifiers: KeyModifiers::CONTROL,
                                ..
                            } => {
                                running = false;
                            }
                            KeyEvent {
                                code: KeyCode::Char('b'),
                                modifiers: KeyModifiers::CONTROL,
                                ..
                            } => {
                                editor.toggle_bold();
                                editor.status_message = None;
                            }
                            KeyEvent {
                                code: KeyCode::Char('u'),
                                modifiers: KeyModifiers::CONTROL,
                                ..
                            } => {
                                editor.toggle_underline();
                                editor.status_message = None;
                            }
                            KeyEvent {
                                code: KeyCode::Char('s'),
                                modifiers: KeyModifiers::CONTROL,
                                ..
                            } => {
                                editor.enter_save_mode();
                                editor.status_message = None;
                            }
                            KeyEvent {
                                code: KeyCode::Char(c),
                                modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
                                ..
                            } => {
                                editor.insert_char(c);
                                editor.status_message = None;
                            }
                            KeyEvent {
                                code: KeyCode::Enter,
                                ..
                            } => {
                                editor.new_line();
                                editor.status_message = None;
                            }
                            KeyEvent {
                                code: KeyCode::Backspace,
                                ..
                            } => {
                                editor.backspace();
                                editor.status_message = None;
                            }
                            KeyEvent {
                                code: KeyCode::Left,
                                ..
                            } => {
                                editor.move_cursor_left();
                                editor.status_message = None;
                            }
                            KeyEvent {
                                code: KeyCode::Right,
                                ..
                            } => {
                                editor.move_cursor_right();
                                editor.status_message = None;
                            }
                            KeyEvent {
                                code: KeyCode::Up,
                                ..
                            } => {
                                editor.move_cursor_up();
                                editor.status_message = None;
                            }
                            KeyEvent {
                                code: KeyCode::Down,
                                ..
                            } => {
                                editor.move_cursor_down();
                                editor.status_message = None;
                            }
                            _ => {}
                        }
                    },
                    EditorMode::SaveAs => {
                        match key.code {
                            KeyCode::Enter => {
                                if let Err(e) = editor.save_file() {
                                    editor.status_message = Some(format!("Error saving file: {}", e));
                                }
                                editor.mode = EditorMode::Normal;
                            },
                            KeyCode::Esc => {
                                editor.exit_save_mode();
                            },
                            KeyCode::Char(c) => {
                                editor.update_save_filename(c);
                            },
                            KeyCode::Backspace => {
                                editor.save_filename.pop();
                            },
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
