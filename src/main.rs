extern crate sdl2;
extern crate fps_clock;

use sdl2::video::Window;
use sdl2::render::Canvas;
use sdl2::EventPump;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use fps_clock::FpsClock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Up,
    Down,
    Left,
    Right
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tile {
    Wall,
    Player,
    PlayerOnGoal,
    Star,
    StarOnGoal,
    Goal,
    Floor,
}
impl Tile {
    /// Character to tile mapping taken from http://sokobano.de/wiki/index.php?title=Level_format
    /// I renamed Box to Star to not conflict with the Box type.
    fn from_char(c: char) -> Result<Tile, String> {
        match c {
            '#' => Ok(Tile::Wall),
            '@' => Ok(Tile::Player),
            '+' => Ok(Tile::PlayerOnGoal),
            '$' => Ok(Tile::Star),
            '*' => Ok(Tile::StarOnGoal),
            '.' => Ok(Tile::Goal),
            ' ' => Ok(Tile::Floor),
            _ => Err(format!("'{}' is an invalid tile", c)),
        }
    }
    fn to_char(&self) -> char {
        use Tile::*;
        match *self {
            Wall => '#',
            Player => '@',
            PlayerOnGoal => '+',
            Star => '$',
            StarOnGoal => '*',
            Goal => '.',
            Floor => ' ',
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Position {
    x: usize,
    y: usize,
}
impl Position {
    fn new(x: usize, y: usize) -> Position {
        Position {x, y}
    }
    fn move_in_direction(&self, dir: Direction) -> Position {
        match dir {
            Direction::Down => Position {y: self.y+1, ..*self},
            Direction::Up => Position {y: self.y-1, ..*self},
            Direction::Left => Position {x: self.x-1, ..*self},
            Direction::Right => Position {x: self.x-1, ..*self},
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Level {
    map: Vec<Vec<Tile>>,
    player_pos: Position,
}
impl Level {
    fn from_lines(lines: Vec<&str>) -> Result<Level, String> {
        let longest_line_len = lines.iter().map(|l| l.len()).max().unwrap();
        let mut map = Vec::with_capacity(lines.len());
        let mut player_pos = None;
        for (y, line) in lines.iter().enumerate() {
            let mut row = Vec::with_capacity(line.len());
            for (x, tile) in line.chars().enumerate() {
                row.push(Tile::from_char(tile)?);
                if tile == '@' || tile == '+' {
                    // This tile is the starting position
                    player_pos = Some(Position::new(x, y));
                }
            }
            if line.len() < longest_line_len {
                for _ in 0..(longest_line_len - line.len()) {
                    row.push(Tile::Floor);
                }
            }
            map.push(row);
        }
        Ok(Level { map: map, player_pos: player_pos.unwrap()})
    }
    fn as_string(&self) -> String {
        let mut map = String::new();
        for line in &self.map {
            for tile in line {
                map.push(tile.to_char());
            }
            map.push('\n');
        }
        map
    }
}

fn load_levels(levels: &str) -> Result<Vec<Level>, String> {
    let mut parsed_levels = Vec::new();
    let mut map_lines = Vec::new();
    for line in levels.lines() {
        let line = line.trim_right();
        let line = if let Some(i) = line.find(';') {
            &line[0..i]
        } else {
            line
        };
        if !line.is_empty() {
            map_lines.push(line);
        } else if line.is_empty() && !map_lines.is_empty() {
            parsed_levels.push(Level::from_lines(map_lines)?);
            map_lines = Vec::new();
        }
    }
    Ok(parsed_levels)
}


fn init_sdl(app_name: &str, width: u32, height: u32) -> Result<(Canvas<Window>, EventPump), String> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem.window(app_name, width, height)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| format!("{}", e))?;

    let canvas = window.into_canvas().build().map_err(|e| format!("{}", e))?;
    let event_pump = sdl_context.event_pump()?;
    Ok((canvas, event_pump))
}

const LEVELS: &'static str = include_str!("../levels.txt");

fn main() {
    let parsed_levels = load_levels(LEVELS).unwrap();
    for level in &parsed_levels[0..5] {
        println!("{}", level.as_string());
    }
    /*
    let (mut canvas, mut event_pump) = init_sdl("Sokoban", 800, 600).unwrap();
    let mut clock = FpsClock::new(60);
    'main: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'main
                },
                _ => {}
            }
        }
        canvas.set_draw_color(Color::RGB(29, 167, 226));
        canvas.clear();
        canvas.present();
        clock.tick();
    }*/
}
