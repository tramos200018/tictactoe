use crate::types::{Rect, Rgba, Vec2i};
use pixels::{Pixels, SurfaceTexture};
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use std::{fs::{self, File}, path::Path, rc::Rc};
use std::io::BufReader;
use std::io::prelude::*;
use std::time::Instant;
use std::{borrow::Borrow, os::macos::raw::stat, task::RawWakerVTable};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit::{dpi::LogicalSize, event};
use winit::{
    event::{Event, VirtualKeyCode},
    window::Window,
};
use serde::{Serialize, Deserialize};
use winit_input_helper::WinitInputHelper; // 0.7.2

// Whoa what's this?
// Mod without brackets looks for a nearby file.
mod screen;
// Then we can use as usual.  The screen module will have drawing utilities.
use screen::Screen;

mod resources;
use resources::Resources;

mod tiles;
use tiles::{Tile, Tilemap, Tileset};
// Lazy glob imports
//use collision::*;
// Texture has our image loading and processing stuff
mod texture;
use texture::Texture;
// Animation will define our animation datatypes and blending or whatever
mod animation;
use animation::Animation;
// Sprite will define our movable sprites
mod sprite;
// Lazy glob import, see the extension trait business later for why
use sprite::*;
// And we'll put our general purpose types like color and geometry here:
mod types;
use types::*;
mod collision;
use collision::{rect_touching, Mobile, Wall};
type Color = [u8; DEPTH];

const CLEAR_COL: Color = [32, 32, 64, 255];
const WALL_COL: Color = [200, 200, 200, 255];
const PLAYER_COL: Color = [255, 128, 128, 255];
const NEXT_COL: Color = [255, 0, 0, 255];
const ARROW_COL: Color = [0, 255, 0, 255];

const EMPTY: usize = 0;
const CIRCLE: usize = 1;
const CROSS: usize = 2;

struct Level {
    gamemap: Vec<Wall>,
    exit: collision::Rect,
    position: Vec2i,
}

// Now this main module is just for the run-loop and rules processing.
pub struct GameState {
    // What data do we need for this game?  Wall positions?
    // Colliders?  Sprites and stuff?
    player: usize,
    animations: Vec<Animation>,
    textures: Vec<Rc<Texture>>,
    sprites: Vec<Sprite>,
    //maps: Vec<Tilemap>,
    //scroll: Vec2i,
    levels: Vec<Level>,
    current_level: usize,
    mode: Mode,
    model: Vec<Vec<usize>>,
    mouse_down: bool,
}
// seconds per frame
const DT: f64 = 1.0 / 60.0;

const WIDTH: usize = 700;
const HEIGHT: usize = 550;
const DEPTH: usize = 4;

const GRID_X: usize = 195;
const GRID_Y: usize = 150;
const GRID_LENGTH: usize = 250;

const CROSS_SIZE: usize = 75;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    TitleScreen,
    GamePlay,
    EndGame,
}

fn main() {
    let mut rsrc = Resources::new();
    let startscreen_tex = rsrc.load_texture(Path::new("start.png"));
    let confetti = rsrc.load_texture(Path::new("confetti2.jpeg"));
    let confetti2 = rsrc.load_texture(Path::new("confetti1.jpeg"));
    
    
    

    let tex = Rc::new(Texture::with_file(Path::new("king.png")));
    let frame1 = Rect {
        x: 0,
        y: 16,
        w: 50,
        h: 50,
    };
    let frame2 = Rect {
        x: 16,
        y: 16,
        w: 50,
        h: 50,
    };
    let mut anim = Rc::new(Animation::new(vec![frame1, frame2]));

    let walls1: Vec<Wall> = vec![
        //top wall
        Wall {
            rect: collision::Rect {
                x: 0,
                y: 0,
                w: WIDTH as u16,
                h: 100,
            },
        },
        //left wall
        Wall {
            rect: collision::Rect {
                x: 0,
                y: 0,
                w: 150,
                h: HEIGHT as u16,
            },
        },
        //right wall
        Wall {
            rect: collision::Rect {
                x: WIDTH as i32 / 3 * 2,
                y: 0,
                w: WIDTH as u16 / 3,
                h: HEIGHT as u16,
            },
        },
        //bottom wall
        Wall {
            rect: collision::Rect {
                x: 0,
                y: HEIGHT as i32 - 16,
                w: WIDTH as u16,
                h: 16,
            },
        },
        //square wall
        Wall {
            rect: collision::Rect {
                x: WIDTH as i32 / 2,
                y: HEIGHT as i32 / 2,
                w: 150,
                h: 300,
            },
        },
    ];
    let walls4: Vec<Wall> = vec![
        //top wall
        Wall {
            rect: collision::Rect {
                x: 0,
                y: 0,
                w: WIDTH as u16,
                h: 100,
            },
        },
    ];

    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        WindowBuilder::new()
            .with_title("TicTactoe")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .with_resizable(false)
            .build(&event_loop)
            .unwrap()
    };
    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WIDTH as u32, HEIGHT as u32, surface_texture).unwrap()
    };

    let level = Level {
        gamemap: walls1,
        exit: collision::Rect {
            x: WIDTH as i32 / 2 + 50,
            y: 100,
            w: 68,
            h: 175,
        },
        position: Vec2i(170, 500),
    };

    let level4 = Level {
        gamemap: walls4,
        //need to correct exit
        exit: collision::Rect {
            x: 373,
            y: 50,
            w: 43,
            h: 10,
        },
        position: Vec2i(110, 463),
    };

    let mut state = GameState {
        // initial game state...
        player: CIRCLE,
        levels: vec![level, level4],
        current_level: 0,
        mode: Mode::TitleScreen,
        animations: vec![],
        sprites: vec![Sprite::new(&confetti, &anim, frame1, 0, Vec2i(0, 0)),Sprite::new(&confetti2, &anim, frame1, 0, Vec2i(0, 500))],
        textures: vec![confetti, confetti2],
        model: vec![
            vec![EMPTY, EMPTY, EMPTY],
            vec![EMPTY, EMPTY, EMPTY],
            vec![EMPTY, EMPTY, EMPTY],
        ],
        mouse_down: false,
    };

    // How many frames have we simulated?
    let mut frame_count: usize = 0;
    // How many unsimulated frames have we saved up?
    let mut available_time = 0.0;
    // Track beginning of play
    let start = Instant::now();
    // Track end of the last frame
    let mut since = Instant::now();
    /*let mut model: Vec<Vec<usize>> = vec![
        vec![EMPTY, EMPTY, EMPTY],
        vec![EMPTY, EMPTY, EMPTY],
        vec![EMPTY, EMPTY, EMPTY]
    ];*/

    event_loop.run(move |event, _, control_flow| {
        // Draw the current frame
        if let Event::RedrawRequested(_) = event {
            let fb = pixels.get_frame();

            collision::clear(fb, CLEAR_COL);

            match state.mode {
                Mode::TitleScreen => {
                    Screen::wrap(pixels.get_frame(), WIDTH, HEIGHT, DEPTH, Vec2i(0, 0)).bitblt(
                        &startscreen_tex,
                        Rect {
                            x: 0,
                            y: 0,
                            w: 700,
                            h: 550,
                        },
                        Vec2i(0, 0),
                    )
                }
                Mode::GamePlay => {
                    //Draw the grid
                    //collision::gameLayout(pixels.get_frame(), GRID_X, GRID_Y, GRID_LENGTH, WALL_COL);
                    collision::gameLayout(pixels.get_frame(), WIDTH, HEIGHT, GRID_LENGTH, WALL_COL);

                    //collision::line(pixels.get_frame(), (GRID_X  + (GRID_LENGTH/3) , 0), (GRID_X  + (GRID_LENGTH/3) , 300), WALL_COL   );
                    //Draw a cross
                    //collision::cross(pixels.get_frame(), 300, 350, 50, WALL_COL);

                    //TODO: for loop that goes through model and draws all the circles and crosses
                    for i in 0..3 {
                        for j in 0..3 {
                            if state.model[i][j] == CIRCLE {
                                let center_x = (i * WIDTH / 3 + WIDTH / 6) as f32;
                                let center_y = (j * HEIGHT / 3 + HEIGHT / 6) as f32;

                                collision::circle(pixels.get_frame(), center_x, center_y);
                            } else if state.model[i][j] == CROSS {
                                let cross_x = (i * WIDTH / 3 + 75);
                                let cross_y = (j * HEIGHT / 3 + 50);

                                collision::cross(
                                    pixels.get_frame(),
                                    cross_x,
                                    cross_y,
                                    CROSS_SIZE,
                                    WALL_COL,
                                );
                            }
                        }
                    }

                    window.request_redraw();
                    /*
                    if circle_x > 0.0 && circle_y > 0.0{
                        collision::draw(pixels.get_frame(), circle_x, circle_y);
                        window.request_redraw();

                    }*/

                    let mut screen =
                        Screen::wrap(pixels.get_frame(), WIDTH, HEIGHT, DEPTH, Vec2i(0, 0));
                }
                Mode::EndGame => {

                    let mut screen = Screen::wrap(fb, WIDTH, HEIGHT, DEPTH, Vec2i(0, 0));

                    for s in state.sprites.iter() {
                        screen.draw_sprite(s);
                    }
                }
            }

            // Flip buffers
            if pixels.render().is_err() {
                *control_flow = ControlFlow::Exit;
                return;
            }

            // Rendering has used up some time.
            // The renderer "produces" time...
            available_time += since.elapsed().as_secs_f64();
        }
        // Handle input events
        if input.update(event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }
            // Resize the window if needed
            if let Some(size) = input.window_resized() {
                pixels.resize(size.width, size.height);
            }
        }

        // And the simulation "consumes" it
        while available_time >= DT {
            // Eat up one frame worth of time
            available_time -= DT;

            update_game(&mut state, &input, frame_count, &pixels);

            // Increment the frame counter
            frame_count += 1;
        }

        // Request redraw
        window.request_redraw();
        // When did the last frame end?
        since = Instant::now();
    });
}

fn update_game(
    state: &mut GameState,
    input: &WinitInputHelper,
    frame: usize,
    pixels: &Pixels<Window>,
) {
    let mut level_index: usize = state.current_level;
    let mut input_x = 0.0;
    let mut input_y = 0.0;
    let mouse_held = input.mouse_held(0);
    let mouse_rel = state.mouse_down && !mouse_held;
    

    match state.mode {
        Mode::TitleScreen => {
            if input.key_held(VirtualKeyCode::Return) {
                state.mode = Mode::GamePlay;
            } else if input.key_held(VirtualKeyCode::L){
                loadGame(state);
                state.mode = Mode::GamePlay;
            }
        }
        Mode::GamePlay => {
            if input.key_held(VirtualKeyCode::S){
                saveGame(state);
            }


            // Player control goes here
            if mouse_rel{
                if let Some((mouse_x, mouse_y)) = input.mouse().and_then(|mp| pixels.window_pos_to_pixel(mp).ok()) {
                        input_x = (mouse_x / (WIDTH/3)) as f32;
                        //println!("{}", circle_x);
                        input_y = (mouse_y/ (HEIGHT/3)) as f32;
                        //println!("{}", circle_y);
                        if state.player == CIRCLE{
                            if state.model[input_x as usize][input_y as usize] == EMPTY{
                                state.model[input_x as usize][input_y as usize] = state.player;
                                state.player = CROSS;
                            }
                            
                        }
                        
                }
            }
            if state.player == CROSS{
                let mut number1: usize = thread_rng().gen_range(0, 3);
                let mut number2: usize = thread_rng().gen_range(0, 3);
                if state.model[number1][number2] == EMPTY{
                    state.model[number1][number2] = state.player;
                    state.player = CIRCLE;
                    
                }
                println!("{}, {}", number1, number2);

            }

            //multiplayer(taking turns)
            /*
            if input.mouse_released(0) == true{
                if let Some((mouse_x, mouse_y)) = input.mouse().and_then(|mp| pixels.window_pos_to_pixel(mp).ok()) {
                        input_x = (mouse_x / (WIDTH/3)) as f32;
                        //println!("{}", circle_x);
                        input_y = (mouse_y/ (HEIGHT/3)) as f32;
                        //println!("{}", circle_y);
                        if state.model[input_x as usize][input_y as usize] == EMPTY{
                            state.model[input_x as usize][input_y as usize] = state.player;
                            if state.player == CIRCLE{
                                state.player = CROSS
                            }
                            else if state.player == CROSS{
                                state.player = CIRCLE
                            }

                        }
                }

            }
            */

            
            if gameOverCircle(state) || gameOverCross(state) || tie(state){
                state.mode = Mode::EndGame;
            }
            
        }

        Mode::EndGame => {
            state.sprites[0].position.1 += 1;
            state.sprites[0].position.0 += 1;
            state.sprites[1].position.1 -= 1;
            state.sprites[1].position.0 += 1;
            if input.key_held(VirtualKeyCode::Return) {
                ResetGame(state);
                state.mode = Mode::GamePlay;
            }
        }
        
    }

    state.mouse_down = mouse_held;

    // Handle collisions: Apply restitution impulses.

    // Update game rules: What happens when the player touches things?
}
pub fn saveGame(state: &mut GameState) -> std::io::Result<()>{
    let serialized = serde_json::to_string(&state.model).unwrap();
    fs::write("saved.txt", serialized);

    let file = File::open("saved.txt")?;
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    buf_reader.read_to_string(&mut contents)?;
    println!("{}", contents);
    Ok(())
}
pub fn loadGame(state: &mut GameState) -> std::io::Result<()>{
    if Path::new("saved.txt").exists(){
        let file = File::open("saved.txt")?;
        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents)?;
        let deserialized: Vec<Vec<usize>> = serde_json::from_str(&contents).unwrap();
        state.model = deserialized;
    }
    //include a message that there was not saved gamestate
    Ok(())
    
}
pub fn gameOverCircle(state: &mut GameState) -> bool {
    //circle
    if (state.model[0][0] == CIRCLE && state.model[0][1] == CIRCLE && state.model[0][2] == CIRCLE) {
        return true;
    } else if (state.model[1][0] == CIRCLE
        && state.model[1][1] == CIRCLE
        && state.model[1][2] == CIRCLE)
    {
        return true;
    } else if (state.model[2][0] == CIRCLE
        && state.model[2][1] == CIRCLE
        && state.model[2][2] == CIRCLE)
    {
        return true;
    } else if (state.model[0][0] == CIRCLE
        && state.model[1][0] == CIRCLE
        && state.model[2][0] == CIRCLE)
    {
        return true;
    } else if (state.model[0][1] == CIRCLE
        && state.model[1][1] == CIRCLE
        && state.model[2][1] == CIRCLE)
    {
        return true;
    } else if (state.model[0][2] == CIRCLE
        && state.model[1][2] == CIRCLE
        && state.model[2][2] == CIRCLE)
    {
        return true;
    } else if (state.model[0][0] == CIRCLE
        && state.model[1][1] == CIRCLE
        && state.model[2][2] == CIRCLE)
    {
        return true;
    } else if (state.model[0][2] == CIRCLE
        && state.model[1][1] == CIRCLE
        && state.model[2][0] == CIRCLE)
    {
        return true;
    }
    return false;
}
pub fn gameOverCross(state: &mut GameState) -> bool {
    //circle
    if (state.model[0][0] == CROSS && state.model[0][1] == CROSS && state.model[0][2] == CROSS) {
        return true;
    } else if (state.model[1][0] == CROSS
        && state.model[1][1] == CROSS
        && state.model[1][2] == CROSS)
    {
        return true;
    } else if (state.model[2][0] == CROSS
        && state.model[2][1] == CROSS
        && state.model[2][2] == CROSS)
    {
        return true;
    } else if (state.model[0][0] == CROSS
        && state.model[1][0] == CROSS
        && state.model[2][0] == CROSS)
    {
        return true;
    } else if (state.model[0][1] == CROSS
        && state.model[1][1] == CROSS
        && state.model[2][1] == CROSS)
    {
        return true;
    } else if (state.model[0][2] == CROSS
        && state.model[1][2] == CROSS
        && state.model[2][2] == CROSS)
    {
        return true;
    } else if (state.model[0][0] == CROSS
        && state.model[1][1] == CROSS
        && state.model[2][2] == CROSS)
    {
        return true;
    } else if (state.model[0][2] == CROSS
        && state.model[1][1] == CROSS
        && state.model[2][0] == CROSS)
    {
        return true;
    }
    return false;
}
pub fn tie(state: &mut GameState) -> bool {
    let mut count = 0;
    for i in 0..3 {
        for j in 0..3 {
            if (state.model[i][j] != EMPTY){
                count = count + 1;
            }
        }
    }

    if (count == 9){
        return true;
    }
    return false;
    
    
}

pub fn ResetGame(state: &mut GameState) {
    for i in 0..3 {
        for j in 0..3 {
            state.model[i][j] = EMPTY;
        }
    }
    state.player = CIRCLE;
}
