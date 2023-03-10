use crossterm::event::{read, Event, KeyCode, KeyEvent};
use crossterm::style::Print;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::{cursor, queue, terminal, QueueableCommand};
use std::fs::File;
use std::io::{self, stdout, BufRead, StdoutLock, Write};
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
    NoOp,
}

struct EditorFile {
    lines: Vec<Line>,
    row_pos: usize,
    col_pos: usize,
    row_scroll_pos: usize,
    col_scroll_pos: usize,
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
            row_scroll_pos: 0,
            col_scroll_pos: 0,
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
                Event::Key(KeyEvent { code, .. }) => match code {
                    KeyCode::Up | KeyCode::Char('k') => EditorAction::MoveUp,
                    KeyCode::Down | KeyCode::Char('j') => EditorAction::MoveDown,
                    KeyCode::Right | KeyCode::Char('l') => EditorAction::MoveRight,
                    KeyCode::Left | KeyCode::Char('h') => EditorAction::MoveLeft,
                    KeyCode::Char('i') => EditorAction::InsertMode,
                    KeyCode::Char('q') => EditorAction::Quit,
                    KeyCode::Char('s') => EditorAction::Save,
                    _ => EditorAction::NoOp,
                },
                _ => EditorAction::NoOp,
            },
            EditorMode::Insert => match event {
                Event::Key(KeyEvent { code, .. }) => match code {
                    KeyCode::Esc => EditorAction::NormalMode,
                    KeyCode::Backspace => EditorAction::Backspace,
                    KeyCode::Enter => EditorAction::SplitLine,
                    KeyCode::Char(c) => EditorAction::InsertChar(c),
                    _ => EditorAction::NoOp,
                },
                _ => EditorAction::NoOp,
            },
        }
    }

    fn print_screen(&self, solock: &mut StdoutLock) {
        let mut iter = self
            .file
            .lines
            .iter()
            .skip(self.file.row_scroll_pos)
            .take(self.num_rows)
            .peekable();

        queue!(solock, cursor::MoveTo(0, 0)).unwrap();
        while let Some(line) = iter.next() {
            if iter.peek().is_some() {
                queue!(
                    solock,
                    Print(
                        &line
                            .chars
                            .chars()
                            .skip(self.file.col_scroll_pos)
                            .take(self.num_cols)
                            .collect::<String>()
                    ),
                    terminal::Clear(terminal::ClearType::UntilNewLine),
                    cursor::MoveToNextLine(1),
                )
                .unwrap();
            } else {
                queue!(
                    solock,
                    Print(
                        line.chars
                            .chars()
                            .skip(self.file.col_scroll_pos)
                            .take(self.num_cols)
                            .collect::<String>()
                    ),
                    terminal::Clear(terminal::ClearType::UntilNewLine),
                )
                .unwrap();
            }
        }
        queue!(
            solock,
            cursor::MoveTo(
                (self.file.col_pos - self.file.col_scroll_pos) as u16,
                (self.file.row_pos - self.file.row_scroll_pos) as u16,
            )
        )
        .unwrap();
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
            row_scroll_pos: 0,
            col_scroll_pos: 0,
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

    editor.print_screen(&mut solock);

    solock.queue(cursor::MoveTo(0, 0)).unwrap();

    solock.flush().unwrap();

    loop {
        let event = read().expect("Failed to read");
        match event {
            Event::Resize(cols, rows) => {
                editor.num_cols = cols as usize;
                editor.num_rows = rows as usize;
                if editor.num_cols < editor.file.col_pos - editor.file.col_scroll_pos {
                    editor.file.col_scroll_pos = editor.file.col_pos - editor.num_cols;
                }
                editor.print_screen(&mut solock);
                solock.flush()?;
                continue;
            }
            _ => {}
        };

        let action = &editor.process_input(event);

        match action {
            EditorAction::Quit => break,
            EditorAction::MoveLeft => {
                if editor.file.col_pos > 0 {
                    editor.file.col_pos -= 1;
                    solock.queue(cursor::MoveLeft(1)).unwrap();
                    if editor.file.col_pos < editor.file.col_scroll_pos {
                        editor.file.col_scroll_pos -= 1;
                        editor.print_screen(&mut solock);
                    }
                }
            }
            EditorAction::MoveDown => {
                if editor.file.row_pos + 1 < editor.file.lines.len() {
                    editor.file.row_pos += 1;
                    solock.queue(cursor::MoveDown(1)).unwrap();
                    if editor.file.row_pos >= editor.file.row_scroll_pos + editor.num_rows {
                        editor.file.row_scroll_pos += 1;
                        queue!(
                            solock,
                            cursor::SavePosition,
                            cursor::MoveToColumn(0),
                            terminal::ScrollUp(1),
                            Print(
                                &editor.file.lines[editor.file.row_pos]
                                    .chars
                                    .chars()
                                    .skip(editor.file.col_scroll_pos)
                                    .take(editor.num_cols)
                                    .collect::<String>()
                            ),
                            cursor::RestorePosition,
                        )?;
                    }
                    let row_len = editor.file.lines[editor.file.row_pos].chars.len();
                    if editor.file.col_scroll_pos > row_len {
                        editor.file.col_scroll_pos = row_len;
                        editor.print_screen(&mut solock);
                    }
                    if editor.file.col_pos > row_len {
                        editor.file.col_pos = row_len;
                        queue!(
                            solock,
                            cursor::MoveToColumn((row_len - editor.file.col_scroll_pos) as u16)
                        )?;
                    }
                }
            }
            EditorAction::MoveUp => {
                if editor.file.row_pos > 0 {
                    editor.file.row_pos -= 1;
                    solock.queue(cursor::MoveUp(1)).unwrap();
                }
                if editor.file.row_pos < editor.file.row_scroll_pos {
                    editor.file.row_scroll_pos -= 1;
                    queue!(
                        solock,
                        cursor::SavePosition,
                        cursor::MoveToColumn(0),
                        terminal::ScrollDown(1),
                        Print(
                            &editor.file.lines[editor.file.row_pos]
                                .chars
                                .chars()
                                .skip(editor.file.col_scroll_pos)
                                .take(editor.num_cols)
                                .collect::<String>()
                        ),
                        cursor::RestorePosition,
                    )?;
                }
                let row_len = editor.file.lines[editor.file.row_pos].chars.len();
                if editor.file.col_scroll_pos > row_len {
                    editor.file.col_scroll_pos = row_len;
                    editor.print_screen(&mut solock);
                }
                if editor.file.col_pos > row_len {
                    editor.file.col_pos = row_len;
                    queue!(
                        solock,
                        cursor::MoveToColumn((row_len - editor.file.col_scroll_pos) as u16)
                    )?;
                }
            }
            EditorAction::MoveRight => {
                if editor.file.col_pos < editor.file.lines[editor.file.row_pos].chars.len() {
                    editor.file.col_pos += 1;
                    solock.queue(cursor::MoveRight(1)).unwrap();
                    if editor.file.col_pos >= editor.num_cols + editor.file.col_scroll_pos {
                        editor.file.col_scroll_pos += 1;
                        editor.print_screen(&mut solock);
                    }
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
                    if editor.file.col_pos < editor.file.col_scroll_pos {
                        editor.file.col_scroll_pos -= 1;
                        editor.print_screen(&mut solock);
                    } else {
                        queue!(solock, cursor::MoveLeft(1), cursor::SavePosition)?;
                        write!(
                            solock,
                            "{}",
                            &editor.file.lines[editor.file.row_pos]
                                .chars
                                .chars()
                                .skip(editor.file.col_pos)
                                .take(
                                    editor.file.col_scroll_pos + editor.num_cols
                                        - editor.file.col_pos
                                )
                                .collect::<String>()
                        )?;
                        queue!(
                            solock,
                            terminal::Clear(terminal::ClearType::UntilNewLine),
                            cursor::RestorePosition
                        )?;
                    }
                } else if editor.file.row_pos > 0 {
                    queue!(solock, terminal::Clear(terminal::ClearType::CurrentLine))?;
                    let current_line = editor.file.lines.remove(editor.file.row_pos);
                    editor.file.row_pos -= 1;
                    editor.file.col_pos = editor.file.lines[editor.file.row_pos].chars.len();
                    editor.file.lines[editor.file.row_pos]
                        .chars
                        .push_str(&current_line.chars);
                    if editor.file.row_pos + 1 == editor.file.row_scroll_pos {
                        editor.file.row_scroll_pos -= 1;
                        queue!(
                            solock,
                            Print(
                                &editor.file.lines[editor.file.row_pos]
                                    .chars
                                    .chars()
                                    .skip(editor.file.col_scroll_pos)
                                    .take(editor.num_cols)
                                    .collect::<String>(),
                            ),
                            cursor::MoveTo(editor.file.col_pos as u16, 0),
                        )?;
                    } else if editor.file.row_pos + 2 - editor.file.row_scroll_pos
                        != editor.num_rows
                    {
                        queue!(
                            solock,
                            cursor::SavePosition,
                            Print(format!(
                                "\x1b[{};{}r",
                                editor.file.row_pos + 2 - editor.file.row_scroll_pos,
                                editor.num_rows
                            )),
                            terminal::ScrollUp(1),
                            Print("\x1b[r")
                        )?;
                    }
                    if editor.file.lines.len() > editor.num_rows + editor.file.row_scroll_pos
                        && editor.file.row_pos != editor.file.row_scroll_pos
                    {
                        queue!(
                            solock,
                            cursor::MoveTo(0, (editor.num_rows - 1) as u16),
                            Print(
                                &editor.file.lines
                                    [editor.num_rows - 1 + editor.file.row_scroll_pos]
                                    .chars
                                    .chars()
                                    .skip(editor.file.col_scroll_pos)
                                    .take(editor.num_cols)
                                    .collect::<String>()
                            ),
                            terminal::Clear(terminal::ClearType::UntilNewLine),
                        )?;
                    }
                    queue!(
                        solock,
                        cursor::RestorePosition,
                        cursor::MoveToPreviousLine(1),
                        Print(
                            &editor.file.lines[editor.file.row_pos]
                                .chars
                                .chars()
                                .skip(editor.file.col_scroll_pos)
                                .take(editor.num_cols)
                                .collect::<String>()
                        ),
                        cursor::MoveToColumn(editor.file.col_pos as u16)
                    )?;
                    if editor.file.col_pos > editor.num_cols - 1 + editor.file.col_scroll_pos {
                        editor.file.col_scroll_pos = editor.file.col_pos - editor.num_cols + 1;
                        editor.print_screen(&mut solock);
                    }
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
            EditorAction::NoOp => continue,
        };

        solock.flush().unwrap();
    }

    queue!(
        solock,
        cursor::SetCursorStyle::DefaultUserShape,
        cursor::MoveTo(0, 0),
        terminal::Clear(terminal::ClearType::All),
    )
    .unwrap();
    solock.flush().unwrap();
    disable_raw_mode().unwrap();

    Ok(())
}
