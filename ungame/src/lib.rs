pub mod game;
mod ghost;
mod ghost_events;
mod mainmenu;
mod maplight;
mod npchelp;
mod pause;
mod systems;

use bevy::prelude::*;
use uncore::utils;

pub fn app_setup(app: &mut App) {
    mainmenu::app_setup(app);
    ghost::app_setup(app);
    ghost_events::app_setup(app);
    pause::app_setup(app);
    maplight::app_setup(app);
    npchelp::app_setup(app);
    systems::object_charge::app_setup(app);
}
