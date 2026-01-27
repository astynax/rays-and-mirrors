use std::collections::HashMap;

use termwiz::caps::Capabilities;
use termwiz::cell::AttributeChange;
use termwiz::color::{AnsiColor, ColorAttribute};
use termwiz::input::{InputEvent, KeyCode, KeyCodeEncodeModes, KeyEvent, Modifiers};
use termwiz::surface::{Change, CursorVisibility, Position, Surface};
use termwiz::terminal::buffered::BufferedTerminal;
use termwiz::terminal::{new_terminal, Terminal};
use termwiz::Error;

enum Cell {
    Ray { color: u8 },
    MirrorLDRU, // "/"
    MirrorLURD, // "\"
    Terminator,
}

impl Cell {
    fn prepare<'a>(self: Self) -> (&'a str, Vec<AttributeChange>) {
        match self {
            Cell::Ray { color } => (" ", vec!(
                AttributeChange::Foreground(AnsiColor::White.into()),
                AttributeChange::Background(match color {
                    1 => AnsiColor::Red,
                    2 => AnsiColor::Lime,
                    4 => AnsiColor::Blue,
                    _ => AnsiColor::White,
                }.into())
            )),
            Cell::MirrorLDRU => ("/", fb(AnsiColor::Black, AnsiColor::White)),
            Cell::MirrorLURD => ("\\", fb(AnsiColor::Black, AnsiColor::White)),
            Cell::Terminator => ("x", fb(AnsiColor::White, AnsiColor::Maroon)),
        }
    }
}

struct Field { cells: HashMap<(usize, usize), Cell> }

impl Field {
    fn draw_to(self: Self, surface: &mut Surface) {
        surface.add_change(Change::ClearScreen(AnsiColor::Black.into()));
        for ((x, y), c) in self.cells {
            surface.add_change(Change::CursorPosition {
                x: Position::Absolute(x),
                y: Position::Absolute(y)
            });
            let (s, attrs) = c.prepare();
            for ac in attrs.iter() {
                surface.add_change(Change::Attribute(ac.clone()));
            }
            surface.add_change(s);
            surface.add_change(AttributeChange::Foreground(ColorAttribute::Default.into()));
            surface.add_change(AttributeChange::Background(ColorAttribute::Default.into()));
        }
    }
}

#[inline]
fn fb(fg: AnsiColor, bg: AnsiColor) -> Vec<AttributeChange> {
    vec!(
        AttributeChange::Foreground(fg.into()),
        AttributeChange::Background(bg.into())
    )
}

fn main() -> Result<(), Error> {
    let mut s = Field { cells: {
        let mut m = HashMap::new();
        m.insert((1, 0), Cell::Ray { color: 1 });
        m.insert((1, 1), Cell::MirrorLURD);
        m.insert((2, 1), Cell::Ray { color: 1 });
        m.insert((3, 1), Cell::Terminator);
        m.insert((3, 2), Cell::Ray { color: 2 });
        m.insert((0, 2), Cell::MirrorLDRU);
        m
    }};

    let caps = Capabilities::new_from_env()?;
    let terminal = new_terminal(caps)?;

    let mut buf = BufferedTerminal::new(terminal)?;
    buf.add_change(Change::CursorVisibility(CursorVisibility::Hidden));

    let (width, height) = buf.dimensions();
    let mut screen_surface = Surface::new(width, height);

    s.cells.insert((width - 1, height - 1), Cell::Terminator);
    s.draw_to(&mut screen_surface);

    buf.add_change(Change::ClearScreen(AnsiColor::Blue.into()));
    buf.flush()?;  // important!
    buf.draw_from_screen(&screen_surface, 0, 0);
    buf.add_change(Change::CursorPosition { x: Position::Absolute(0), y: Position::Absolute(0) });
    buf.flush()?;

    buf.terminal().set_raw_mode()?;
    loop {
        match buf.terminal().poll_input(None) {
            Ok(Some(input)) => match input {
                InputEvent::Key(KeyEvent {
                    key: KeyCode::Escape,
                    ..
                }) => break,
                InputEvent::Key(KeyEvent {
                    key: KeyCode::Char('c'),
                    modifiers: Modifiers::CTRL
                }) => break,
                _ => {
                    print!("{:?}\r\n", input);
                }
            },
            Ok(None) => {}
            Err(e) => {
                print!("{:?}\r\n", e);
                break;
            }
        }
    }

    buf.add_change(Change::CursorVisibility(CursorVisibility::Visible));

    Ok(())
}
