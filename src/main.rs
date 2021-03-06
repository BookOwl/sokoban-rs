extern crate sdl2;
extern crate fps_clock;
extern crate tinyfiledialogs;
#[macro_use]
extern crate lazy_static;

use std::cmp::PartialEq;
use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::env;

use sdl2::video::Window;
use sdl2::render::Canvas;
use sdl2::EventPump;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::image::INIT_PNG;
use sdl2::rect::{Rect, Point};
use sdl2::ttf::Sdl2TtfContext;
use sdl2::surface::Surface;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rwops::RWops;
use sdl2::image::ImageRWops;

use fps_clock::FpsClock;

macro_rules! rect {
    ($x:expr, $y:expr, $w:expr, $h:expr) => (Rect::new($x as i32, $y as i32, $w as u32, $h as u32))
}

const LEVELS: &'static str = include_str!("../levels.txt");
const SPRITESHEET_BYTES: &'static [u8] = include_bytes!("../resources/images/sokoban_spritesheet.png");
const FONT_BYTES: &'static [u8] = include_bytes!("../resources/font/swansea.ttf");
const WIDTH: u32 = 900;
const HEIGHT: u32 = 675;
const HALF_WIDTH: u32 = 450;
const HALF_HEIGHT: u32 = 337;
const TILE_WIDTH: u32 = 64;
const TILE_HEIGHT: u32 = 64;

lazy_static! {
    static ref BACKGROUND_COLOR: Color = Color::RGB(115, 139, 139);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Up,
    Down,
    Left,
    Right
}
impl Direction {
    fn as_offset(&self) -> (i32, i32) {
        match *self {
            Direction::Up => (0, -1),
            Direction::Right => (1, 0),
            Direction::Down => (0, 1),
            Direction::Left => (-1, 0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tile {
    Wall,
    Player,
    PlayerOnGoal,
    Star,
    StarOnGoal,
    Goal,
    OutsideFloor,
    InsideFloor
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
            ' ' => Ok(Tile::OutsideFloor),
            _ => Err(format!("'{}' is an invalid tile", c)),
        }
    }
    fn spritesheet_rect(&self) -> Rect {
        match *self {
            Tile::Wall => rect!(448, 64, 64, 64),
            Tile::PlayerOnGoal | Tile::Goal | Tile::StarOnGoal => rect!(60, 576, 20, 20),
            Tile::Star => rect!(384, 0, 64, 64),
            Tile::InsideFloor => rect!(192, 528, 64, 64),
            _ => panic!()
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
            Direction::Right => Position {x: self.x+1, ..*self},
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
    fn spritesheet_rect(&self) -> Rect {
        match self.direction {
            Direction::Down => rect!(554, 208, 42, 50),
            Direction::Left => rect!(543, 440, 45, 50),
            Direction::Right => rect!(512, 108, 45, 50),
            Direction::Up => rect!(554, 158, 42, 50),
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Goal {
    position: Position
}
impl Goal {
    fn new(position: Position) -> Goal {
        Goal { position }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GameState {
    player: Player,
    stars: Vec<Star>,
    goals: Vec<Goal>,
    steps: usize,
}
impl GameState {
    fn new(player: Player, stars: Vec<Star>, goals: Vec<Goal>,  steps: usize) -> GameState {
        GameState { player, stars, goals, steps }
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
        let mut goals = Vec::new();
        let mut player_pos = None;
        for (y, line) in lines.iter().enumerate() {
            let mut row = Vec::with_capacity(line.len());
            for (x, tile) in line.chars().enumerate() {
                let tile = Tile::from_char(tile)?;
                if tile == Tile::Player || tile == Tile::PlayerOnGoal {
                    // This tile is the starting position
                    player_pos = Some(Position::new(x, y));
                    row.push(Tile::OutsideFloor);
                } else if tile == Tile::Star || tile == Tile::StarOnGoal {
                    stars.push(Star::new(Position::new(x, y)));
                    row.push(Tile::OutsideFloor);
                } else if tile == Tile::PlayerOnGoal 
                          || tile == Tile::StarOnGoal 
                          || tile == Tile::Goal {
                    goals.push(Goal::new(Position::new(x, y)));
                    row.push(Tile::OutsideFloor);
                } else {
                    row.push(tile);
                }
            }
            if line.len() < longest_line_len {
                for _ in 0..(longest_line_len - line.len()) {
                    row.push(Tile::OutsideFloor);
                }
            }
            map.push(row);
        }
        let pos = player_pos.ok_or_else(|| "Invalid level: Level has no starting position")?;
        let start_state = GameState::new(Player::new(pos, Direction::Right),
                                         stars,
                                         goals,
                                         0);
        let height = map.len();
        floodfill(&mut map, Tile::OutsideFloor, Tile::InsideFloor, pos.x, pos.y);
        Ok(Level { map, width: longest_line_len, height, start_state})
    }
    fn is_wall(&self, x: i32, y: i32) -> bool {
        if y < 0 || y >= self.height as i32 || x < 0 || x > self.height as i32{
            false
        } else {
            self.map[y as usize][x as usize] == Tile::Wall
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Camera {
    x_offset: i32,
    y_offset: i32,
    max_x_pan: i32,
    max_y_pan: i32,
    speed: i32,
}
impl Camera {
    fn new(x_offset: i32, y_offset: i32, max_x_pan: i32, max_y_pan: i32, speed: i32) -> Camera {
        Camera {
            x_offset,
            y_offset,
            max_x_pan,
            max_y_pan,
            speed,
        }
    }
    fn move_up(&mut self) {
        if self.y_offset < self.max_y_pan {
            self.y_offset += self.speed;
        }
    }
    fn move_down(&mut self) {
        if self.y_offset > -self.max_y_pan {
            self.y_offset -= self.speed;
        }
    }
    fn move_right(&mut self) {
        if self.x_offset > -self.max_x_pan {
            self.x_offset -= self.speed;
        }
    }
    fn move_left(&mut self) {
        if self.x_offset < self.max_x_pan {
            self.x_offset += self.speed;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Game {
    level: Level,
    state: GameState,
    camera: Camera,
    camera_moving: bool,
    camera_direction: Direction,
}
impl Game {
    fn new(level: Level, state: GameState, camera: Camera) -> Game {
        Game { level, state, camera, camera_moving: false, camera_direction: Direction::Left}
    }
    fn from_level(level: Level) -> Game {
        let h = level.height;
        let w = level.width;
        Game::new(level.clone(), 
                  level.start_state, 
                  Camera::new(0, 
                              0, 
                              (HALF_HEIGHT as i32 - (h/2) as i32).abs() + TILE_HEIGHT as i32,
                              (HALF_WIDTH as i32 - (w/2) as i32).abs() + TILE_WIDTH as i32,
                              5))
    }
    fn step(&mut self, event: &Event) {
        if self.camera_moving {
            let d = self.camera_direction;
            self.move_camera(d);
        }
        match *event {
            Event::KeyUp{..} => self.camera_moving = false,
            // Move the player
            Event::KeyDown{keycode: Some(Keycode::Up), ..} => self.make_move(Direction::Up),
            Event::KeyDown{keycode: Some(Keycode::Down), ..} => self.make_move(Direction::Down),
            Event::KeyDown{keycode: Some(Keycode::Left), ..} => self.make_move(Direction::Left),
            Event::KeyDown{keycode: Some(Keycode::Right), ..} => self.make_move(Direction::Right),
            // Move the camera
            Event::KeyDown{keycode: Some(Keycode::W), ..} => self.move_camera(Direction::Up),
            Event::KeyDown{keycode: Some(Keycode::S), ..} => self.move_camera(Direction::Down),
            Event::KeyDown{keycode: Some(Keycode::A), ..} => self.move_camera(Direction::Left),
            Event::KeyDown{keycode: Some(Keycode::D), ..} => self.move_camera(Direction::Right),
            _ => ()
        }
    }
    fn move_camera(&mut self, dir: Direction) {
        self.camera_direction = dir;
        self.camera_moving = true;
        match dir {
            Direction::Up => self.camera.move_up(),
            Direction::Down => self.camera.move_down(),
            Direction::Left => self.camera.move_left(),
            Direction::Right => self.camera.move_right(),
        }
    }
    fn render_to_surface<'a>(&self, spritesheet_surf: &Surface<'a>) -> Surface<'static> {
        let level = &self.level;
        let state = &self.state;
        let map = &level.map;
        let surf = Surface::new((level.width * 64) as u32, 
                                    (level.height * 64) as u32, 
                                    PixelFormatEnum::ABGR1555 /* <- I have no clue if this is right or not */).unwrap();
        let mut canvas = surf.into_canvas().unwrap();
        let texture_creator = canvas.texture_creator();
        let spritesheet = texture_creator.create_texture_from_surface(spritesheet_surf).unwrap();
        canvas.set_draw_color(*BACKGROUND_COLOR);
        canvas.clear();
        for (y, row) in map.iter().enumerate() {
            for (x, tile) in row.iter().enumerate() {
                match *tile {
                    Tile::OutsideFloor => (),
                    Tile::InsideFloor | Tile::Wall => {
                        canvas.copy(&spritesheet, tile.spritesheet_rect(), rect!(x*64, y*64, 64, 64)).unwrap();
                    },
                    _ => ()
                }
            }
        }
        for goal in &state.goals {
            let (x, y) = (&goal.position.x, &goal.position.y);
            canvas.copy(&spritesheet, Tile::Goal.spritesheet_rect(), rect!(x*64+22, y*64+22, 20, 20)).unwrap();
        }
        for star in &state.stars {
            let (x, y) = (&star.position.x, &star.position.y);
            canvas.copy(&spritesheet, Tile::Star.spritesheet_rect(), rect!(x*64, y*64, 64, 64)).unwrap();
        }
        let player = state.player;
        let (player_x, player_y) = (player.position.x, player.position.y);
        let player_rect = player.spritesheet_rect();
        let w = player_rect.width();
        let h = player_rect.height();
        let r = Rect::from_center(Point::new((player_x*64+32) as i32, (player_y*64+32) as i32), w, h);
        canvas.copy(&spritesheet, player_rect, r).unwrap();
        canvas.into_surface()
    }
    fn make_move(&mut self, direction: Direction) -> () {
        self.state.player.direction = direction;
        let (x_off, y_off) = direction.as_offset();
        let (new_x, new_y) = (self.state.player.position.x as i32 + x_off, 
                              self.state.player.position.y as i32 + y_off);
        if !self.level.is_wall(new_x, new_y) {
            let star = Star::new(Position::new(new_x as usize, new_y as usize));
            if self.state.stars.contains(&star) {
                if !self.is_blocked(new_x + x_off, new_y + y_off) {
                    let ind = self.state.stars.iter().position(|&s| s == star).unwrap();
                    self.state.stars[ind] = self.state.stars[ind].move_in_direction(direction);
                } else {
                    return
                }
            }
            self.state.player = self.state.player.move_in_direction(direction);
        }
    }
    fn is_blocked(&self, x: i32, y: i32) -> bool {
        self.level.is_wall(x, y) || self.state.stars.contains(&Star::new(Position::new(x as usize, y as usize)))
    }
    fn solved(&self) -> bool {
        self.state.stars.iter().all(|s| self.state.goals.contains(&Goal::new(s.position)))
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

fn floodfill<T: PartialEq + Copy>(map: &mut Vec<Vec<T>>, old: T, new: T, x: usize, y: usize) {
    if map[y][x] == old {
        map[y][x] = new;
    }
    if x > 0 && map[y][x-1] == old {
        floodfill(map, old, new, x-1, y);
    }
    if x + 1 < map[y].len() && map[y][x+1] == old {
        floodfill(map, old, new, x+1, y);
    }
    if y > 0 && map[y-1][x] == old {
        floodfill(map, old, new, x, y-1)
    }
    if y + 1 < map.len() && map[y+1][x] == old {
        floodfill(map, old, new, x, y+1)
    }
}

fn init_sdl(app_name: &str, width: u32, height: u32) -> Result<(Canvas<Window>, EventPump, Sdl2TtfContext), String> {
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
    let ttf_context = sdl2::ttf::init().map_err(|e| format!("{}", e))?;
    Ok((canvas, event_pump, ttf_context))
}

fn main() {
    // Load all of the game resources and start the game
    let mut level_number: i32 = 0;
    let mut parsed_levels = load_levels(LEVELS).unwrap();
    let mut game = Game::from_level(parsed_levels[level_number as usize].clone());
    let (mut canvas, mut event_pump, ttf_context) = init_sdl("Sokoban", WIDTH, HEIGHT).unwrap();
    let spritesheet_rw = RWops::from_bytes(&SPRITESHEET_BYTES).unwrap();
    let spritesheet_surf = spritesheet_rw.load().unwrap();
    let ttf_rw = RWops::from_bytes(&FONT_BYTES).unwrap();
    let texture_creator = canvas.texture_creator();
    let font = ttf_context.load_font_from_rwops(ttf_rw, 32).unwrap();
    let ttf_rw = RWops::from_bytes(&FONT_BYTES).unwrap();
    let big_font = ttf_context.load_font_from_rwops(ttf_rw, 64).unwrap();
    let mut clock = FpsClock::new(30);
    'main: loop {
        for event in event_pump.poll_iter() {
            match event {
                // The game exits when the user quits or hits the escape key
                Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'main
                },
                // Move to the next level if the user pressed N
                Event::KeyDown { keycode: Some(Keycode::N), .. } => {
                    let len = parsed_levels.len() as i32;
                    level_number = (level_number + len + 1) % len;
                    game = Game::from_level(parsed_levels[level_number as usize].clone());
                },
                // Move to the previous level if the user pressed B
                Event::KeyDown { keycode: Some(Keycode::B), .. } => {
                    let len = parsed_levels.len() as i32;
                    level_number = (level_number + len - 1) % len;
                    game = Game::from_level(parsed_levels[level_number as usize].clone());
                },
                // Reset the level if the user pressed Backspace
                Event::KeyDown { keycode: Some(Keycode::Backspace), .. } => {
                    game = Game::from_level(parsed_levels[level_number as usize].clone());
                },
                // Load a new level file if the user pressed L
                Event::KeyDown { keycode: Some(Keycode::L), .. } => {
                    if let Some(path) = tinyfiledialogs::open_file_dialog(
                                            "Select a level file",
                                            env::current_dir().unwrap().to_str().unwrap(), 
                                            None) {
                        println!("loading {}", path);
                        let mut contents = String::new();
                        let mut f = File::open(path).unwrap();
                        f.read_to_string(&mut contents).unwrap();
                        if let Ok(levels) = load_levels(&contents) {
                            level_number = 0;
                            parsed_levels = levels;
                            game = Game::from_level(parsed_levels[level_number as usize].clone());
                        } else {
                            tinyfiledialogs::message_box_ok(
                                "Error!", 
                                "Could not load level file", 
                                tinyfiledialogs::MessageBoxIcon::Error
                            );
                        }
                    }
                },
                // Let the game object handle the event
                event => game.step(&event),
            }
        }
        // Render the new game state
        let level_surf = game.render_to_surface(&spritesheet_surf);
        let mut rect = level_surf.rect();
        rect.center_on(Point::new(HALF_WIDTH as i32 + game.camera.x_offset, HALF_HEIGHT as i32 + game.camera.y_offset));
        let level_texture = texture_creator.create_texture_from_surface(level_surf).unwrap();
        let text_texture = texture_creator.create_texture_from_surface(
                                font.render(&format!("Level {}", level_number+1))
                                    .blended(Color::RGB(0, 0, 0)).unwrap()
                            ).unwrap();
        canvas.set_draw_color(*BACKGROUND_COLOR);
        canvas.clear();
        canvas.copy(&level_texture, None, Some(rect)).expect("Render failed");
        canvas.copy(&text_texture, None, Some(rect!(20, 20, text_texture.query().width, text_texture.query().height))).unwrap();
        canvas.present();
        if game.solved() {
            let you_win_texture = texture_creator.create_texture_from_surface(
                                big_font.render("You solved it!")
                                    .blended(Color::RGB(0, 0, 0)).unwrap()
                            ).unwrap();
            let you_win_rect = Rect::from_center(Point::new(HALF_WIDTH as i32, (HALF_HEIGHT - you_win_texture.query().height) as i32), 
                                                you_win_texture.query().width, 
                                                you_win_texture.query().height);
            let hit_key_texture = texture_creator.create_texture_from_surface(
                                        font.render("Hit any key to move on")
                                            .blended(Color::RGB(0, 0, 0)).unwrap()
                                    ).unwrap();
            let hit_key_rect = Rect::from_center(Point::new(HALF_WIDTH as i32, (HALF_HEIGHT + you_win_texture.query().height) as i32), 
                                                hit_key_texture.query().width, 
                                                hit_key_texture.query().height);
            // I'm not quite sure why I need to clear() and reblit the level texture,
            // but if I don't the game shows the previous move before the user solved the level.
            canvas.clear();
            canvas.copy(&level_texture, None, Some(rect)).expect("Render failed");
            canvas.copy(&text_texture, None, Some(rect!(20, 20, text_texture.query().width, text_texture.query().height))).expect("Render failed");
            canvas.copy(&you_win_texture, None, Some(you_win_rect)).expect("Render failed");
            canvas.copy(&hit_key_texture, None, Some(hit_key_rect)).expect("Render failed");
            canvas.present();
            // Loop until the user presses a key to move on.
            'you_win: loop {
                for event in event_pump.poll_iter() {
                    match event {
                        Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                            break 'main
                        },
                        Event::KeyDown { .. } => {
                            break 'you_win
                        },
                        _ => ()
                    }
                }
                clock.tick()
            }
            // Move to the next level
            let len = parsed_levels.len() as i32;
            level_number = (level_number + len + 1) % len;
            game = Game::from_level(parsed_levels[level_number as usize].clone());
        }
        clock.tick();
    }
}
