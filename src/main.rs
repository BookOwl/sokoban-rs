extern crate sdl2;
extern crate fps_clock;

use sdl2::video::Window;
use sdl2::render::Canvas;
use sdl2::EventPump;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::image::{LoadTexture, INIT_PNG};
use sdl2::rect::Rect;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Player {
    position: Position,
    direction: Direction,
}
impl Player {
    fn new(position: Position, direction: Direction) -> Player {
        Player {
            position,
            direction,
        }
    }
    fn move_in_direction(&self, direction: Direction) -> Player {
        Player::new(self.position.move_in_direction(direction), direction)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Star {
    position: Position
}
impl Star {
    fn new(position: Position) -> Star {
        Star { position }
    }
    fn move_in_direction(&self, direction: Direction) -> Star {
        Star::new(self.position.move_in_direction(direction))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GameState {
    player: Player,
    stars: Vec<Star>,
    steps: usize,
}
impl GameState {
    fn new(player: Player, stars: Vec<Star>, steps: usize) -> GameState {
        GameState { player, stars, steps }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Level {
    width: usize,
    height: usize,
    map: Vec<Vec<Tile>>,
    start_state: GameState,
}
impl Level {
    fn from_lines(lines: Vec<&str>) -> Result<Level, String> {
        let longest_line_len = lines.iter()
                                .map(|l| l.len())
                                .max()
                                .ok_or_else(|| "Invalid level: Level is empty")?;
        let mut map = Vec::with_capacity(lines.len());
        let mut stars = Vec::new();
        let mut player_pos = None;
        for (y, line) in lines.iter().enumerate() {
            let mut row = Vec::with_capacity(line.len());
            for (x, tile) in line.chars().enumerate() {
                let tile = Tile::from_char(tile)?;
                row.push(tile);
                if tile == Tile::Player || tile == Tile::PlayerOnGoal {
                    // This tile is the starting position
                    player_pos = Some(Position::new(x, y));
                } else if tile == Tile::Star || tile == Tile::StarOnGoal {
                    stars.push(Star::new(Position::new(x, y)));
                }
            }
            if line.len() < longest_line_len {
                for _ in 0..(longest_line_len - line.len()) {
                    row.push(Tile::Floor);
                }
            }
            map.push(row);
        }
        let start_state = GameState::new(Player::new(player_pos.ok_or_else(|| "Invalid level: Level has no starting position")?, Direction::Left),
                                         stars,
                                         0);
        let height = map.len();
        Ok(Level { map, width: longest_line_len, height, start_state})
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
    let _image_context = sdl2::image::init(INIT_PNG)?;
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

macro_rules! rect {
    ($x:expr, $y:expr, $w:expr, $h:expr) => (Rect::new($x as i32, $y as i32, $w as u32, $h as u32))
}

fn rect_at_center_of(pos: Position, w: u32, h: u32) -> Rect {
    let topleft_x = pos.x as u32 - w/2;
    let topleft_y = pos.y as u32 - h/2;
    rect!(topleft_x, topleft_y, w, h)
}

const LEVELS: &'static str = include_str!("../levels.txt");
const SPRITE_PATH: &'static str = "resources/images/sokoban_spritesheet.png";

fn main() {
    let (width, height) = (800, 600);
    let parsed_levels = load_levels(LEVELS).unwrap();
    let (mut canvas, mut event_pump) = init_sdl("Sokoban", width, height).unwrap();
    let texture_creator = canvas.texture_creator();
    let texture = texture_creator.load_texture(SPRITE_PATH).unwrap();
    let center_rect = rect_at_center_of(Position::new((width/2) as usize, (height/2) as usize), width/2, height/2);
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
        canvas.copy(&texture, None, Some(center_rect)).expect("Render failed");
        canvas.present();
        clock.tick();
    }
}
