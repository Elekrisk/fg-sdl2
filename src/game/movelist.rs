use super::{
    input::{Button, InputHistory, INPUT_BUFFER},
    time::Frame, fixed_point::FixedPoint,
};

use super::character::CharacterState;

pub struct Movelist {
    moves: Vec<Move>,
}

impl Movelist {
    pub fn new(moves: impl IntoIterator<Item = Move>) -> Self {
        Self {
            moves: moves.into_iter().collect(),
        }
    }

    pub fn perform(
        &self,
        frame: Frame,
        state: &CharacterState,
        input: &InputHistory,
    ) -> Option<&Move> {
        self.moves
            .iter()
            .filter(|mov| {
                mov.state_matcher.matches(state) && mov.input_matcher.matches(frame, input)
            })
            .max_by_key(|mov| mov.priority)
    }
}

pub struct Move {
    pub name: String,
    pub input_matcher: InputMatcher,
    pub state_matcher: StateMatcher,
    pub priority: usize,
    pub new_state: CharacterState,
    pub stops_momentum: bool,
}

pub struct InputMatcher {
    func: Box<dyn Fn(Frame, &InputHistory) -> bool>,
}

impl InputMatcher {
    pub fn new(func: impl Fn(Frame, &InputHistory) -> bool + 'static) -> Self {
        Self {
            func: Box::new(func),
        }
    }

    fn matches(&self, frame: Frame, input: &InputHistory) -> bool {
        (self.func)(frame, input)
    }

    pub fn neutral_normal(button: Button) -> Self {
        Self::new(move |frame, history| {
            history
                .iter(INPUT_BUFFER, frame)
                .find(|ie| ie.pressed && match ie.kind {
                    super::input::InputKind::Button(btn) => btn == button,
                    _ => false,
                })
                .is_some()
        })
    }
}

pub struct StateMatcher {
    func: Box<dyn Fn(&CharacterState) -> bool>,
}

impl StateMatcher {
    pub fn new(func: impl Fn(&CharacterState) -> bool + 'static) -> Self {
        Self {
            func: Box::new(func),
        }
    }

    fn matches(&self, state: &CharacterState) -> bool {
        (self.func)(state)
    }

    pub fn any() -> Self {
        Self::new(|_| true)
    }

    pub fn idle() -> Self {
        Self::new(|s| matches!(s, CharacterState::Idle(_)))
    }

    pub fn airborne() -> Self {
        Self::new(|s| matches!(s, CharacterState::Airborne(_)))
    }

    pub fn blockstun() -> Self {
        Self::new(|s| matches!(s, CharacterState::Blockstun(_)))
    }

    pub fn hitstun() -> Self {
        Self::new(|s| matches!(s, CharacterState::Hitstun(_)))
    }

    pub fn specific(state: CharacterState) -> Self {
        Self::new(move |s| *s == state)
    }
}

#[derive(Clone)]
pub struct HitEffect {
    pub unblockable: bool,
    pub effects_on_hit: Vec<Effect>,
    pub effects_on_block: Vec<Effect>,
}

#[derive(Clone)]
pub enum Effect {
    Damage(FixedPoint),
    Hitstun(usize),
    Blockstun(usize),
    Knockback(FixedPoint),
    StateChange(CharacterState),
}
