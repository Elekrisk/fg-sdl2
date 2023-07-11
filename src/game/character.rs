mod guy;

use std::collections::HashMap;

use macros::CharacterStateContainer;
use sdl2::{render::TextureCreator, video::WindowContext};

use crate::{fixed_point::FixedPoint, input::InputDirection, time::Frame};

use super::{animation::Animation, movelist::Movelist, GameInfo, GameState, PlayerSide};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Character {
    Guy,
}

pub struct CharacterProto {
    pub name: String,
    pub animations: HashMap<&'static str, &'static Animation>,
    pub movelist: Movelist,
}

impl CharacterProto {
    pub fn create_guy(texture_creator: &'static TextureCreator<WindowContext>) -> Self {
        Self {
            name: "Guy".into(),
            animations: [
                (
                    "idle",
                    Animation::load("assets/animations/c1_idle.anim", texture_creator).unwrap(),
                ),
                (
                    "walking",
                    Animation::load("assets/animations/c1_walking_v2.anim", texture_creator)
                        .unwrap(),
                ),
                (
                    "punch",
                    Animation::load("assets/animations/c1_punch.anim", texture_creator).unwrap(),
                ),
                (
                    "hitstun",
                    Animation::load("assets/animations/c1_hitstun.anim", texture_creator).unwrap(),
                ),
            ]
            .into_iter()
            .collect(),
            movelist: guy::movelist(),
        }
    }
}

pub trait State: Sized {
    fn on_exit(
        &mut self,
        player: PlayerSide,
    ) -> Option<Box<dyn FnOnce(&mut GameState, &GameInfo)>> {
        let _ = player;
        None
    }

    fn pre_tick(
        &mut self,
        frame: Frame,
        player: PlayerSide,
    ) -> Option<Box<dyn FnOnce(&mut GameState, &GameInfo)>> {
        let _ = (frame, player);
        None
    }

    fn tick(
        &mut self,
        frame: Frame,
        player: PlayerSide,
    ) -> Option<Box<dyn FnOnce(&mut GameState, &GameInfo)>> {
        let _ = (frame, player);
        None
    }

    fn on_enter(
        &mut self,
        player: PlayerSide,
    ) -> Option<Box<dyn FnOnce(&mut GameState, &GameInfo)>> {
        let _ = player;
        None
    }

    fn priority(&self) -> usize;
}

#[derive(Clone)]
pub struct StateTransitionRequests {
    requests: Vec<StateTransitionRequest>,
}

impl StateTransitionRequests {
    pub fn new() -> Self {
        Self { requests: vec![] }
    }

    pub fn add(&mut self, req: StateTransitionRequest) {
        self.requests.push(req)
    }

    pub fn take(&mut self) -> Option<StateTransitionRequest> {
        self.requests
            .drain(..)
            .max_by_key(|req| req.insert_priority)
    }
}

#[derive(Clone)]
pub struct StateTransitionRequest {
    pub state: CharacterState,
    pub insert_priority: usize,
}

impl StateTransitionRequest {
    pub fn new(state: CharacterState, insert_priority: impl Into<Option<usize>>) -> Self {
        let insert_priority = insert_priority.into().unwrap_or_else(|| state.priority());
        Self {
            state,
            insert_priority,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, CharacterStateContainer)]
pub enum CharacterState {
    Idle(IdleState),
    Airborne(AirborneState),
    Blockstun(BlockstunState),
    Hitstun(HitstunState),
    CharacterSpecific(CharacterSpecificState),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdleState;

impl State for IdleState {
    fn on_enter(
        &mut self,
        player: PlayerSide,
    ) -> Option<Box<dyn FnOnce(&mut GameState, &GameInfo)>> {
        Some(Box::new(move |state, info| {
            let frame = state.current_frame;
            let player = state.player_mut(player);
            player.velocity.x = FixedPoint::ZERO;
            player.animator.switch_animation(
                frame,
                info.character_protos[&player.character].animations["idle"],
                None,
            );
        }))
    }

    fn tick(
        &mut self,
        frame: Frame,
        player: PlayerSide,
    ) -> Option<Box<dyn FnOnce(&mut GameState, &GameInfo)>> {
        Some(Box::new(move |state, info| {
            let player = state.player_mut(player);
            let dir = player.input_history.iter(0, frame).input_dir_as_of_here();

            match dir {
                InputDirection::Right => {
                    player.velocity.x = FixedPoint::from(60.0);
                    player.animator.make_sure_animation(
                        frame,
                        info.character_protos[&player.character].animations["walking"],
                        None,
                    );
                }
                InputDirection::Left => {
                    player.velocity.x = FixedPoint::from(-60.0);
                    player.animator.make_sure_animation(
                        frame,
                        info.character_protos[&player.character].animations["walking"],
                        None,
                    );
                }
                InputDirection::Neutral => {
                    player.velocity.x = FixedPoint::ZERO;
                    player.animator.make_sure_animation(
                        frame,
                        info.character_protos[&player.character].animations["idle"],
                        None,
                    );
                }
                InputDirection::Up => {
                    player.velocity.y = -FixedPoint::from(200);
                    player
                        .state_transition_requests
                        .add(StateTransitionRequest::new(
                            AirborneState(JumpDirection::Up).wrap(),
                            None,
                        ));
                }
                InputDirection::UpRight => {
                    player.velocity.y = -FixedPoint::from(200);
                    player
                        .state_transition_requests
                        .add(StateTransitionRequest::new(
                            AirborneState(JumpDirection::Right).wrap(),
                            None,
                        ));
                }
                InputDirection::UpLeft => {
                    player.velocity.y = -FixedPoint::from(200);
                    player
                        .state_transition_requests
                        .add(StateTransitionRequest::new(
                            AirborneState(JumpDirection::Left).wrap(),
                            None,
                        ));
                }
                _ => {}
            }
        }))
    }

    fn priority(&self) -> usize {
        0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpDirection {
    Left,
    Up,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AirborneState(pub JumpDirection);

impl State for AirborneState {
    fn on_enter(
        &mut self,
        player: PlayerSide,
    ) -> Option<Box<dyn FnOnce(&mut GameState, &GameInfo)>> {
        let dir = self.0;
        Some(Box::new(move |state, info| {
            state.player_mut(player).velocity.x = FixedPoint::from(match dir {
                JumpDirection::Left => -60.0,
                JumpDirection::Up => 0.0,
                JumpDirection::Right => 60.,
            })
        }))
    }

    fn priority(&self) -> usize {
        1000
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockstunState(pub usize);

impl State for BlockstunState {
    fn pre_tick(
        &mut self,
        frame: Frame,
        player: PlayerSide,
    ) -> Option<Box<dyn FnOnce(&mut GameState, &GameInfo)>> {
        if self.0 <= 1 {
            Some(Box::new(move |state, _| {
                state
                    .player_mut(player)
                    .state_transition_requests
                    .add(StateTransitionRequest::new(IdleState.wrap(), usize::MAX))
            }))
        } else {
            self.0 -= 1;
            None
        }
    }

    fn priority(&self) -> usize {
        1000
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HitstunState(pub usize);

impl State for HitstunState {
    fn pre_tick(
        &mut self,
        frame: Frame,
        player: PlayerSide,
    ) -> Option<Box<dyn FnOnce(&mut GameState, &GameInfo)>> {
        if self.0 <= 1 {
            Some(Box::new(move |state, _| {
                state
                    .player_mut(player)
                    .state_transition_requests
                    .add(StateTransitionRequest::new(IdleState.wrap(), usize::MAX))
            }))
        } else {
            self.0 -= 1;
            None
        }
    }

    fn on_enter(
        &mut self,
        player: PlayerSide,
    ) -> Option<Box<dyn FnOnce(&mut GameState, &GameInfo)>> {
        Some(Box::new(move |state, info| {
            let frame = state.current_frame;
            let player = state.player_mut(player);
            player.animator.switch_animation(
                frame,
                info.character_protos[&player.character].animations["hitstun"],
                None,
            )
        }))
    }

    fn priority(&self) -> usize {
        1000
    }
}

#[derive(Debug, Clone, PartialEq, Eq, CharacterStateContainer)]
pub enum CharacterSpecificState {
    Guy(guy::GuyState),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Facing {
    Left,
    Right,
}

pub enum CharacterDirection {
    Neutral,
    Forward,
    DownForward,
    Down,
    DownBackward,
    Backward,
    UpBackward,
    Up,
    UpForward,
}

impl CharacterDirection {
    pub fn from_input_dir(dir: InputDirection, facing: Facing) -> Self {
        match (dir, facing) {
            (InputDirection::Neutral, _) => CharacterDirection::Neutral,
            (InputDirection::Up, _) => CharacterDirection::Up,
            (InputDirection::Down, _) => CharacterDirection::Down,
            (InputDirection::Right, Facing::Right) => CharacterDirection::Forward,
            (InputDirection::Right, Facing::Left) => CharacterDirection::Backward,
            (InputDirection::UpRight, Facing::Right) => CharacterDirection::UpForward,
            (InputDirection::UpRight, Facing::Left) => CharacterDirection::UpBackward,
            (InputDirection::DownRight, Facing::Right) => CharacterDirection::DownForward,
            (InputDirection::DownRight, Facing::Left) => CharacterDirection::DownBackward,
            (InputDirection::Left, Facing::Right) => CharacterDirection::Backward,
            (InputDirection::Left, Facing::Left) => CharacterDirection::Forward,
            (InputDirection::UpLeft, Facing::Right) => CharacterDirection::UpBackward,
            (InputDirection::UpLeft, Facing::Left) => CharacterDirection::UpForward,
            (InputDirection::DownLeft, Facing::Right) => CharacterDirection::DownBackward,
            (InputDirection::DownLeft, Facing::Left) => CharacterDirection::DownForward,
        }
    }

    pub fn is_down(&self) -> bool {
        matches!(self, Self::Down | Self::DownBackward | Self::DownForward)
    }

    pub fn is_up(&self) -> bool {
        matches!(self, Self::Up | Self::UpBackward | Self::UpForward)
    }

    pub fn is_forward(&self) -> bool {
        matches!(self, Self::Forward | Self::DownForward | Self::UpForward)
    }

    pub fn is_backward(&self) -> bool {
        matches!(self, Self::Backward | Self::DownBackward | Self::UpBackward)
    }
}
