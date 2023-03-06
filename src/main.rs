use crossterm::event::{read, Event, KeyCode};
use crossterm::style::Print;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::{cursor, queue, terminal, QueueableCommand};
use serde_derive::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, stdout, BufRead, Write};
use std::{env, fs};

struct Line {
    chars: String,
}

enum EditorMode {
    Normal,
    Insert,
}

enum EditorAction {
    MoveLeft,
    MoveDown,
    MoveRight,
    MoveUp,
    Save,
    InsertMode,
    Quit,
    NormalMode,
    Backspace,
    SplitLine,
    InsertChar,
}

struct EditorFile {
    lines: Vec<Line>,
    row_pos: usize,
    col_pos: usize,
}

struct Editor {
    mode: EditorMode,
    file: Option<EditorFile>,
    num_rows: usize,
    num_cols: usize,
}

impl Editor {
    fn open_file(&mut self, filename: &str) -> io::Result<()> {
        let file = File::open(filename)?;
        let mut editor_file = EditorFile {
            lines: Vec::new(),
            row_pos: 0,
            col_pos: 0,
        };

        let lines = io::BufReader::new(file).lines();
        for line in lines {
            let editor_line = Line { chars: line? };
            editor_file.lines.push(editor_line);
        }

        self.file = Some(editor_file);

        Ok(())
    }
}

#[derive(Deserialize)]
struct Keys {
    normal: HashMap<String, String>,
    insert: HashMap<String, String>,
}

// Top level struct to hold the TOML data.
#[derive(Deserialize)]
struct Data {
    keys: Keys,
}

fn main() -> io::Result<()> {
    let filename = "config.toml";

    // Read the contents of the file using a `match` block
    // to return the `data: Ok(c)` as a `String`
    // or handle any `errors: Err(_)`.
    let contents = fs::read_to_string(filename).unwrap();

    let data: Data = toml::from_str(&contents).unwrap();

    let stringToAction = HashMap::from([("move_left", EditorAction::MoveLeft)]);

    let args: Vec<String> = env::args().collect();
    let filename = &args[1];

    let window_size = match terminal::size() {
        Ok(size) => size,
        Err(err) => {
            eprintln!("Could not determine terminal size: {}", err);
            return Err(err);
        }
    };
    let mut editor = Editor {
        mode: EditorMode::Normal,
        file: None,
        num_rows: (window_size.1 as usize),
        num_cols: (window_size.0 as usize),
    };

    let mut solock = stdout().lock();
    if let Err(err) = editor.open_file(filename) {
        eprintln!("Error while opening \"{}\": {}", filename, err);
        return Err(err);
    };

    let mut editor_file = match editor.file {
        Some(file) => file,
        None => panic!("No file present"),
    };

    enable_raw_mode().unwrap();
    solock
        .queue(terminal::Clear(terminal::ClearType::All))
        .unwrap()
        .queue(cursor::MoveTo(0, 0))
        .unwrap()
        .queue(cursor::SetCursorStyle::SteadyBlock)
        .unwrap();

    let mut iter = editor_file.lines.iter().take(editor.num_rows).peekable();

    while let Some(line) = iter.next() {
        if iter.peek().is_some() {
            write!(solock, "{}\r\n", line.chars).unwrap();
        } else {
            write!(solock, "{}", line.chars).unwrap();
        }
    }

    solock.queue(cursor::MoveTo(0, 0)).unwrap();

    solock.flush().unwrap();

    loop {
        let event = read().expect("Failed to read");

        let input = match event {
            Event::Key(keyevent) => match keyevent.code {
                KeyCode::Up => "ARROW_UP".to_string(),
                KeyCode::Down => "ARROW_DOWN".to_string(),
                KeyCode::Left => "ARROW_LEFT".to_string(),
                KeyCode::Right => "ARROW_RIGHT".to_string(),
                KeyCode::Esc => "ESC".to_string(),
                KeyCode::Enter => "ENTER".to_string(),
                KeyCode::Backspace => "BACKSPACE".to_string(),
                KeyCode::Char(c) => c.to_string(),
                _ => continue,
            },
            _ => continue,
        };

        let action = match editor.mode {
            EditorMode::Normal => match data.keys.normal.get(&input) {
                Some(action) => {
                    if let Some(ea) = stringToAction.get(action.as_str()) {
                        ea
                    } else {
                        continue;
                    }
                }
                None => continue,
            },
            EditorMode::Insert => match data.keys.insert.get(&input) {
                Some(action) => {
                    if let Some(ea) = stringToAction.get(action.as_str()) {
                        ea
                    } else {
                        continue;
                    }
                }
                None => &EditorAction::InsertChar,
            },
        };
        match action {
            EditorAction::Quit => break,
            EditorAction::MoveLeft => {
                if editor_file.col_pos > 0 {
                    editor_file.col_pos -= 1;
                    solock.queue(cursor::MoveLeft(1)).unwrap();
                }
            }
            EditorAction::MoveDown => {
                if editor_file.row_pos < editor_file.lines.len() {
                    editor_file.row_pos += 1;
                    solock.queue(cursor::MoveDown(1)).unwrap();
                }
            }
            EditorAction::MoveUp => {
                if editor_file.row_pos > 0 {
                    editor_file.row_pos -= 1;
                    solock.queue(cursor::MoveUp(1)).unwrap();
                }
            }
            EditorAction::MoveRight => {
                if editor_file.col_pos < editor_file.lines[editor_file.row_pos].chars.len() {
                    editor_file.col_pos += 1;
                    solock.queue(cursor::MoveRight(1)).unwrap();
                }
            }
            EditorAction::InsertMode => {
                editor.mode = EditorMode::Insert;
                solock.queue(cursor::SetCursorStyle::SteadyBar).unwrap();
            }
            EditorAction::Save => {
                let lines = editor_file
                    .lines
                    .iter()
                    .map(|l| l.chars.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");

                fs::write("testfile.rs", lines).expect("Could not write file");
            }
            EditorAction::NormalMode => {
                solock.queue(cursor::SetCursorStyle::SteadyBlock).unwrap();
                editor.mode = EditorMode::Normal;
            }
            // Handle backspace
            EditorAction::Backspace => {
                if editor_file.col_pos > 0 {
                    editor_file.col_pos -= 1;
                    editor_file.lines[editor_file.row_pos]
                        .chars
                        .remove(editor_file.col_pos);
                    queue!(solock, cursor::MoveLeft(1), cursor::SavePosition)?;
                    write!(
                        solock,
                        "{}",
                        &editor_file.lines[editor_file.row_pos].chars[editor_file.col_pos..]
                    )?;
                    queue!(
                        solock,
                        terminal::Clear(terminal::ClearType::UntilNewLine),
                        cursor::RestorePosition
                    )?;
                } else if editor_file.row_pos > 0 {
                    let current_line = editor_file.lines.remove(editor_file.row_pos);
                    editor_file.row_pos -= 1;
                    editor_file.col_pos = editor_file.lines[editor_file.row_pos].chars.len();
                    editor_file.lines[editor_file.row_pos]
                        .chars
                        .push_str(&current_line.chars);
                    queue!(
                        solock,
                        cursor::SavePosition,
                        Print(format!(
                            "\x1b[{};{}r",
                            editor_file.row_pos + 2,
                            editor.num_rows
                        )),
                        terminal::ScrollUp(1),
                        Print("\x1b[r"),
                        cursor::RestorePosition,
                        cursor::MoveToPreviousLine(1),
                        Print(&editor_file.lines[editor_file.row_pos].chars),
                        cursor::MoveToColumn(editor_file.col_pos as u16)
                    )?;
                }
            }
            // Handle enter
            EditorAction::SplitLine => {
                // Split current line at cursor position
                let new_line = editor_file.lines[editor_file.row_pos]
                    .chars
                    .split_off(editor_file.col_pos);
                // Reprint current line and move to the next line
                queue!(
                    solock,
                    cursor::MoveToColumn(0),
                    terminal::Clear(terminal::ClearType::CurrentLine),
                    Print(&editor_file.lines[editor_file.row_pos].chars),
                    cursor::MoveToNextLine(1)
                )?;

                // Insert the next line
                editor_file.row_pos += 1;
                editor_file.col_pos = 0;

                editor_file
                    .lines
                    .insert(editor_file.row_pos, Line { chars: new_line });

                // Scroll down the terminal and print the new line
                queue!(
                    solock,
                    cursor::SavePosition,
                    Print(format!(
                        "\x1b[{};{}r",
                        editor_file.row_pos + 1,
                        editor.num_rows
                    )),
                    terminal::ScrollDown(1),
                    Print("\x1b[r"),
                    cursor::RestorePosition,
                    terminal::Clear(terminal::ClearType::CurrentLine),
                    Print(&editor_file.lines[editor_file.row_pos].chars),
                    cursor::MoveToColumn(0),
                )?;
            }
            EditorAction::InsertChar => {
                let c = input.chars().next().unwrap();
                editor_file.lines[editor_file.row_pos]
                    .chars
                    .insert(editor_file.col_pos, c);
                editor_file.col_pos += 1;
                write!(solock, "{}", { c }).unwrap();
                solock.queue(cursor::SavePosition).unwrap();
                write!(
                    solock,
                    "{}",
                    &editor_file.lines[editor_file.row_pos].chars[editor_file.col_pos..]
                )
                .unwrap();
                solock.queue(cursor::RestorePosition).unwrap();
            }
        };

        solock.flush().unwrap();
    }

    solock
        .queue(cursor::SetCursorStyle::DefaultUserShape)
        .unwrap();
    solock.flush().unwrap();
    disable_raw_mode().unwrap();

    Ok(())
}
