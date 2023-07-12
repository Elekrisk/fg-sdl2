#![feature(type_alias_impl_trait)]
#![feature(int_roundings)]
#![feature(let_chains)]

mod app_context;
mod asset_manager;
mod fvec2;
mod game;
mod gui;
mod state;
mod updater;

use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    time::{Duration, Instant},
};

use app_context::AppContext;
use asset_manager::AssetManager;
use clap::Parser;
use game::{PlayerSide, FPS};
use ggrs::{SessionBuilder, SessionState, UdpNonBlockingSocket};
use libp2p::PeerId;
use sdl2::{event::Event, keyboard::Keycode, pixels::Color};

use crate::game::{GameInfo, GameRunner, GameState, PlayerType};

#[derive(Parser)]
struct Opts {
    player_side: Option<PlayerSide>,
}

fn mai2() {
    let local_key = libp2p::identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {local_peer_id:?}")
}

fn main() {
    let opts = Opts::parse();

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let game_controller_subsystem = sdl_context.game_controller().unwrap();
    // let ttf_subsystem = sdl2::ttf::init().unwrap();
    game_controller_subsystem
        .load_mappings("assets/controllerdb.txt")
        .unwrap();
    // let font = ttf_subsystem.load_font("assets/fonts/RobotoSlab-Regular.ttf", 20).unwrap();
    // app_context.asset_manager.insert("RobotoSlab", font);

    let window = video_subsystem
        .window("fighting game", 1280, 720)
        .resizable()
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    canvas.set_blend_mode(sdl2::render::BlendMode::Blend);

    let texture_creator = Box::leak(Box::new(canvas.texture_creator()));

    let mut app_context = AppContext {
        sdl_context: &sdl_context,
        video_subsystem: &video_subsystem,
        game_controller_subsystem: &game_controller_subsystem,
    };
    let mut game_info = GameInfo::create(texture_creator);
    let PlayerType::Local { mapping, input } = &mut game_info.player_2 else {
        unreachable!()
    };
    mapping.grab_controller(0);

    let game = GameState::new(&game_info);

    let mut runner = GameRunner::new(opts.player_side, game_info, game);

    canvas.set_draw_color(Color::RGB(0, 255, 255));
    canvas.clear();
    canvas.present();

    let mut controllers = HashMap::new();

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut then = Instant::now();

    let mut timer = Duration::new(0, 0);

    let mut then2 = Instant::now();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown {
                    timestamp,
                    window_id,
                    keycode,
                    scancode,
                    keymod,
                    repeat,
                } => {
                    runner.input(scancode.unwrap(), true);
                }
                Event::KeyUp {
                    timestamp,
                    window_id,
                    keycode,
                    scancode,
                    keymod,
                    repeat,
                } => {
                    runner.input(scancode.unwrap(), false);
                }
                Event::ControllerButtonDown {
                    timestamp,
                    which,
                    button,
                } => {
                    runner.input(button, true);
                }
                Event::ControllerButtonUp {
                    timestamp,
                    which,
                    button,
                } => {
                    runner.input(button, false);
                }
                Event::ControllerDeviceAdded { timestamp, which } => {
                    println!("Controller added: {which}");
                    controllers.insert(which, game_controller_subsystem.open(which).unwrap());
                }
                Event::ControllerDeviceRemoved { timestamp, which } => {
                    println!("Controller removed: {which}");
                    controllers.remove(&which);
                }
                _ => {}
            }
        }

        let now = Instant::now();
        let time = now.duration_since(then);
        then = now;

        timer += time;

        if timer.as_secs_f32() >= 1.0 / FPS as f32 {
            timer -= Duration::from_secs_f32(1.0 / FPS as f32);
            let now = Instant::now();
            let time = now.duration_since(then2);
            then2 = now;

            let fps = 1.0 / time.as_secs_f32();
            // println!("{fps:.1}");

            let now = Instant::now();
            runner.tick(&mut canvas);
            let time = Instant::now().duration_since(now);
            // println!("{:?}", time)
        }
    }
}
