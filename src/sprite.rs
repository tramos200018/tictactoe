use crate::animation::Animation;
use crate::texture::Texture;
use crate::types::{Rect, Vec2i};
use std::rc::Rc;
use std::sync::Arc;

pub struct Sprite {
    image: Rc<Texture>,
    pub animation: Rc<Animation>, // Maybe better to use a type that can't have a negative origin
    // Or use =animation:Animation= instead of a frame field
    pub current_frame: Rect,
    pub elapsed_time: usize,
    pub position: Vec2i,
}

impl Sprite {
    pub fn new(
        image: &Rc<Texture>,
        animation: &Rc<Animation>,
        current_frame: Rect,
        elapsed_time: usize,
        position: Vec2i,
    ) -> Self {
        Self {
            image: Rc::clone(image),
            animation: Rc::clone(animation),
            current_frame,
            elapsed_time,
            position,
        }
    }

    pub fn update_anim(&mut self) {
        if self.elapsed_time < 1 {
            self.current_frame = self.animation.frames[1];
            self.elapsed_time += 1;
        } else {
            self.current_frame = self.animation.frames[0];
            self.elapsed_time -= 1;
        }
    }
}

pub trait DrawSpriteExt {
    fn draw_sprite(&mut self, s: &Sprite);
}

use crate::screen::Screen;
impl<'fb> DrawSpriteExt for Screen<'fb> {
    fn draw_sprite(&mut self, s: &Sprite) {
        // This works because we're only using a public method of Screen here,
        // and the private fields of sprite are visible inside this module
        self.bitblt(&s.image, s.current_frame, s.position);
    }
}
