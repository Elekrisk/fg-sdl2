use sdl2::{render::Canvas, video::Window};

use crate::app_context::AppContext;

pub struct StateStack {
    states: Vec<Box<dyn State>>,
}

impl StateStack {
    pub fn new(initial_state: Box<dyn State>) -> Self {
        Self {
            states: vec![initial_state],
        }
    }

    pub fn push(&mut self, mut state: Box<dyn State>, ctx: &mut AppContext) {
        if let Some(last) = self.states.last_mut() {
            last.on_suspend(ctx);
        }
        state.on_enter(ctx);
        self.states.push(state);
    }

    pub fn pop(&mut self, ctx: &mut AppContext) {
        if let Some(mut popped) = self.states.pop() {
            popped.on_exit(ctx);
        }

        if let Some(last) = self.states.last_mut() {
            last.on_resume(ctx);
        }
    }

    pub fn tick(&mut self, ctx: &mut AppContext) {
        match self.states.last_mut().unwrap().tick(ctx) {
            StateTransition::None => {}
            StateTransition::Push(state) => self.push(state, ctx),
            StateTransition::Pop(amt) => {
                for _ in 0..amt {
                    self.pop(ctx);
                }
            }
            StateTransition::PopPush(amt, state) => {
                for _ in 0..amt {
                    self.pop(ctx);
                }
                self.push(state,ctx
                 );
            }
        }
    }

    pub fn render(&mut self, canvas: &mut Canvas<Window>, ctx: &mut AppContext) {
        for state in &mut self.states {
            state.render(canvas, ctx);
        }
    }

    pub fn event(&mut self, event: sdl2::event::Event, ctx: &mut AppContext) {
        self.states.last_mut().unwrap().event(event, ctx);
    }

    pub fn len(&self) -> usize {
        self.states.len()
    }
}

pub enum StateTransition {
    None,
    Push(Box<dyn State>),
    Pop(usize),
    PopPush(usize, Box<dyn State>),
}

pub trait State {
    fn on_enter(&mut self, ctx: &mut AppContext);
    fn on_suspend(&mut self, ctx: &mut AppContext);
    fn on_resume(&mut self, ctx: &mut AppContext);
    fn on_exit(&mut self, ctx: &mut AppContext);

    fn event(&mut self, event: sdl2::event::Event, ctx: &mut AppContext);
    fn tick(&mut self, ctx: &mut AppContext) -> StateTransition;
    fn render(&mut self, canvas: &mut Canvas<Window>, ctx: &mut AppContext);
}
