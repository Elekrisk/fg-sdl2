use macros::CharacterStateContainer;

use crate::{
    game::{
        movelist::{InputMatcher, Move, Movelist, StateMatcher, HitEffect, Effect},
        GameInfo, GameState, PlayerSide, Attack,
    },
    input::Button,
    time::Frame, fixed_point::FixedPoint,
};

use super::{CharacterSpecificState, CharacterState, IdleState, State};

#[derive(Debug, Clone, PartialEq, Eq, CharacterStateContainer)]
pub enum GuyState {
    Normal(Normal),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Normal {
    NeutralPunch,
}

impl State for Normal {
    fn on_enter(
        &mut self,
        player: PlayerSide,
    ) -> Option<Box<dyn FnOnce(&mut GameState, &GameInfo)>> {
        match self {
            Normal::NeutralPunch => Some(Box::new(move |state, info| {
                let frame = state.current_frame;
                let player = state.player_mut(player);
                player.animator.make_sure_animation(
                    frame,
                    info.character_protos[&player.character].animations["punch"],
                    Some(IdleState.wrap()),
                );
                player.current_attack = Some(Attack {
                    has_hit_player: false,
                    hit_effect: HitEffect {
                        unblockable: false,
                        effects_on_hit: vec![Effect::Hitstun(14), Effect::Knockback(FixedPoint::from(30.0)), Effect::Damage(2.into())],
                        effects_on_block: vec![Effect::Blockstun(14), Effect::Knockback(FixedPoint::from(30.0))],
                    },
                })
            })),
        }
    }

    fn on_exit(
            &mut self,
            player: PlayerSide,
        ) -> Option<Box<dyn FnOnce(&mut GameState, &GameInfo)>> {
        match self {
            Normal::NeutralPunch => Some(Box::new(move |state, info| {
                state.player_mut(player).current_attack = None;
            })),
        }
    }

    fn priority(&self) -> usize {
        match self {
            Normal::NeutralPunch => 10,
        }
    }
}

pub fn movelist() -> Movelist {
    Movelist::new([Move {
        name: "Punch".into(),
        input_matcher: InputMatcher::neutral_normal(Button::Punch),
        state_matcher: StateMatcher::idle(),
        priority: 10,
        new_state: Normal::NeutralPunch.wrap().wrap().wrap(),
        stops_momentum: true,
    }])
}
