use sdl2::{Sdl, VideoSubsystem, GameControllerSubsystem};

use crate::asset_manager::AssetManager;


pub struct AppContext<'a> {
    // pub asset_manager: AssetManager,
    pub sdl_context: &'a Sdl,
    pub video_subsystem: &'a VideoSubsystem,
    pub game_controller_subsystem: &'a GameControllerSubsystem,
}
