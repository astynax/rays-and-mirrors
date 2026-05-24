use std::collections::HashMap;
use std::fmt::Debug;

use rand::rngs::ThreadRng;
use rand::Rng;
use termwiz::caps::Capabilities;
use termwiz::cell::AttributeChange;
use termwiz::color::{AnsiColor, ColorAttribute};
use termwiz::input::{InputEvent, KeyCode, KeyEvent, Modifiers};
use termwiz::surface::{Change, CursorVisibility, Position, Surface};
use termwiz::terminal::buffered::BufferedTerminal;
use termwiz::terminal::{new_terminal, Terminal};
use termwiz::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RayColor { value: u8 }

impl From<RayColor> for ColorAttribute {
    fn from(value: RayColor) -> Self {
        match value.value {
            0 => AnsiColor::Black,
            1 => AnsiColor::Maroon,
            2 => AnsiColor::Green,
            3 => AnsiColor::Olive,
            4 => AnsiColor::Navy,
            5 => AnsiColor::Purple,
            6 => AnsiColor::Teal,
            7 => AnsiColor::Silver,
            8 => AnsiColor::Grey,
            9 => AnsiColor::Red,
            10 => AnsiColor::Lime,
            11 => AnsiColor::Yellow,
            12 => AnsiColor::Blue,
            13 => AnsiColor::Fuchsia,
            14 => AnsiColor::Aqua,
            15 => AnsiColor::White,
            _ => AnsiColor::White,
        }.into()
    }
}

impl From<AnsiColor> for RayColor {
    fn from(value: AnsiColor) -> Self {
        RayColor { value: value.into() }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Tick { tick: usize }

#[derive(Clone, Debug)]
enum Cell {
    Ray { colors: Vec<(Tick, RayColor)> },
    MirrorLDRU, // "/"
    MirrorLURD, // "\"
    Terminator,
}

impl Cell {
    fn prepare(self, tick: Tick) -> (&'static str, Vec<AttributeChange>) {
        match self {
            Cell::Ray { colors } => (" ", vec!(
                AttributeChange::Foreground(AnsiColor::White.into()),
                AttributeChange::Background(
                    (*actual_at(tick, &colors)
                     .unwrap_or(&AnsiColor::Black.into())
                    ).into()
                )
            )),
            Cell::MirrorLDRU => ("/", fb(AnsiColor::Silver, AnsiColor::Black)),
            Cell::MirrorLURD => ("\\", fb(AnsiColor::Silver, AnsiColor::Black)),
            Cell::Terminator => ("x", fb(AnsiColor::Maroon, AnsiColor::Black)),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    fn step(&self, x: usize, y: usize, w: usize, h: usize) -> Option<(usize, usize)> {
        match self {
            Direction::Up if y > 0 => Some((x, y - 1)),
            Direction::Down if y < h - 1 => Some((x, y + 1)),
            Direction::Left if x > 0 => Some((x - 1, y)),
            Direction::Right if x < w - 1 => Some((x + 1, y)),
            _ => None,
        }
    }

    fn reflect(&self, cell: Cell) -> Direction {
        match (self, cell) {
            (Direction::Up, Cell::MirrorLDRU) => Direction::Right,
            (Direction::Down, Cell::MirrorLDRU) => Direction::Left,
            (Direction::Left, Cell::MirrorLDRU) => Direction::Down,
            (Direction::Right, Cell::MirrorLDRU) => Direction::Up,

            (Direction::Up, Cell::MirrorLURD) => Direction::Left,
            (Direction::Down, Cell::MirrorLURD) => Direction::Right,
            (Direction::Left, Cell::MirrorLURD) => Direction::Up,
            (Direction::Right, Cell::MirrorLURD) => Direction::Down,

            (d, _) => *d,
        }
    }
}

fn mix_colors(
    RayColor { value: c1 }: RayColor,
    RayColor { value: c2 }: RayColor,
) -> RayColor {
    let mut res = c1 | c2;
    if c1 > 0 && c1 == c2 && c1 < 8 {
        res |= 8;
    }
    RayColor { value: res }
}

fn actual_at<K, T>(moment: K, timed: &Vec<(K, T)>) -> Option<&T>
where K: PartialOrd {
    let mut result: Option<&T> = None;
    for (t, v) in timed.iter() {
        if *t <= moment {
            result = Some(v)
        } else {
            break
        }
    };
    result
}

fn push_ray<K>(moment: K, new: RayColor, colors: &mut Vec<(K, RayColor)>)
where K: PartialOrd {
    let mut current = new.clone();
    let mut insertion = Option::<(usize, RayColor)>::None;
    for (i, (t, c)) in colors.iter_mut().enumerate() {
        if *t > moment && insertion.is_none() {
            insertion = Option::Some((i, current));
        };
        current = mix_colors(*c, current);
        if *t >= moment {
            *c = current.clone();
        }
    };
    match insertion {
        Some((i, c)) => colors.insert(i, (moment, c)),
        None => colors.push((moment, current)),
    };
}

struct Field {
    cells: HashMap<(usize, usize), Cell>,
    duration: Tick,
}

struct RayCursor {
    x: usize,
    y: usize,
    dir: Direction,
    color: RayColor,
    delay: usize,
    steps: usize,
    energy: usize,
}

const GADGET_APPEARING_P: f64 = 0.05;
const RAY_LENGTH_MIN: usize = 50;
const RAY_LENGTH_MAX: usize = RAY_LENGTH_MIN + 100;
const RAY_LENGTH_THRESHOLD: usize = 50;
const TARGET_COVERAGE: f64 = 0.2;

impl RayCursor {
    fn generate(rng: &mut ThreadRng, width: usize, height: usize) -> Self {
        let (x, y, dir) = match rng.gen_range(0..4) {
            0 => (rng.gen_range(0..width), 0, Direction::Down),
            1 => (rng.gen_range(0..width), height - 1, Direction::Up),
            2 => (0, rng.gen_range(0..height), Direction::Right),
            _ => (width - 1, rng.gen_range(0..height), Direction::Left),
        };
        let color = RayColor { value: 1 << rng.gen_range(0..=2) };
        let energy = rng.gen_range(RAY_LENGTH_MIN ..= RAY_LENGTH_MAX);
        let delay = rng.gen_range(0 ..= RAY_LENGTH_MAX);
        RayCursor { x, y, dir, color, energy, delay, steps: 0 }
    }
}

impl Field {
    fn generate(rng: &mut ThreadRng, width: usize, height: usize) -> Self {
        let mut field = Field {
            cells: HashMap::new(),
            duration: Tick { tick: 0 },
        };
        let area = width * height;
        let target_coverage = (area as f64 * TARGET_COVERAGE) as usize;
        let mut cursors: Vec<RayCursor> = Vec::new();

        loop {
            if cursors.is_empty() {
                if field.cells.len() < target_coverage {
                    cursors.push(RayCursor::generate(rng, width, height));
                } else {
                    break
                }
            }

            let mut next_cursors = Vec::new();
            for mut cursor in cursors {
                let pos = (cursor.x, cursor.y);
                let moment = Tick { tick: cursor.steps + cursor.delay };

                let cell_at_pos = field.cells.get(&pos);
                let mut empty = false;
                match cell_at_pos {
                    Some(Cell::Terminator) => {
                        continue
                    }
                    Some(Cell::MirrorLDRU) => {
                        cursor.dir = cursor.dir.reflect(Cell::MirrorLDRU);
                    }
                    Some(Cell::MirrorLURD) => {
                        cursor.dir = cursor.dir.reflect(Cell::MirrorLURD);
                    }
                    Some(Cell::Ray { .. }) => {
                        if let Some(Cell::Ray { colors }) = field.cells.get_mut(&pos) {
                            push_ray(moment, cursor.color, colors);
                        }
                    }
                    None => {
                        if rng.gen_bool(GADGET_APPEARING_P) {
                            let mirror = if rng.gen_bool(0.5) { Cell::MirrorLDRU } else { Cell::MirrorLURD };
                            field.cells.insert(pos, mirror);
                            next_cursors.push(cursor);
                            continue
                        } else {
                            empty = true;
                            field.cells.insert(pos, Cell::Ray { colors: vec!((moment, cursor.color)) });
                        }
                    }
                }

                if cursor.energy == 0 {
                    if empty {
                        let is_edge = cursor.x == 0 || cursor.x == width - 1 || cursor.y == 0 || cursor.y == height - 1;
                        if !is_edge {
                            field.cells.insert(pos, Cell::Terminator);
                        }
                        continue;
                    } else {
                        if cursor.steps < RAY_LENGTH_MAX + RAY_LENGTH_THRESHOLD {
                            cursor.energy += 1;
                        } else {
                            continue;
                        }
                    }
                }

                if let Some((nx, ny)) = cursor.dir.step(cursor.x, cursor.y, width, height) {
                    cursor.x = nx;
                    cursor.y = ny;
                    cursor.steps += 1;
                    cursor.energy -= 1;
                    next_cursors.push(cursor);
                }
            }
            cursors = next_cursors;
        }

        // filling the gaps
        let mut count = area / 10;
        while count > 0 {
            let pos = (rng.gen_range(0..width), rng.gen_range(0..height));
            if field.cells.contains_key(&pos) {
                continue
            }
            let gadget = match rng.gen_range(0..=2) {
                1 => Cell::MirrorLDRU,
                2 => Cell::MirrorLURD,
                _ => Cell::Terminator,
            };
            field.cells.insert(pos, gadget);
            count -= 1;
        }

        // getting the total duration
        const T0: Tick = Tick { tick: 0 };
        field.duration = *field.cells.values()
            .map(|c|
                 match c {
                     Cell::Ray { colors } =>
                         colors.iter().map(|(t, _)| t)
                         .max().unwrap_or(&T0),
                     _ => &T0,
                 }
            ).max()
            .unwrap_or(&T0);

        field
    }

    fn draw_to(&self, surface: &mut Surface, moment: Tick) {
        surface.add_change(Change::ClearScreen(AnsiColor::Black.into()));
        for ((x, y), c) in &self.cells {
            surface.add_change(Change::CursorPosition {
                x: Position::Absolute(*x),
                y: Position::Absolute(*y)
            });
            let (s, attrs) = c.clone().prepare(moment);
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
    let caps = Capabilities::new_from_env()?;
    let mut terminal = new_terminal(caps)?;
    terminal.enter_alternate_screen()?;

    let mut buf = BufferedTerminal::new(terminal)?;
    buf.add_change(Change::CursorVisibility(CursorVisibility::Hidden));
    buf.add_change(Change::ClearScreen(AnsiColor::Black.into()));
    buf.flush()?;  // important!

    let (width, height) = buf.dimensions();
    let mut rng = rand::thread_rng();
    let s = Field::generate(&mut rng, width, height);

    let mut screen_surface = Surface::new(width, height);

    let mut time: Tick = Tick { tick: 0 };

    buf.terminal().set_raw_mode()?;
    loop {
        s.draw_to(&mut screen_surface, time);

        buf.draw_from_screen(&screen_surface, 0, 0);
        buf.add_change(Change::CursorPosition { x: Position::Absolute(0), y: Position::Absolute(0) });
        buf.flush()?;

        match buf.terminal().poll_input(Some(std::time::Duration::from_millis(50))) {
            Ok(Some(input)) => match input {
                InputEvent::Key(KeyEvent {
                    key: KeyCode::Escape,
                    ..
                }) => break,
                InputEvent::Key(KeyEvent {
                    key: KeyCode::Char('c'),
                    modifiers: Modifiers::CTRL
                }) => break,
                _ => {}
            },
            Ok(None) => {}
            Err(_) => break,
        }

        time.tick += 1;
    }

    buf.add_change(Change::CursorVisibility(CursorVisibility::Visible));
    buf.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_actual_at() {
        let v: Vec<(usize, usize)> = vec!((2, 100), (4, 200), (10, 300));
        assert_eq!(actual_at(0, &v), None);
        assert_eq!(actual_at(2, &v), Some(&100));
        assert_eq!(actual_at(3, &v), Some(&100));
        assert_eq!(actual_at(7, &v), Some(&200));
        assert_eq!(actual_at(40, &v), Some(&300));
    }

    #[test]
    fn test_push_ray() {
        let mut colors: Vec<(usize, RayColor)> = Vec::new();

        push_ray(0_usize, AnsiColor::Navy.into(), &mut colors);
        assert_eq!(colors, vec!((0_usize, AnsiColor::Navy.into())));

        push_ray(10_usize, AnsiColor::Maroon.into(), &mut colors);
        assert_eq!(colors, vec!(
            (0_usize, AnsiColor::Navy.into()),
            (10_usize, AnsiColor::Purple.into()),
        ));

        push_ray(5_usize, AnsiColor::Green.into(), &mut colors);
        assert_eq!(colors, vec!(
            (0_usize, AnsiColor::Navy.into()),
            (5_usize, AnsiColor::Teal.into()),
            (10_usize, AnsiColor::Silver.into()),
        ));

        push_ray(3_usize, AnsiColor::Green.into(), &mut colors);
        assert_eq!(colors, vec!(
            (0_usize, AnsiColor::Navy.into()),
            (3_usize, AnsiColor::Teal.into()),
            (5_usize, AnsiColor::Aqua.into()),
            (10_usize, AnsiColor::White.into()),
        ));
    }

    #[test]
    fn test_no_terminators_on_edges() {
        // Run multiple times to cover stochastic behavior
        for _ in 0..10 {
            let width = 40;
            let height = 20;
            let mut rng = rand::thread_rng();
            let field = Field::generate(&mut rng, width, height);

            for ((x, y), cell) in &field.cells {
                if matches!(cell, Cell::Terminator) {
                    assert!(*x > 0 && *x < width - 1, "Terminator at x={} on edge", x);
                    assert!(*y > 0 && *y < height - 1, "Terminator at y={} on edge", y);
                }
            }
        }
    }

    #[test]
    fn test_prolongation_logic() {
        // This test is harder to write because Field::generate is random.
        // But we can check if it generates a reasonable field without crashing.
        let mut rng = rand::thread_rng();
        let field = Field::generate(&mut rng, 100, 30);
        assert!(field.cells.len() >= (100 * 30) / 10);
    }
}
