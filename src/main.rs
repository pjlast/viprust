use crossterm::event::{read, Event, KeyCode, KeyEvent};
use crossterm::style::Print;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::{cursor, queue, terminal, QueueableCommand};
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
    InsertChar(char),
}

struct EditorFile {
    lines: Vec<Line>,
    row_pos: usize,
    col_pos: usize,
}

struct Editor {
    mode: EditorMode,
    file: EditorFile,
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

        self.file = editor_file;

        Ok(())
    }

    fn process_input(&self, event: Event) -> EditorAction {
        match self.mode {
            EditorMode::Normal => match event {
                Event::Key(keyevent) => match keyevent {
                    KeyEvent {
                        code: KeyCode::Up, ..
                    } => EditorAction::MoveUp,
                    KeyEvent {
                        code: KeyCode::Down,
                        ..
                    } => EditorAction::MoveDown,
                    _ => EditorAction::Quit,
                },
                _ => EditorAction::Quit,
            },
            EditorMode::Insert => match event {
                Event::Key(keyevent) => match keyevent {
                    KeyEvent {
                        code: KeyCode::Esc, ..
                    } => EditorAction::NormalMode,
                    KeyEvent {
                        code: KeyCode::Char(c),
                        ..
                    } => EditorAction::InsertChar(c),
                    _ => EditorAction::Quit,
                },
                _ => EditorAction::Quit,
            },
        }
    }
}

fn main() -> io::Result<()> {
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
        file: EditorFile {
            lines: Vec::new(),
            row_pos: 0,
            col_pos: 0,
        },
        num_rows: (window_size.1 as usize),
        num_cols: (window_size.0 as usize),
    };

    let mut solock = stdout().lock();
    if let Err(err) = editor.open_file(filename) {
        eprintln!("Error while opening \"{}\": {}", filename, err);
        return Err(err);
    };

    enable_raw_mode().unwrap();
    solock
        .queue(terminal::Clear(terminal::ClearType::All))
        .unwrap()
        .queue(cursor::MoveTo(0, 0))
        .unwrap()
        .queue(cursor::SetCursorStyle::SteadyBlock)
        .unwrap();

    let mut iter = editor.file.lines.iter().take(editor.num_rows).peekable();

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

        let action = &editor.process_input(event);

        match action {
            EditorAction::Quit => break,
            EditorAction::MoveLeft => {
                if editor.file.col_pos > 0 {
                    editor.file.col_pos -= 1;
                    solock.queue(cursor::MoveLeft(1)).unwrap();
                }
            }
            EditorAction::MoveDown => {
                if editor.file.row_pos < editor.file.lines.len() {
                    editor.file.row_pos += 1;
                    solock.queue(cursor::MoveDown(1)).unwrap();
                }
            }
            EditorAction::MoveUp => {
                if editor.file.row_pos > 0 {
                    editor.file.row_pos -= 1;
                    solock.queue(cursor::MoveUp(1)).unwrap();
                }
            }
            EditorAction::MoveRight => {
                if editor.file.col_pos < editor.file.lines[editor.file.row_pos].chars.len() {
                    editor.file.col_pos += 1;
                    solock.queue(cursor::MoveRight(1)).unwrap();
                }
            }
            EditorAction::InsertMode => {
                editor.mode = EditorMode::Insert;
                solock.queue(cursor::SetCursorStyle::SteadyBar).unwrap();
            }
            EditorAction::Save => {
                let lines = editor
                    .file
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
                if editor.file.col_pos > 0 {
                    editor.file.col_pos -= 1;
                    editor.file.lines[editor.file.row_pos]
                        .chars
                        .remove(editor.file.col_pos);
                    queue!(solock, cursor::MoveLeft(1), cursor::SavePosition)?;
                    write!(
                        solock,
                        "{}",
                        &editor.file.lines[editor.file.row_pos].chars[editor.file.col_pos..]
                    )?;
                    queue!(
                        solock,
                        terminal::Clear(terminal::ClearType::UntilNewLine),
                        cursor::RestorePosition
                    )?;
                } else if editor.file.row_pos > 0 {
                    let current_line = editor.file.lines.remove(editor.file.row_pos);
                    editor.file.row_pos -= 1;
                    editor.file.col_pos = editor.file.lines[editor.file.row_pos].chars.len();
                    editor.file.lines[editor.file.row_pos]
                        .chars
                        .push_str(&current_line.chars);
                    queue!(
                        solock,
                        cursor::SavePosition,
                        Print(format!(
                            "\x1b[{};{}r",
                            editor.file.row_pos + 2,
                            editor.num_rows
                        )),
                        terminal::ScrollUp(1),
                        Print("\x1b[r"),
                        cursor::RestorePosition,
                        cursor::MoveToPreviousLine(1),
                        Print(&editor.file.lines[editor.file.row_pos].chars),
                        cursor::MoveToColumn(editor.file.col_pos as u16)
                    )?;
                }
            }
            // Handle enter
            EditorAction::SplitLine => {
                // Split current line at cursor position
                let new_line = editor.file.lines[editor.file.row_pos]
                    .chars
                    .split_off(editor.file.col_pos);
                // Reprint current line and move to the next line
                queue!(
                    solock,
                    cursor::MoveToColumn(0),
                    terminal::Clear(terminal::ClearType::CurrentLine),
                    Print(&editor.file.lines[editor.file.row_pos].chars),
                    cursor::MoveToNextLine(1)
                )?;

                // Insert the next line
                editor.file.row_pos += 1;
                editor.file.col_pos = 0;

                editor
                    .file
                    .lines
                    .insert(editor.file.row_pos, Line { chars: new_line });

                // Scroll down the terminal and print the new line
                queue!(
                    solock,
                    cursor::SavePosition,
                    Print(format!(
                        "\x1b[{};{}r",
                        editor.file.row_pos + 1,
                        editor.num_rows
                    )),
                    terminal::ScrollDown(1),
                    Print("\x1b[r"),
                    cursor::RestorePosition,
                    terminal::Clear(terminal::ClearType::CurrentLine),
                    Print(&editor.file.lines[editor.file.row_pos].chars),
                    cursor::MoveToColumn(0),
                )?;
            }
            EditorAction::InsertChar(c) => {
                editor.file.lines[editor.file.row_pos]
                    .chars
                    .insert(editor.file.col_pos, *c);
                editor.file.col_pos += 1;
                write!(solock, "{}", { c }).unwrap();
                solock.queue(cursor::SavePosition).unwrap();
                write!(
                    solock,
                    "{}",
                    &editor.file.lines[editor.file.row_pos].chars[editor.file.col_pos..]
                )
                .unwrap();
                solock.queue(cursor::RestorePosition).unwrap();
            }
        };

        solock.flush().unwrap();
    }

    queue!(
        solock,
        cursor::SetCursorStyle::DefaultUserShape,
        terminal::Clear(terminal::ClearType::All),
    )
    .unwrap();
    solock.flush().unwrap();
    disable_raw_mode().unwrap();

    Ok(())
}
