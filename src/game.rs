mod animation;
mod character;
mod movelist;
mod camera;
pub mod fixed_point;
mod input;
mod time;

use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    marker::PhantomData,
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
    str::FromStr,
    time::Duration,
};

use clap::ValueEnum;
use ggrs::{GGRSError, P2PSession, SessionState, UdpNonBlockingSocket};
use sdl2::{
    pixels::Color,
    render::{Canvas, Texture, TextureCreator},
    video::{Window, WindowContext},
};

use camera::Camera;
use fixed_point::{FixedPoint, Rect, Vec2};
use crate::fvec2::FVec2;
use input::{Action, BoxedInput, Input, InputEvent, InputHistory, InputMapping};
use time::Frame;

use crate::state::StateTransition;

use self::{
    animation::Animation,
    character::{
        BlockstunState, Character, CharacterDirection, CharacterProto, CharacterState, Facing,
        HitstunState, IdleState, State, StateTransitionRequest, StateTransitionRequests,
    },
    movelist::{Effect, HitEffect},
};

pub const FPS: usize = 60;

pub enum PlayerType {
    Local {
        mapping: InputMapping,
        input: BoxedInput,
    },
    Remote,
}

pub struct GameInfo {
    pub character_protos: HashMap<Character, CharacterProto>,
    pub bg_texture: Texture<'static>,
    pub player_1: PlayerType,
    pub player_2: PlayerType,
    pub waiting_for_network: bool,
}

impl GameInfo {
    pub fn create(texture_creator: &'static TextureCreator<WindowContext>) -> Self {
        Self {
            character_protos: [(Character::Guy, CharacterProto::create_guy(texture_creator))]
                .into_iter()
                .collect(),
            bg_texture: {
                let img = image::load_from_memory(&std::fs::read("assets/sprites/bg.png").unwrap())
                    .unwrap()
                    .into_rgba8();
                let mut texture = texture_creator
                    .create_texture(
                        sdl2::pixels::PixelFormatEnum::ABGR8888,
                        sdl2::render::TextureAccess::Static,
                        1920,
                        1080,
                    )
                    .unwrap();
                texture.set_blend_mode(sdl2::render::BlendMode::Blend);
                texture.update(None, &img, 1920 * 4).unwrap();
                texture
            },
            player_1: PlayerType::Local {
                mapping: InputMapping::default_keyboard(),
                input: BoxedInput::new(),
            },
            player_2: PlayerType::Local {
                mapping: InputMapping::default_gamepad(),
                input: BoxedInput::new(),
            },
            waiting_for_network: false,
        }
    }
}

#[derive(Clone)]
pub struct GameState {
    player_1: Player,
    player_2: Player,

    camera: Camera,

    current_frame: Frame,
}

impl GameState {
    pub fn new(game_info: &GameInfo) -> Self {
        Self {
            player_1: Player {
                health: FixedPoint::from(100),
                position: Vec2::new((-50.0).try_into().unwrap(), 0.0.try_into().unwrap()),
                velocity: Vec2::new(FixedPoint::from(0), FixedPoint::from(0)),
                character: Character::Guy,
                last_input: BoxedInput::new(),
                input_history: InputHistory::new(),
                animator: Animator::new(
                    Frame::new(),
                    &game_info.character_protos[&Character::Guy].animations["idle"],
                ),
                current_state: IdleState.wrap(),
                state_transition_requests: StateTransitionRequests::new(),
                hurtboxes: vec![],
                hitboxes: vec![],
                current_attack: None,
                facing: Facing::Right,
                grounded: true,
            },
            player_2: Player {
                health: FixedPoint::from(100),
                position: Vec2::new((50.0).try_into().unwrap(), 0.0.try_into().unwrap()),
                velocity: Vec2::new(FixedPoint::from(0), FixedPoint::from(0)),
                character: Character::Guy,
                last_input: BoxedInput::new(),
                input_history: InputHistory::new(),
                animator: Animator::new(
                    Frame::new(),
                    &game_info.character_protos[&Character::Guy].animations["idle"],
                ),
                current_state: IdleState.wrap(),
                state_transition_requests: StateTransitionRequests::new(),
                hurtboxes: vec![],
                hitboxes: vec![],
                current_attack: None,
                facing: Facing::Left,
                grounded: true,
            },
            camera: Camera {
                center: FVec2::new(0.0, 30.0),
                scale: 5.0,
                width: 1280.0,
                height: 720.0,
                offset: FVec2::new(0.0, 0.0),
            },
            current_frame: Frame::new(),
        }
    }

    pub fn player(&self, player: PlayerSide) -> &Player {
        match player {
            PlayerSide::Player1 => &self.player_1,
            PlayerSide::Player2 => &self.player_2,
        }
    }

    pub fn player_mut(&mut self, player: PlayerSide) -> &mut Player {
        match player {
            PlayerSide::Player1 => &mut self.player_1,
            PlayerSide::Player2 => &mut self.player_2,
        }
    }

    fn player_input(&mut self, player: PlayerSide, input: BoxedInput) {
        let frame = self.current_frame;
        self.player_mut(player).input(frame, input);
    }

    fn do_player_pre_tick(&mut self, game_info: &GameInfo) {
        self.player_pre_tick(game_info, PlayerSide::Player1);
        self.player_pre_tick(game_info, PlayerSide::Player2);
    }

    fn player_pre_tick(&mut self, game_info: &GameInfo, player_side: PlayerSide) {
        let frame = self.current_frame;
        if let Some(cmd) = self
            .player_mut(player_side)
            .current_state
            .pre_tick(frame, player_side)
        {
            cmd(self, game_info);
        }
    }

    fn do_player_tick(&mut self, game_info: &GameInfo) {
        self.player_tick(game_info, PlayerSide::Player1);
        self.player_tick(game_info, PlayerSide::Player2);
    }

    fn player_tick(&mut self, game_info: &GameInfo, player_side: PlayerSide) {
        let frame = self.current_frame;
        if let Some(cmd) = self
            .player_mut(player_side)
            .current_state
            .tick(frame, player_side)
        {
            cmd(self, game_info);
        }
    }

    fn moves(&mut self, game_info: &GameInfo) {
        self.handle_player_moves(game_info, PlayerSide::Player1);
        self.handle_player_moves(game_info, PlayerSide::Player2);
    }

    fn handle_player_moves(&mut self, game_info: &GameInfo, player_side: PlayerSide) {
        let frame = self.current_frame;
        let player = self.player_mut(player_side);
        let proto = &game_info.character_protos[&player.character];
        if let Some(mov) =
            proto
                .movelist
                .perform(frame, &player.current_state, &player.input_history)
        {
            player
                .state_transition_requests
                .add(StateTransitionRequest::new(mov.new_state.clone(), None));
            if mov.stops_momentum {
                player.velocity = Vec2::new(FixedPoint::ZERO, FixedPoint::ZERO);
            }
        }
    }

    fn state_transitions(&mut self, game_info: &GameInfo) {
        self.handle_player_state_transitions(game_info, PlayerSide::Player1);
        self.handle_player_state_transitions(game_info, PlayerSide::Player2);
    }

    fn handle_player_state_transitions(&mut self, game_info: &GameInfo, player_side: PlayerSide) {
        let player = self.player_mut(player_side);
        let Some(req) = player.state_transition_requests.take() else {
            return;
        };

        println!("Transitioning to {:?}", req.state);

        if req.insert_priority > player.current_state.priority() {
            if let Some(cmd) = player.current_state.on_exit(player_side) {
                cmd(self, game_info)
            }
            let player = self.player_mut(player_side);
            player.current_state = req.state;

            if let Some(cmd) = player.current_state.on_enter(player_side) {
                cmd(self, game_info)
            }
        }
    }

    fn handle_player_hitreg(&mut self, player_side: PlayerSide) -> Vec<Hit> {
        let attacker = self.player(player_side);
        let target = self.player(player_side.reverse());

        let mut hits = vec![];

        for hitbox in &attacker.hitboxes {
            for hurtbox in &target.hurtboxes {
                if hitbox
                    .rect
                    .offset(attacker.position)
                    .overlaps(hurtbox.rect.offset(target.position))
                {
                    hits.push(Hit {
                        attacker: player_side,
                        target: player_side.reverse(),
                        attacker_tag: hitbox.tag.clone(),
                        target_tag: hurtbox.tag.clone(),
                    });
                }
            }
        }

        hits
    }

    fn hitreg(&mut self, game_info: &GameInfo) {
        let mut hits = vec![];
        hits.append(&mut self.handle_player_hitreg(PlayerSide::Player1));
        hits.append(&mut self.handle_player_hitreg(PlayerSide::Player2));

        for hit in hits {
            let Some(attack) = &mut self.player_mut(hit.attacker).current_attack else {
                continue;
            };
            if attack.has_hit_player {
                continue;
            }
            attack.has_hit_player = true;
            let Some(attack) = &self.player(hit.attacker).current_attack else {
                continue;
            };

            let attacker = self.player(hit.attacker);
            let target = self.player(hit.target);

            let blocked = !attack.hit_effect.unblockable && target.is_blocking(self.current_frame);

            let effects = if blocked {
                &attack.hit_effect.effects_on_block
            } else {
                &attack.hit_effect.effects_on_hit
            };

            for effect in effects.clone() {
                self.player_mut(hit.target).apply_effect(&effect);
            }
        }
    }

    fn player_physics(&mut self) {
        let delta = FixedPoint::from(1.0 / 60.0);

        let deccel = FixedPoint::from(100.0) * delta;

        for player in [&mut self.player_1, &mut self.player_2] {
            player.position += player.velocity * delta;
            player.velocity.y += FixedPoint::from(400.0) * delta;
            if player.position.y > FixedPoint::ZERO {
                player.position.y = FixedPoint::ZERO;
                player.velocity.y = FixedPoint::ZERO;
                player.grounded = true;
                if matches!(player.current_state, CharacterState::Airborne(_)) {
                    player
                        .state_transition_requests
                        .add(StateTransitionRequest::new(IdleState.wrap(), 10000));
                }
            } else if player.position.y < FixedPoint::ZERO {
                player.grounded = false;
            }

            if player.grounded {
                if player.velocity.x.abs() <= deccel {
                    player.velocity.x = FixedPoint::ZERO
                } else if player.velocity.x > FixedPoint::ZERO {
                    player.velocity.x -= deccel
                } else if player.velocity.x < FixedPoint::ZERO {
                    player.velocity.x += deccel
                }
            }
        }
    }

    pub fn tick(&mut self, game_info: &GameInfo) {
        if let Some(req) = self.player_1.animator.pre_tick(self.current_frame) {
            self.player_1.state_transition_requests.add(req);
        }
        if let Some(req) = self.player_2.animator.pre_tick(self.current_frame) {
            self.player_2.state_transition_requests.add(req);
        }

        self.do_player_pre_tick(game_info);

        self.state_transitions(game_info);

        self.moves(game_info);

        self.state_transitions(game_info);

        self.do_player_tick(game_info);

        self.state_transitions(game_info);

        self.player_1.animator.tick(
            self.player_1.facing,
            &mut self.player_1.hitboxes,
            &mut self.player_1.hurtboxes,
            self.current_frame,
        );
        self.player_2.animator.tick(
            self.player_2.facing,
            &mut self.player_2.hitboxes,
            &mut self.player_2.hurtboxes,
            self.current_frame,
        );

        self.state_transitions(game_info);

        self.player_physics();

        self.state_transitions(game_info);

        self.hitreg(game_info);

        self.state_transitions(game_info);

        let p1: FVec2 = self.player_1.position.into();
        let p2: FVec2 = self.player_2.position.into();

        let mut camera_center = (p1 + p2) / 2.0;
        camera_center.y = -50.0;
        self.camera.center = camera_center.into();

        self.current_frame.tick(false);
    }

    pub fn render(&mut self, game_info: &GameInfo, canvas: &mut Canvas<Window>) {
        canvas.set_draw_color(Color::BLACK);
        canvas.clear();

        let (width, height) = canvas.window().size();

        let aspect = width as f32 / height as f32;
        let target_aspect = 16.0 / 9.0;

        let (width, height) = if aspect > target_aspect {
            let logical_height = height;
            let logical_width = (height as f32 * target_aspect) as u32;

            let offset = (width - logical_width) / 2;

            canvas.set_clip_rect(sdl2::rect::Rect::new(
                offset as _,
                0,
                logical_width,
                logical_height,
            ));

            self.camera.offset = FVec2::new(offset as _, 0.0);

            (logical_width, logical_height)
        } else {
            let logical_height = (width as f32 / target_aspect) as u32;
            let logical_width = width;

            let offset = (height - logical_height) / 2;

            canvas.set_clip_rect(sdl2::rect::Rect::new(
                0,
                offset as _,
                logical_width,
                logical_height,
            ));

            self.camera.offset = FVec2::new(0.0, offset as _);

            (logical_width, logical_height)
        };
        canvas.set_draw_color(Color::WHITE);
        canvas.fill_rect(None).unwrap();

        self.camera.width = width as f32;
        self.camera.height = height as f32;

        self.camera.scale = self.camera.width / 1280.0 * 5.0;

        let bg_pos = FVec2::new(0.0, -354.0);
        let offset = FVec2::new(1920.0 / 2.0, 1080.0 / 2.0);
        let min = self.camera.to_screen_space(bg_pos - offset);
        let max = self.camera.to_screen_space(bg_pos + offset);

        let rect = sdl2::rect::Rect::new(
            min.x as _,
            min.y as _,
            (max.x - min.x) as _,
            (max.y - min.y) as _,
        );

        canvas.copy(&game_info.bg_texture, None, rect).unwrap();

        let w = width as i32;
        let h = height as i32;

        self.player_1.render(&self.camera, game_info, canvas);
        self.player_2.render(&self.camera, game_info, canvas);

        let margin = 50;
        let space_between = 200;
        let health_width = (w - margin * 2 - space_between) / 2;

        let p1hx = margin;
        let p2hx = margin + health_width + space_between;

        let hp_height = (10.0 * self.camera.scale) as _;

        canvas.set_draw_color(Color::RGB(127, 127, 127));
        canvas
            .fill_rect(sdl2::rect::Rect::new(
                p1hx + self.camera.offset.x as i32,
                margin + self.camera.offset.y as i32,
                health_width as _,
                hp_height,
            ))
            .unwrap();
        canvas
            .fill_rect(sdl2::rect::Rect::new(
                p2hx + self.camera.offset.x as i32,
                margin + self.camera.offset.y as i32,
                health_width as _,
                hp_height,
            ))
            .unwrap();

        let p1h = ((f64::from(self.player_1.health) / 100.0) * health_width as f64) as u32;
        let p2h = ((f64::from(self.player_2.health) / 100.0) * health_width as f64) as u32;

        let p2hx = p2hx + health_width as i32 - p2h as i32;

        canvas.set_draw_color(Color::RGB(255, 0, 0));
        canvas
            .fill_rect(sdl2::rect::Rect::new(
                p1hx + self.camera.offset.x as i32,
                margin + self.camera.offset.y as i32,
                p1h,
                hp_height,
            ))
            .unwrap();
        canvas
            .fill_rect(sdl2::rect::Rect::new(
                p2hx + self.camera.offset.x as i32,
                margin + self.camera.offset.y as i32,
                p2h,
                hp_height,
            ))
            .unwrap();

        if game_info.waiting_for_network {
            let offset = FVec2::new(25.0, 25.0);
            let min = self.camera.to_screen_space(self.camera.center - offset);
            let max = self.camera.to_screen_space(self.camera.center + offset);

            let x = min.x;
            let y = min.y;
            let w = max.x - min.x;
            let h = max.y - min.y;

            canvas.set_draw_color(Color::RGBA(255, 0, 0, 127));
            canvas.fill_rect(sdl2::rect::Rect::new(x as _, y as _, w as _, h as _)).unwrap();
        }

        canvas.present();
    }
}

pub struct Hit {
    attacker: PlayerSide,
    target: PlayerSide,
    attacker_tag: String,
    target_tag: String,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, ValueEnum)]
pub enum PlayerSide {
    Player1,
    Player2,
}

impl PlayerSide {
    pub fn reverse(self) -> Self {
        match self {
            PlayerSide::Player1 => PlayerSide::Player2,
            PlayerSide::Player2 => PlayerSide::Player1,
        }
    }
}

impl FromStr for PlayerSide {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_ref() {
            "1" | "p1" | "player1" | "player_1" => Ok(PlayerSide::Player1),
            "2" | "p2" | "player2" | "player_2" => Ok(PlayerSide::Player2),
            _ => Err(()),
        }
    }
}

#[derive(Clone)]
pub struct Player {
    health: FixedPoint,
    position: Vec2,
    velocity: Vec2,
    character: Character,
    last_input: BoxedInput,
    input_history: InputHistory,
    animator: Animator,
    current_state: CharacterState,
    state_transition_requests: StateTransitionRequests,
    hurtboxes: Vec<Hurtbox>,
    hitboxes: Vec<Hitbox>,
    current_attack: Option<Attack>,
    facing: Facing,
    grounded: bool,
}

impl Player {
    fn set_facing(&mut self, new_facing: Facing) {
        if self.facing == new_facing {
            return;
        }

        self.facing = new_facing;

        for hitbox in &mut self.hitboxes {
            let min_x = hitbox.rect.min.x;
            let max_x = hitbox.rect.max.x;

            hitbox.rect.min.x = -max_x;
            hitbox.rect.max.x = -min_x;
        }
    }

    fn input(&mut self, frame: Frame, input: BoxedInput) {
        let last_input = self.last_input;

        let last_dir = last_input.input_dir();
        let new_dir = input.input_dir();

        if last_dir != new_dir {
            self.input_history.add(InputEvent {
                frame,
                kind: input::InputKind::Direction(last_dir),
                pressed: false,
            });

            self.input_history.add(InputEvent {
                frame,
                kind: input::InputKind::Direction(new_dir),
                pressed: true,
            });
        }

        let last_punch = self.last_input.is_pressed(Action::Punch);
        let new_punch = input.is_pressed(Action::Punch);
        let last_kick = self.last_input.is_pressed(Action::Kick);
        let new_kick = input.is_pressed(Action::Kick);

        if last_punch != new_punch {
            self.input_history.add(InputEvent {
                frame,
                kind: input::InputKind::Button(input::Button::Punch),
                pressed: new_punch,
            });
        }

        if last_kick != new_kick {
            self.input_history.add(InputEvent {
                frame,
                kind: input::InputKind::Button(input::Button::Kick),
                pressed: new_kick,
            });
        }

        self.last_input = input;
    }

    pub fn render(&self, camera: &Camera, game_info: &GameInfo, canvas: &mut Canvas<Window>) {
        let flip = self.facing == Facing::Left;

        let anim = self.animator.current_animation;
        let texture = &anim.texture_atlas;

        let pos = self.position;

        let mut origin = anim.frame_data[self.animator.current_frame].origin;
        if flip {
            origin.x = FixedPoint::from(anim.cell_width) - origin.x;
        }

        let pos = pos - origin;

        let screen_pos = camera.to_screen_space(pos);

        let index = self.animator.current_frame;
        let cell_y = index / anim.columns;
        let cell_x = index % anim.columns;

        let src = sdl2::rect::Rect::new(
            (cell_x * anim.cell_width) as _,
            (cell_y * anim.cell_height) as _,
            anim.cell_width as _,
            anim.cell_height as _,
        );
        let w = anim.cell_width as f32 * camera.scale;
        let h = anim.cell_height as f32 * camera.scale;
        let dst = sdl2::rect::Rect::new(screen_pos.x as _, screen_pos.y as _, w as _, h as _);

        // let src = None;
        // let dst = None;
        canvas
            .copy_ex(texture, src, dst, 0.0, None, flip, false)
            .unwrap();

        // for hurtbox in &self.hurtboxes {
        //     let mut min = hurtbox.rect.min;
        //     let mut max = hurtbox.rect.max;

        //     // if flip {
        //     //     let min_x = min.x;
        //     //     let max_x = max.x;

        //     //     min.x = -max_x;
        //     //     max.x = -min_x;
        //     // }

        //     let x = self.position.x + min.x;
        //     let y = self.position.y + min.y;

        //     let pos = Vec2::new(x, y);

        //     let min = camera.to_screen_space(pos);

        //     let x = self.position.x + max.x;
        //     let y = self.position.y + max.y;

        //     let pos = Vec2::new(x, y);

        //     let max = camera.to_screen_space(pos);

        //     let pos = min;
        //     let size = max - min;
        //     let dst = sdl2::rect::Rect::new(pos.x as _, pos.y as _, size.x as _, size.y as _);

        //     canvas.set_draw_color(Color::RGBA(0, 255, 0, 120));
        //     canvas.fill_rect(dst).unwrap();
        // }

        // for hitbox in &self.hitboxes {
        //     let mut min = hitbox.rect.min;
        //     let mut max = hitbox.rect.max;

        //     // if flip {
        //     //     let min_x = min.x;
        //     //     let max_x = max.x;

        //     //     min.x = -max_x;
        //     //     max.x = -min_x;
        //     // }

        //     let x = self.position.x + min.x;
        //     let y = self.position.y + min.y;

        //     let pos = Vec2::new(x, y);

        //     let min = camera.to_screen_space(pos);

        //     let x = self.position.x + max.x;
        //     let y = self.position.y + max.y;

        //     let pos = Vec2::new(x, y);

        //     let max = camera.to_screen_space(pos);

        //     let pos = min;
        //     let size = max - min;
        //     let dst = sdl2::rect::Rect::new(pos.x as _, pos.y as _, size.x as _, size.y as _);

        //     canvas.set_draw_color(Color::RGBA(255, 0, 0, 120));
        //     canvas.fill_rect(dst).unwrap();
        // }
    }

    pub fn is_blocking(&self, frame: Frame) -> bool {
        if matches!(&self.current_state, CharacterState::Blockstun(_)) {
            return true;
        }

        if matches!(&self.current_state, CharacterState::Idle(_))
            && CharacterDirection::from_input_dir(
                self.input_history.iter(0, frame).input_dir_as_of_here(),
                self.facing,
            )
            .is_backward()
        {
            return true;
        }

        false
    }

    pub fn apply_effect(&mut self, effect: &Effect) {
        match effect {
            Effect::Damage(amt) => self.health -= *amt,
            Effect::Hitstun(amt) => self.state_transition_requests.add(StateTransitionRequest {
                state: HitstunState(*amt).wrap(),
                insert_priority: usize::MAX,
            }),
            Effect::Blockstun(amt) => self.state_transition_requests.add(StateTransitionRequest {
                state: BlockstunState(*amt).wrap(),
                insert_priority: usize::MAX,
            }),
            Effect::Knockback(amount) => {
                self.velocity.x = match self.facing {
                    Facing::Left => *amount,
                    Facing::Right => -*amount,
                };
            }
            Effect::StateChange(_) => todo!(),
        }
    }
}

#[derive(Clone)]
pub struct Attack {
    has_hit_player: bool,
    hit_effect: HitEffect,
}

#[derive(Debug, Clone)]
pub struct Hurtbox {
    rect: Rect,
    tag: String,
}

#[derive(Debug, Clone)]
pub struct Hitbox {
    rect: Rect,
    tag: String,
}

#[derive(Clone)]
pub struct Animator {
    current_animation: &'static Animation,
    current_frame: usize,
    last_change_frame: Frame,
    state_after_animation: Option<CharacterState>,
    update_hitboxes: bool,
}

impl Animator {
    pub fn new(current_frame: Frame, initial_animation: &'static Animation) -> Self {
        Self {
            current_animation: initial_animation,
            current_frame: 0,
            last_change_frame: current_frame,
            state_after_animation: None,
            update_hitboxes: true,
        }
    }

    pub fn switch_animation(
        &mut self,
        frame: Frame,
        new_animation: &'static Animation,
        state_after: Option<CharacterState>,
    ) {
        self.current_animation = new_animation;
        self.current_frame = 0;
        self.last_change_frame = frame;
        self.state_after_animation = state_after;
        self.update_hitboxes = true;
    }

    pub fn make_sure_animation(
        &mut self,
        frame: Frame,
        animation: &'static Animation,
        state_after: Option<CharacterState>,
    ) {
        if !std::ptr::eq(self.current_animation, animation) {
            self.switch_animation(frame, animation, state_after);
        }
    }

    pub fn pre_tick(&mut self, current_frame: Frame) -> Option<StateTransitionRequest> {
        if self.current_frame + 1 >= self.current_animation.frame_data.len()
            && current_frame.since_without_freeze(self.last_change_frame)
                >= self.current_animation.frame_data[self.current_frame].delay
        {
            self.state_after_animation
                .take()
                .map(|state| StateTransitionRequest::new(state, usize::MAX))
        } else {
            None
        }
    }

    pub fn tick(
        &mut self,
        facing: Facing,
        hitboxes: &mut Vec<Hitbox>,
        hurtboxes: &mut Vec<Hurtbox>,
        current_frame: Frame,
    ) {
        if current_frame.since_without_freeze(self.last_change_frame)
            >= self.current_animation.frame_data[self.current_frame].delay
        {
            self.last_change_frame = current_frame;
            self.current_frame += 1;
            if self.current_frame >= self.current_animation.frame_data.len() {
                self.current_frame = 0;
            }

            self.update_hitboxes = true;
        }

        if self.update_hitboxes {
            hitboxes.clear();
            hurtboxes.clear();

            let cur_frame = &self.current_animation.frame_data[self.current_frame];
            let flip = facing == Facing::Left;

            for (id, pos) in &cur_frame.hitboxes {
                if !pos.enabled {
                    continue;
                }
                let mut rect = Rect::new(pos.pos, pos.pos + pos.size);
                if flip {
                    let min_x = rect.min.x;
                    let max_x = rect.max.x;

                    rect.min.x = -max_x;
                    rect.max.x = -min_x;
                }
                let hitbox_info = &self.current_animation.hitboxes[id];
                if hitbox_info.is_hurtbox {
                    hurtboxes.push(Hurtbox {
                        rect,
                        tag: hitbox_info.tag.clone(),
                    })
                } else {
                    hitboxes.push(Hitbox {
                        rect,
                        tag: hitbox_info.tag.clone(),
                    })
                }
            }
        }
    }
}

pub const ROLLBACK_WINDOW: usize = 12;

pub struct GameRunner {
    info: GameInfo,
    state: GameState,
    session: P2PSession<GGRSConfig>,
    skip_frames: u32,
    waiting_for_network: bool,
}

struct GGRSConfig;

impl ggrs::Config for GGRSConfig {
    type Input = BoxedInput;

    type State = GameState;

    type Address = SocketAddr;
}

impl GameRunner {
    pub fn new(player_side: Option<PlayerSide>, info: GameInfo, state: GameState) -> Self {
        let p1_port = 42343;
        let p2_port = 54834;

        let local_port = match player_side {
            Some(PlayerSide::Player1) => p1_port,
            Some(PlayerSide::Player2) => p2_port,
            None => 54342,
        };
        let remote_port = match player_side {
            Some(PlayerSide::Player1) => p2_port,
            Some(PlayerSide::Player2) => p1_port,
            None => 54342,
        };

        let remote_addr: SocketAddr =
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), remote_port));

        println!("Binding to local port {local_port}");
        println!("Remote address is {remote_addr}");

        let socket = UdpNonBlockingSocket::bind_to_port(local_port).unwrap();

        let (p1, p2) = match player_side {
            Some(PlayerSide::Player1) => (
                ggrs::PlayerType::Local,
                ggrs::PlayerType::Remote(remote_addr),
            ),
            Some(PlayerSide::Player2) => (
                ggrs::PlayerType::Remote(remote_addr),
                ggrs::PlayerType::Local,
            ),
            None => (ggrs::PlayerType::Local, ggrs::PlayerType::Local),
        };

        println!("{p1:?} -- {p2:?}");

        let mut session = ggrs::SessionBuilder::<GGRSConfig>::new()
            .with_fps(FPS)
            .unwrap()
            .with_desync_detection_mode(ggrs::DesyncDetection::On { interval: 10 })
            .with_num_players(2)
            .with_disconnect_timeout(Duration::from_secs_f32(30.0))
            .add_player(p1, 0)
            .unwrap()
            .add_player(p2, 1)
            .unwrap()
            .start_p2p_session(socket)
            .unwrap();

        Self {
            info,
            state,
            session,
            skip_frames: 0,
            waiting_for_network: false,
        }
    }

    pub fn input(&mut self, input: impl Into<Input>, pressed: bool) {
        let input = input.into();

        let players = [&mut self.info.player_1, &mut self.info.player_2];

        for player in players {
            match player {
                PlayerType::Local {
                    mapping,
                    input: boxed_input,
                } => {
                    if let Some(action) = mapping.get_action(input) {
                        if pressed {
                            boxed_input.press(action);
                        } else {
                            boxed_input.release(action);
                        }
                    }
                }
                PlayerType::Remote => {}
            }
        }
    }

    pub fn tick(&mut self, canvas: &mut Canvas<Window>) {
        for (handle, player) in [&self.info.player_1, &self.info.player_2]
            .into_iter()
            .enumerate()
        {
            match player {
                PlayerType::Local { input, .. } => {
                    if self.session.local_player_handles().contains(&handle) {
                        self.session.add_local_input(handle, *input).unwrap();
                    }
                }
                PlayerType::Remote => {}
            }
        }

        for event in self.session.events() {
            match event {
                ggrs::GGRSEvent::Synchronizing { addr, total, count } => {
                    println!("Synchronizing...");
                }
                ggrs::GGRSEvent::Synchronized { addr } => {
                    println!("Synchronized");
                    self.waiting_for_network = false;
                }
                ggrs::GGRSEvent::Disconnected { addr } => {
                    println!("Disconnected");
                }
                ggrs::GGRSEvent::NetworkInterrupted {
                    addr,
                    disconnect_timeout,
                } => {
                    println!("Network interrupted");
                    self.waiting_for_network = true;
                }
                ggrs::GGRSEvent::NetworkResumed { addr } => {
                    println!("Network resumed");
                    self.waiting_for_network = false;
                }
                ggrs::GGRSEvent::WaitRecommendation { skip_frames } => {
                    self.skip_frames += skip_frames;
                    println!("Wait recommendation: {skip_frames}");
                }
                ggrs::GGRSEvent::DesyncDetected {
                    frame,
                    local_checksum,
                    remote_checksum,
                    addr,
                } => {
                    println!("Desync detected");
                }
            }
        }

        if self.session.current_state() == SessionState::Synchronizing {
            self.waiting_for_network = true;
        }

        if self.skip_frames > 0 {
            self.skip_frames -= 1;
            return;
        }
        self.info.waiting_for_network = self.waiting_for_network;

        if !self.waiting_for_network {
            let reqs = match self.session.advance_frame() {
                Ok(reqs) => {
                    self.info.waiting_for_network = false;
                    Some(reqs)
                },
                Err(GGRSError::PredictionThreshold) => {
                    self.info.waiting_for_network = true;
                    None
                },
                e => Some(e.unwrap()),
            };

            if let Some(reqs) = reqs {
                for req in reqs {
                    match req {
                        ggrs::GGRSRequest::SaveGameState { cell, frame } => {
                            // println!("Save");
                            cell.save(frame, Some(self.state.clone()), Some({ 0 }))
                        }
                        ggrs::GGRSRequest::LoadGameState { cell, frame } => {
                            // println!("Load");
                            self.state = cell.load().unwrap()
                        }
                        ggrs::GGRSRequest::AdvanceFrame { inputs } => {
                            // println!("Advance");
                            let p1_input = inputs[0].0;
    
                            self.state.player_input(PlayerSide::Player1, p1_input);
    
                            if inputs.len() > 1 {
                                let p2_input = inputs[1].0;
                                self.state.player_input(PlayerSide::Player2, p2_input);
                            }
    
                            self.state.tick(&self.info);
                        }
                    }
                }
            }
        } else {
            self.session.poll_remote_clients();
        }
        self.state.render(&self.info, canvas);
    }
}

impl crate::state::State for GameRunner {
    fn on_enter(&mut self, ctx: &mut crate::app_context::AppContext) {
        todo!()
    }

    fn on_suspend(&mut self, ctx: &mut crate::app_context::AppContext) {
        todo!()
    }

    fn on_resume(&mut self, ctx: &mut crate::app_context::AppContext) {
        todo!()
    }

    fn on_exit(&mut self, ctx: &mut crate::app_context::AppContext) {
        todo!()
    }

    fn event(&mut self, event: sdl2::event::Event, ctx: &mut crate::app_context::AppContext) {
        todo!()
    }

    fn tick(&mut self, ctx: &mut crate::app_context::AppContext) -> StateTransition {
        todo!()
    }

    fn render(&mut self, canvas: &mut Canvas<Window>, ctx: &mut crate::app_context::AppContext) {
        todo!()
    }
}
