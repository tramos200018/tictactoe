use crate::texture::Texture;
use std::path::Path;
use std::rc::Rc;
pub struct Resources();

impl Resources {
    pub fn new() -> Self {
        Self()
    }
    pub fn load_texture(&self, p: impl AsRef<Path>) -> Rc<Texture> {
        Rc::new(Texture::with_file(p.as_ref()))
    }
}
