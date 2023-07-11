use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use sdl2::keyboard::Scancode;

use crate::time::Frame;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Input {
    Keyboard(Scancode),
    Gamepad(sdl2::controller::Button),
}

impl From<Scancode> for Input {
    fn from(value: Scancode) -> Self {
        Self::Keyboard(value)
    }
}

impl From<sdl2::controller::Button> for Input {
    fn from(value: sdl2::controller::Button) -> Self {
        Self::Gamepad(value)
    }
}

pub struct InputMapping {
    grabbed_controller: Option<u32>,
    map: HashMap<Input, Action>,
}

impl InputMapping {
    pub fn empty() -> Self {
        Self {
            grabbed_controller: None,
            map: HashMap::new(),
        }
    }

    pub fn grab_controller(&mut self, controller: u32) {
        self.grabbed_controller = Some(controller);
    }

    pub fn release_controller(&mut self) {
        self.grabbed_controller = None;
    }

    pub fn default_keyboard() -> Self {
        let mut s = Self::empty();
        s.add_mapping(Scancode::W, Action::MoveUp);
        s.add_mapping(Scancode::A, Action::MoveLeft);
        s.add_mapping(Scancode::S, Action::MoveDown);
        s.add_mapping(Scancode::D, Action::MoveRight);
        s.add_mapping(Scancode::J, Action::Punch);
        s.add_mapping(Scancode::K, Action::Kick);
        s
    }

    pub fn default_gamepad() -> Self {
        let mut s = Self::empty();
        s.add_mapping(sdl2::controller::Button::DPadUp, Action::MoveUp);
        s.add_mapping(sdl2::controller::Button::DPadLeft, Action::MoveLeft);
        s.add_mapping(sdl2::controller::Button::DPadDown, Action::MoveDown);
        s.add_mapping(sdl2::controller::Button::DPadRight, Action::MoveRight);
        s.add_mapping(sdl2::controller::Button::X, Action::Punch);
        s.add_mapping(sdl2::controller::Button::A, Action::Kick);
        s
    }

    pub fn add_mapping(&mut self, input: impl Into<Input>, action: Action) {
        self.map.insert(input.into(), action);
    }

    pub fn get_action(&self, input: impl Into<Input>) -> Option<Action> {
        self.map.get(&input.into()).copied()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    Punch,
    Kick,
}

impl Action {
    fn bit(self) -> usize {
        match self {
            Action::MoveLeft => 0,
            Action::MoveRight => 1,
            Action::MoveUp => 2,
            Action::MoveDown => 3,
            Action::Punch => 4,
            Action::Kick => 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Pod, Zeroable)]
#[repr(transparent)]
pub struct BoxedInput(u8);

impl BoxedInput {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn press(&mut self, action: Action) {
        self.0 |= 1 << action.bit();
    }

    pub fn release(&mut self, action: Action) {
        self.0 &= !(1 << action.bit());
    }

    pub fn is_pressed(&self, action: Action) -> bool {
        self.0 & (1 << action.bit()) != 0
    }

    pub fn is_released(&self, action: Action) -> bool {
        !self.is_pressed(action)
    }

    pub fn input_dir(&self) -> InputDirection {
        enum H {
            L,
            N,
            R,
        }

        enum V {
            U,
            N,
            D,
        }

        let h = match (
            self.is_pressed(Action::MoveLeft),
            self.is_pressed(Action::MoveRight),
        ) {
            (false, false) | (true, true) => H::N,
            (true, false) => H::L,
            (false, true) => H::R,
        };

        let v = match (
            self.is_pressed(Action::MoveUp),
            self.is_pressed(Action::MoveDown),
        ) {
            (false, false) | (true, true) => V::N,
            (true, false) => V::U,
            (false, true) => V::D,
        };

        match (h, v) {
            (H::L, V::U) => InputDirection::UpLeft,
            (H::L, V::N) => InputDirection::Left,
            (H::L, V::D) => InputDirection::DownLeft,
            (H::N, V::U) => InputDirection::Up,
            (H::N, V::N) => InputDirection::Neutral,
            (H::N, V::D) => InputDirection::Down,
            (H::R, V::U) => InputDirection::UpRight,
            (H::R, V::N) => InputDirection::Right,
            (H::R, V::D) => InputDirection::DownRight,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputDirection {
    Neutral,
    Right,
    DownRight,
    Down,
    DownLeft,
    Left,
    UpLeft,
    Up,
    UpRight,
}

#[derive(Debug, Clone, Hash)]
pub struct InputHistory {
    events: Vec<InputEvent>,
}

impl InputHistory {
    pub fn new() -> Self {
        Self { events: vec![] }
    }

    pub fn add(&mut self, event: InputEvent) {
        self.events.push(event);
    }

    pub fn iter(&self, allowed_diff: usize, current_frame: Frame) -> InputIter {
        InputIter {
            iter: self.events.iter().rev(),
            allowed_diff,
            current_frame,
        }
    }
}

type I<'a> = impl Iterator<Item = &'a InputEvent> + Clone;

pub struct InputIter<'a> {
    iter: I<'a>,
    allowed_diff: usize,
    current_frame: Frame,
}

pub const INPUT_BUFFER: usize = 5;

impl<'a> InputIter<'a> {
    pub fn update_frame_basis(&mut self, new_basis: Frame) {
        self.current_frame = new_basis;
    }

    pub fn update_allowed_diff(&mut self, allowed_diff: usize) {
        self.allowed_diff = allowed_diff;
    }

    pub fn input_dir_as_of_here(&self) -> InputDirection {
        let mut iter = self.iter.clone();

        iter.find_map(|ie| match &ie.kind {
            InputKind::Direction(dir) if ie.pressed => Some(*dir),
            _ => None,
        })
        .unwrap_or(InputDirection::Neutral)
    }
}

impl<'a> Iterator for InputIter<'a> {
    type Item = &'a InputEvent;

    fn next(&mut self) -> Option<Self::Item> {
        let event = self.iter.next()?;
        if self.current_frame.since_with_freeze(event.frame) > self.allowed_diff {
            None
        } else {
            Some(event)
        }
    }
}

#[derive(Debug, Clone, Hash)]
pub struct InputEvent {
    pub frame: Frame,
    pub kind: InputKind,
    pub pressed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InputKind {
    Direction(InputDirection),
    Button(Button),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Button {
    Punch,
    Kick,
}
