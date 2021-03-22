use pixels::{Pixels, SurfaceTexture};
use std::time::Instant;
use winit::dpi::PhysicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

// seconds per frame
const DT: f64 = 1.0 / 60.0;

const DEPTH: usize = 4;
const WIDTH: usize = 700;
const HEIGHT: usize = 550;
const PITCH: usize = WIDTH * DEPTH;

// We'll make our Color type an RGBA8888 pixel.
type Color = [u8; DEPTH];

const CLEAR_COL: Color = [32, 32, 64, 255];
const WALL_COL: Color = [200, 200, 200, 255];
const PLAYER_COL: Color = [255, 128, 128, 255];
const NEXT_COL: Color = [255, 0, 0, 255];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u16,
    pub h: u16,
}
struct Level {}

pub struct Wall {
    pub rect: Rect,
}

pub struct Mobile {
    pub rect: Rect,
    pub vx: i32,
    pub vy: i32,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum ColliderID {
    Static(usize),
    Dynamic(usize),
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
struct Contact {
    a: ColliderID,
    b: ColliderID,
    mtv: (i32, i32),
}

// pixels gives us an rgba8888 framebuffer
pub fn clear(fb: &mut [u8], c: Color) {
    // Four bytes per pixel; chunks_exact_mut gives an iterator over 4-element slices.
    // So this way we can use copy_from_slice to copy our color slice into px very quickly.
    for px in fb.chunks_exact_mut(4) {
        px.copy_from_slice(&c);
    }
}
pub fn rect_touching(r1: Rect, r2: Rect) -> bool {
    // r1 left is left of r2 right
    r1.x <= r2.x+r2.w as i32 &&
        // r2 left is left of r1 right
        r2.x <= r1.x+r1.w as i32 &&
        // those two conditions handle the x axis overlap;
        // the next two do the same for the y axis:
        r1.y <= r2.y+r2.h as i32 &&
        r2.y <= r1.y+r1.h as i32
}
fn hline(fb: &mut [u8], x0: usize, x1: usize, y: usize, c: Color) {
    assert!(y < HEIGHT);
    assert!(x0 <= x1);
    assert!(x1 < WIDTH);
    for p in fb[(y * WIDTH * 4 + x0 * 4)..(y * WIDTH * 4 + x1 * 4)].chunks_exact_mut(4) {
        p.copy_from_slice(&c);
    }
}

fn line(fb: &mut [u8], (x0, y0): (usize, usize), (x1, y1): (usize, usize), col: Color) {
    let mut x = x0 as i64;
    let mut y = y0 as i64;
    let x0 = x0 as i64;
    let y0 = y0 as i64;
    let x1 = x1 as i64;
    let y1 = y1 as i64;
    let dx = (x1 - x0).abs();
    let sx: i64 = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy: i64 = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    while x != x1 || y != y1 {
        fb[(y as usize * WIDTH * DEPTH + x as usize * DEPTH)
            ..(y as usize * WIDTH * DEPTH + (x as usize + 1) * DEPTH)]
            .copy_from_slice(&col);
        let e2 = 2 * err;
        if dy <= e2 {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

#[allow(dead_code)]
pub fn rect(fb: &mut [u8], r: Rect, c: Color) {
    assert!(r.x < WIDTH as i32);
    assert!(r.y < HEIGHT as i32);
    // NOTE, very fragile! will break for out of bounds rects!  See next week for the fix.
    let x1 = (r.x + r.w as i32).min(WIDTH as i32) as usize;
    let y1 = (r.y + r.h as i32).min(HEIGHT as i32) as usize;
    for row in fb[(r.y as usize * PITCH)..(y1 * PITCH)].chunks_exact_mut(PITCH) {
        for p in row[(r.x as usize * DEPTH)..(x1 * DEPTH)].chunks_exact_mut(DEPTH) {
            p.copy_from_slice(&c);
        }
    }
}
pub fn triangle(fb: &mut [u8], (x0, y0): (usize, usize), b: usize, h: usize, col: Color) {
    line(fb, (x0, y0), (x0, y0 + h), col);
    line(fb, (x0, y0 + h), (x0 + b, y0 + h / 2), col);
    line(fb, (x0, y0), (x0 + b, y0 + h / 2), col);
}
pub fn frameRect(fb: &mut [u8], r: Rect, c: Color) {
    assert!(r.x < WIDTH as i32);
    assert!(r.y < HEIGHT as i32);
    // NOTE, very fragile! will break for out of bounds rects!  See next week for the fix.
    let x1 = (r.x + r.w as i32).min(WIDTH as i32) as usize;
    let y1 = (r.y + r.h as i32).min(HEIGHT as i32) as usize;
    hline(
        fb,
        r.x as usize,
        r.x as usize + r.w as usize,
        r.y as usize,
        c,
    );
    hline(
        fb,
        r.x as usize,
        r.x as usize + r.w as usize,
        r.y as usize + r.h as usize,
        c,
    );
    line(
        fb,
        (r.x as usize, r.y as usize),
        (r.x as usize, r.y as usize + r.h as usize),
        c,
    );
    line(
        fb,
        (r.x as usize + r.w as usize, r.y as usize),
        (r.x as usize + r.w as usize, r.y as usize + r.h as usize),
        c,
    );
}
fn rect_displacement(r1: Rect, r2: Rect) -> Option<(i32, i32)> {
    // Draw this out on paper to double check, but these quantities
    // will both be positive exactly when the conditions in rect_touching are true.
    let x_overlap = (r1.x + r1.w as i32).min(r2.x + r2.w as i32) - r1.x.max(r2.x);
    let y_overlap = (r1.y + r1.h as i32).min(r2.y + r2.h as i32) - r1.y.max(r2.y);
    if x_overlap >= 0 && y_overlap >= 0 {
        // This will return the magnitude of overlap in each axis.
        Some((x_overlap, y_overlap))
    } else {
        None
    }
}

// Here we will be using push() on into, so it can't be a slice
fn gather_contacts(statics: &[Wall], dynamics: &[Mobile], into: &mut Vec<Contact>) {
    // collide mobiles against mobiles
    for (ai, a) in dynamics.iter().enumerate() {
        for (bi, b) in dynamics.iter().enumerate().skip(ai + 1) {
            if let Some(disp) = rect_displacement(a.rect, b.rect) {
                into.push(Contact {
                    a: ColliderID::Dynamic(ai),
                    b: ColliderID::Dynamic(bi),
                    mtv: disp,
                });
            }
        }
    }
    // collide mobiles against walls
    for (ai, a) in dynamics.iter().enumerate() {
        for (bi, b) in statics.iter().enumerate() {
            if let Some(disp) = rect_displacement(a.rect, b.rect) {
                into.push(Contact {
                    a: ColliderID::Dynamic(ai),
                    b: ColliderID::Static(bi),
                    mtv: disp,
                });
            }
        }
    }
}

fn restitute(statics: &[Wall], dynamics: &mut [Mobile], contacts: &mut [Contact]) {
    // handle restitution of dynamics against dynamics and dynamics against statics wrt contacts.
    // You could instead make contacts `Vec<Contact>` if you think you might remove contacts.
    // You could also add an additional parameter, a slice or vec representing how far we've displaced each dynamic, to avoid allocations if you track a vec of how far things have been moved.
    // You might also want to pass in another &mut Vec<Contact> to be filled in with "real" touches that actually happened.
    contacts.sort_unstable_by_key(|c| -(c.mtv.0 * c.mtv.0 + c.mtv.1 * c.mtv.1));
    for contact in contacts.iter() {
        match contact {
            Contact {
                a: ColliderID::Dynamic(f),
                b: ColliderID::Static(g),
                mtv,
            } => {
                let f: usize = f.to_owned();
                let g: usize = g.to_owned();

                if rect_touching(dynamics[f].rect, statics[g].rect) {
                    dynamics[f].rect.x = 170;
                    dynamics[f].rect.y = 170;
                }
            }
            _ => {}
        }
    }
    // Keep going!  Note that you can assume every contact has a dynamic object in .a.
    // You might decide to tweak the interface of this function to separately take dynamic-static and dynamic-dynamic contacts, to avoid a branch inside of the response calculation.
    // Or, you might decide to calculate signed mtvs taking direction into account instead of the unsigned displacements from rect_displacement up above.  Or calculate one MTV per involved entity, then apply displacements to both objects during restitution (sorting by the max or the sum of their magnitudes)
}

fn main() {}
