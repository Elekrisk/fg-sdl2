mod label;

use sdl2::{video::Window, render::Canvas, rect::Rect};

use crate::fvec2::FVec2;


pub trait Widget<S> {
    fn render(&self, window: RenderWindow);
}

pub struct RenderWindow<'a> {
    canvas: &'a mut Canvas<Window>,
    offset: FVec2,
    size: FVec2,
    orig_viewport: Rect
}

impl<'a> RenderWindow<'a> {
    fn new(canvas: &'a mut Canvas<Window>, size: FVec2) -> Self {
        let orig_viewport = canvas.viewport();
        canvas.set_viewport(Rect::new(0, 0, size.x as _, size.y as _));
        Self {
            canvas,
            offset: FVec2::new(0.0, 0.0),
            size,
            orig_viewport,
        }
    }

    fn subwindow(&mut self, offset: FVec2, size: FVec2) -> RenderWindow {
        let orig_viewport = self.canvas.viewport();
        let offset = self.offset + offset;
        self.canvas.set_viewport(Rect::new(offset.x as _, offset.y as _, size.x as _, size.y as _));
        RenderWindow { canvas: self.canvas, offset: offset, size, orig_viewport }
    }
}

impl<'a> Drop for RenderWindow<'a> {
    fn drop(&mut self) {
        self.canvas.set_viewport(self.orig_viewport);
    }
}
