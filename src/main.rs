use std::collections::HashMap;

use ansi_escapes::{CursorDown, CursorTo, EraseScreen};
use ansi_term::{ANSIString, Color};

enum Cell {
    Ray { color: u8 },
    MirrorLDRU, // "/"
    MirrorLURD, // "\"
    Terminator,
}

impl Cell {
    fn draw<'a>(self: Self) -> ANSIString<'a> {
        match self {
            Cell::Ray { color } => Color::White.on(Color::Fixed(color)).paint(" "),
            Cell::MirrorLDRU => Color::Black.on(Color::White).paint("/"),
            Cell::MirrorLURD => Color::Black.on(Color::White).paint("\\"),
            Cell::Terminator => Color::Red.on(Color::White).paint("x"),
        }
    }
}

struct Screen { cells: HashMap<(u16, u16), Cell> }

impl Screen {
    fn draw(self: Self) {
        for ((x, y), c) in self.cells {
            print!("{}{}", CursorTo::AbsoluteXY(x, y), c.draw());
        }
    }
}

fn main() {
    let s = Screen { cells: {
        let mut m = HashMap::new();
        m.insert((1, 0), Cell::Ray { color: 1 });
        m.insert((1, 1), Cell::MirrorLURD);
        m.insert((2, 1), Cell::Ray { color: 1 });
        m.insert((3, 1), Cell::Terminator);
        m.insert((3, 2), Cell::Ray { color: 2 });
        m.insert((0, 2), Cell::MirrorLDRU);
        m
    }};
    print!("{}", EraseScreen);
    s.draw();
    print!("{}", CursorDown(3));
    println!();
}
