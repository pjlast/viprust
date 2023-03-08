use crossterm::terminal::{disable_raw_mode, enable_raw_mode};123123123123123123123123123123123123123123123123
use crossterm::{cursor, terminal, QueueableCommand};
use std::fs::File;
use std::io::{self, stdin, stdout, BufRead, Read, Write};aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
use std::{env, fs};

struct Line {
    chars: String,
}

enum EditorMode {
    Normal,
    Insert,
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
        file: None,
        num_rows: (window_size.1 as usize),
        num_cols: (window_size.0 as usize),
    };

    let mut solock = stdout().lock();
    let mut silockbytes = stdin().lock().bytes();
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
        let c = match silockbytes.next() {
            Some(Ok(c)) => c as char,
            _ => continue,
        };

        match editor.mode {
            EditorMode::Normal => match c {
                '\x1b' => break,
                'h' => {
                    if editor_file.col_pos > 0 {
                        editor_file.col_pos -= 1;
                        solock.queue(cursor::MoveLeft(1)).unwrap();
                    }
                }
                'j' => {
                    if editor_file.row_pos < editor_file.lines.len() {
                        editor_file.row_pos += 1;
                        solock.queue(cursor::MoveDown(1)).unwrap();
                    }
                }
                'k' => {
                    if editor_file.row_pos > 0 {
                        editor_file.row_pos -= 1;
                        solock.queue(cursor::MoveUp(1)).unwrap();
                    }
                }
                'l' => {
                    if editor_file.col_pos < editor_file.lines[editor_file.row_pos].chars.len() {
                        editor_file.col_pos += 1;
                        solock.queue(cursor::MoveRight(1)).unwrap();
                    }
                }
                'i' => {
                    editor.mode = EditorMode::Insert;
                    solock.queue(cursor::SetCursorStyle::SteadyBar).unwrap();
                }
                's' => {
                    let lines = editor_file
                        .lines
                        .iter()
                        .map(|l| l.chars.as_str())
                        .collect::<Vec<_>>()
                        .join("\n");

                    fs::write("testfile.rs", lines).expect("Could not write file");
                }
                _ => continue,
            },
            EditorMode::Insert => match c {
                '\x1b' => {
                    solock.queue(cursor::SetCursorStyle::SteadyBlock).unwrap();
                    editor.mode = EditorMode::Normal;
                }
                '\x7F' => {
                    if editor_file.col_pos > 0 {
                        editor_file.col_pos -= 1;
                        editor_file.lines[editor_file.row_pos]
                            .chars
                            .remove(editor_file.col_pos);
                        solock.queue(cursor::MoveLeft(1)).unwrap();
                        solock.queue(cursor::SavePosition).unwrap();
                        write!(
                            solock,
                            "{}",
                            &editor_file.lines[editor_file.row_pos].chars[editor_file.col_pos..]
                        )
                        .unwrap();
                        solock
                            .queue(terminal::Clear(terminal::ClearType::UntilNewLine))
                            .unwrap();
                        solock.queue(cursor::RestorePosition).unwrap();
                    }
                }
                _ => {
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
            },
        }

        solock.flush().unwrap();
    }

    solock
        .queue(cursor::SetCursorStyle::DefaultUserShape)
        .unwrap();
    solock.flush().unwrap();
    disable_raw_mode().unwrap();

    Ok(())
}