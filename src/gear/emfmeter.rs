use bevy::prelude::*;
use rand::Rng as _;

use crate::board::Position;

use super::{on_off, playergear::EquipmentPosition, Gear, GearKind, GearSpriteID, GearUsable};

#[derive(Debug, Clone, Default)]
pub enum EMFLevel {
    #[default]
    None,
    EMF2,
    EMF3,
    EMF4,
    EMF5,
}

impl EMFLevel {
    pub fn from_milligauss(mg: f32) -> EMFLevel {
        if mg > 20.0 {
            return EMFLevel::EMF5;
        }
        if mg > 10.0 {
            return EMFLevel::EMF4;
        }
        if mg > 2.5 {
            return EMFLevel::EMF3;
        }
        if mg > 1.5 {
            return EMFLevel::EMF2;
        }
        EMFLevel::None
    }

    pub fn to_spriteid(&self) -> GearSpriteID {
        match self {
            EMFLevel::None => GearSpriteID::EMFMeter0,
            EMFLevel::EMF2 => GearSpriteID::EMFMeter1,
            EMFLevel::EMF3 => GearSpriteID::EMFMeter2,
            EMFLevel::EMF4 => GearSpriteID::EMFMeter3,
            EMFLevel::EMF5 => GearSpriteID::EMFMeter4,
        }
    }

    pub fn to_status(&self) -> &'static str {
        match self {
            EMFLevel::None => "",
            EMFLevel::EMF2 => "EMF2",
            EMFLevel::EMF3 => "EMF3",
            EMFLevel::EMF4 => "EMF4",
            EMFLevel::EMF5 => "EMF5",
        }
    }
}

#[derive(Component, Debug, Clone, Default)]
pub struct EMFMeter {
    pub enabled: bool,
    pub frame_counter: u16,
    pub temp_l2: Vec<f32>,
    pub temp_l1: f32,
    pub emf: f32,
    pub emf_level: EMFLevel,
    pub last_sound_secs: f32,
    pub last_meter_update_secs: f32,
}

impl GearUsable for EMFMeter {
    fn get_sprite_idx(&self) -> GearSpriteID {
        match self.enabled {
            true => self.emf_level.to_spriteid(),
            false => GearSpriteID::EMFMeterOff,
        }
    }

    fn get_display_name(&self) -> &'static str {
        "EMF Meter"
    }

    fn get_status(&self) -> String {
        let name = self.get_display_name();
        let on_s = on_off(self.enabled);
        let msg = if self.enabled {
            format!(
                "Reading: {:>6.1}mG {}",
                self.emf,
                self.emf_level.to_status()
            )
        } else {
            "".to_string()
        };
        format!("{name}:  {on_s}\n{msg}")
    }

    fn set_trigger(&mut self, _gs: &mut super::GearStuff) {
        self.enabled = !self.enabled;
    }
    fn update(&mut self, gs: &mut super::GearStuff, pos: &Position, ep: &EquipmentPosition) {
        let mut rng = rand::thread_rng();
        self.frame_counter += 1;
        if self.frame_counter > 65413 {
            self.frame_counter = 0;
        }
        const K: f32 = 0.5;
        let pos = Position {
            x: pos.x + rng.gen_range(-K..K) + rng.gen_range(-K..K),
            y: pos.y + rng.gen_range(-K..K) + rng.gen_range(-K..K),
            z: pos.z + rng.gen_range(-K..K) + rng.gen_range(-K..K),
            global_z: pos.global_z,
        };
        let bpos = pos.to_board_position();
        let Some(temperature) = gs.bf.temperature_field.get(&bpos) else {
            return;
        };
        let temp_reading = temperature;
        const AIR_MASS: f32 = 5.0;

        if self.temp_l2.len() < 2 {
            self.temp_l2.push(*temp_reading);
        }

        // Double noise reduction to remove any noise from measurement.
        let n = self.frame_counter as usize % self.temp_l2.len();
        self.temp_l2[n] = (self.temp_l2[n] * AIR_MASS + temp_reading) / (AIR_MASS + 1.0);
        self.temp_l1 = (self.temp_l1 * AIR_MASS + temp_reading) / (AIR_MASS + 1.0);
        if self.temp_l2.len() < 40 {
            self.temp_l2.push(self.temp_l1);
        }

        let sec = gs.time.elapsed_seconds();
        if self.last_meter_update_secs + 0.5 < sec {
            self.last_meter_update_secs = sec;
            let sum_temp: f32 = self.temp_l2.iter().sum();
            let avg_temp: f32 = sum_temp / self.temp_l2.len() as f32;
            let new_emf = (avg_temp - self.temp_l1).abs() * 3.0;
            self.emf -= 0.2;
            self.emf /= 1.4;
            self.emf = self.emf.max(new_emf);
            self.emf_level = EMFLevel::from_milligauss(self.emf);
        }
        if self.enabled {
            let delta = 10.0 / (self.emf + 0.5).powf(1.5);
            if self.last_sound_secs + delta < sec {
                self.last_sound_secs = sec;
                match ep {
                    EquipmentPosition::Hand(_) => {
                        gs.play_audio("sounds/effects-chirp-shorter.ogg".into(), 1.0)
                    }
                    EquipmentPosition::Stowed => {
                        gs.play_audio("sounds/effects-chirp-shorter.ogg".into(), 0.5)
                    }
                }
            }
        }
    }
    fn box_clone(&self) -> Box<dyn GearUsable> {
        Box::new(self.clone())
    }
}

impl From<EMFMeter> for Gear {
    fn from(value: EMFMeter) -> Self {
        Gear::new_from_kind(GearKind::EMFMeter(value))
    }
}
