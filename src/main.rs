use crate::types::{Rect, Rgba, Vec2i};
use pixels::{Pixels, SurfaceTexture};
use std::rc::Rc;
use std::time::Instant;
use std::{borrow::Borrow, os::macos::raw::stat, path::Path, task::RawWakerVTable};
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit::{dpi::LogicalSize, event};
use winit_input_helper::WinitInputHelper;

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

struct Level {
    gamemap: Vec<Wall>,
    exit: collision::Rect,
    position: Vec2i,
}

// Now this main module is just for the run-loop and rules processing.
struct GameState {
    // What data do we need for this game?  Wall positions?
    // Colliders?  Sprites and stuff?
    player: Mobile,
    animations: Vec<Animation>,
    textures: Vec<Rc<Texture>>,
    sprites: Vec<Sprite>,
    //maps: Vec<Tilemap>,
    //scroll: Vec2i,
    levels: Vec<Level>,
    current_level: usize,
    mode: Mode,
}
// seconds per frame
const DT: f64 = 1.0 / 60.0;

const WIDTH: usize = 700;
const HEIGHT: usize = 550;
const DEPTH: usize = 4;

const GRID_X: usize = 195;
const GRID_Y: usize = 150;
const GRID_LENGTH: usize = 250;




#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    TitleScreen,
    GamePlay,
    EndGame,
}

fn main() {
    let mut rsrc = Resources::new();
    let startscreen_tex = rsrc.load_texture(Path::new("start.png"));
    let endscreen_tex = rsrc.load_texture(Path::new("end.jpg"));

    let tex = Rc::new(Texture::with_file(Path::new("king.png")));
    let frame1 = Rect {
        x: 0,
        y: 16,
        w: 16,
        h: 16,
    };
    let frame2 = Rect {
        x: 16,
        y: 16,
        w: 16,
        h: 16,
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
        player: Mobile {
            rect: collision::Rect {
                x: 170,
                y: 500,
                w: 11,
                h: 11,
            },
            vx: 0,
            vy: 0,
        },
        levels: vec![level, level4],
        current_level: 0,
        mode: Mode::TitleScreen,
        animations: vec![],
        sprites: vec![Sprite::new(&tex, &anim, frame1, 0, Vec2i(170, 500))],
        textures: vec![tex],
    };


    // How many frames have we simulated?
    let mut frame_count: usize = 0;
    // How many unsimulated frames have we saved up?
    let mut available_time = 0.0;
    // Track beginning of play
    let start = Instant::now();
    // Track end of the last frame
    let mut since = Instant::now();
    let mut circle_x = -5.0;
    let mut circle_y = -5.0;
    event_loop.run(move |event, _, control_flow| {
        // Draw the current frame
        if let Event::RedrawRequested(_) = event {
            //let fb = pixels.get_frame();

            collision::clear(pixels.get_frame(), CLEAR_COL);

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
                    collision::gameLayout(pixels.get_frame(), GRID_X, GRID_Y, GRID_LENGTH, WALL_COL);

                    //Draw a cross
                    collision::cross(pixels.get_frame(), 300, 350, 50, WALL_COL);

                    if input.mouse_released(0) == true{
                        if let Some((x, y)) = input.mouse().and_then(|mp| pixels.window_pos_to_pixel(mp).ok())
                        {
                            circle_x = x as f32;
                            circle_y = y as f32;

                        }

                    }
                    if circle_x > 0.0 && circle_y > 0.0{
                        collision::draw(pixels.get_frame(), circle_x, circle_y);
                        window.request_redraw();

                    }






                    let mut screen = Screen::wrap(pixels.get_frame(), WIDTH, HEIGHT, DEPTH, Vec2i(0, 0));
                }
                Mode::EndGame => {
                    Screen::wrap(pixels.get_frame(), WIDTH, HEIGHT, DEPTH, Vec2i(0, 0)).bitblt(
                        &endscreen_tex,
                        Rect {
                            x: 0,
                            y: 0,
                            w: 700,
                            h: 550,
                        },
                        Vec2i(0, 0),
                    )
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
            let mut screen = Screen::wrap(pixels.get_frame(), WIDTH, HEIGHT, DEPTH, Vec2i(0, 0));
            // Eat up one frame worth of time
            available_time -= DT;

            update_game(&mut state, &input, frame_count);

            // Increment the frame counter
            frame_count += 1;
        }
        // Request redraw
        window.request_redraw();
        // When did the last frame end?
        since = Instant::now();
    });
}

fn update_game(state: &mut GameState, input: &WinitInputHelper, frame: usize) {
    let mut level_index: usize = state.current_level;
    match state.mode {
        Mode::TitleScreen => {
            if input.key_held(VirtualKeyCode::Return) {
                state.mode = Mode::GamePlay
            }
        }
        Mode::GamePlay => {
            // Player control goes here

            if (level_index == 1) {
                state.mode = Mode::EndGame;
            }
        }

        Mode::EndGame => {
            if input.key_held(VirtualKeyCode::Return) {
                state.current_level = 0;
                state.mode = Mode::GamePlay
            }
        }
    }

    // Handle collisions: Apply restitution impulses.

    // Update game rules: What happens when the player touches things?
}
