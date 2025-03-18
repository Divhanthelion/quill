# Simple Text Editor

A text editor with formatting capabilities written in Rust.

## Features

- Text input and editing
- Formatting text (bold, underline)
- Multi-line support
- Cursor navigation
- Save functionality (saves to "output" folder)
- Available as both terminal-based and GUI application

## Terminal Version

### Usage

Run the terminal version:

```
cargo run --bin terminal
```

### Keyboard Controls

- **Typing**: Type any character to insert text
- **Enter**: Insert a new line
- **Backspace**: Delete character before cursor
- **Arrow keys**: Navigate the cursor
- **Ctrl+B**: Toggle bold formatting
- **Ctrl+U**: Toggle underline formatting
- **Ctrl+S**: Save file
- **Ctrl+X**: Quit the application

## GUI Version

The GUI version is still under development but can be tested with:

```
cargo run --bin gui
```

### GUI Controls

- **Text area**: Type your text here
- **B button**: Toggle bold formatting
- **U button**: Toggle underline formatting
- **Save button**: Open save dialog

## Building

```
cargo build --release
```

The executables will be available at:
- `target/release/terminal` - Terminal version
- `target/release/gui` - GUI version (when fully implemented)

## Dependencies

- ratatui: Terminal UI library
- crossterm: Terminal manipulation
- unicode-width: Unicode width calculation
- egui/eframe: GUI toolkit (for GUI version) 