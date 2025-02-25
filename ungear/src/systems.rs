use super::components::deployedgear::{DeployedGear, DeployedGearData};
use super::components::playergear::PlayerGear;
use bevy::prelude::*;
use bevy_persistent::Persistent;
use uncore::components::board::position::Position;
use uncore::components::game_config::GameConfig;
use uncore::components::player_inventory::{Inventory, InventoryNext, InventoryStats};
use uncore::components::player_sprite::PlayerSprite;
use uncore::events::sound::SoundEvent;
use uncore::systemparam::gear_stuff::GearStuff;
use uncore::traits::gear_usable::GearUsable;
use uncore::types::gear::equipmentposition::EquipmentPosition;
use unsettings::audio::AudioSettings;

/// System for updating the internal state of all gear carried by the player.
///
/// This system iterates through the player's gear and calls the `update` method
/// for each piece of gear, allowing gear to update their state based on time,
/// player actions, or environmental conditions.
pub fn update_playerheld_gear_data(
    mut q_gear: Query<(&Position, &mut PlayerGear)>,
    mut gs: GearStuff,
) {
    for (position, mut playergear) in q_gear.iter_mut() {
        for (gear, epos) in playergear.as_vec_mut().into_iter() {
            gear.update(&mut gs, position, &epos);
        }
    }
}

/// System for updating the internal state of all gear deployed in the environment.
pub fn update_deployed_gear_data(
    mut q_gear: Query<(&Position, &DeployedGear, &mut DeployedGearData)>,
    mut gs: GearStuff,
) {
    for (position, _deployed_gear, mut gear_data) in q_gear.iter_mut() {
        gear_data
            .gear
            .update(&mut gs, position, &EquipmentPosition::Deployed);
    }
}

/// System for updating the sprites of deployed gear to reflect their internal
/// state.
pub fn update_deployed_gear_sprites(mut q_gear: Query<(&mut Sprite, &DeployedGearData)>) {
    for (mut sprite, gear_data) in q_gear.iter_mut() {
        let new_index = gear_data.gear.get_sprite_idx() as usize;
        if let Some(texture_atlas) = &mut sprite.texture_atlas {
            if texture_atlas.index != new_index {
                texture_atlas.index = new_index;
            }
        }
    }
}

/// System to handle the SoundEvent, playing the sound with volume adjusted by
/// distance.
pub fn sound_playback_system(
    mut sound_events: EventReader<SoundEvent>,
    asset_server: Res<AssetServer>,
    gc: Res<GameConfig>,
    qp: Query<(&Position, &PlayerSprite)>,
    mut commands: Commands,
    audio_settings: Res<Persistent<AudioSettings>>,
) {
    for sound_event in sound_events.read() {
        // Get player position (Match against the player ID from GameConfig)
        let Some((player_position, _)) = qp.iter().find(|(_, p)| p.id == gc.player_id) else {
            return;
        };
        let adjusted_volume = match sound_event.position {
            Some(position) => {
                const MIN_DIST: f32 = 25.0;

                // Calculate distance from player to sound source
                let distance2 = player_position.distance2(&position) + MIN_DIST;
                let distance = distance2.powf(0.7) + MIN_DIST;

                // Calculate adjusted volume based on distance and audio settings
                (sound_event.volume / distance2 * MIN_DIST
                    + sound_event.volume / distance * MIN_DIST)
                    .clamp(0.0, 1.0)
            }
            None => sound_event.volume,
        };

        // Spawn an AudioBundle with the adjusted volume
        commands
            .spawn(AudioPlayer::<AudioSource>(
                asset_server.load(sound_event.sound_file.clone()),
            ))
            .insert(PlaybackSettings {
                mode: bevy::audio::PlaybackMode::Despawn,
                volume: bevy::audio::Volume::new(
                    adjusted_volume
                        * audio_settings.volume_effects.as_f32()
                        * audio_settings.volume_master.as_f32(),
                ),
                speed: 1.0,
                paused: false,
                spatial: false,
                spatial_scale: None,
            });
    }
}

pub fn keyboard_gear(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut q_gear: Query<(&PlayerSprite, &mut PlayerGear)>,
    mut gs: GearStuff,
) {
    for (ps, mut playergear) in q_gear.iter_mut() {
        if keyboard_input.just_pressed(ps.controls.cycle) {
            playergear.cycle();
        }
        if keyboard_input.just_pressed(ps.controls.swap) {
            playergear.swap();
        }
        if keyboard_input.just_released(ps.controls.trigger) {
            playergear.right_hand.set_trigger(&mut gs);
        }
        if keyboard_input.just_released(ps.controls.torch) {
            playergear.left_hand.set_trigger(&mut gs);
        }
    }
}

pub fn update_gear_ui(
    gc: Res<GameConfig>,
    q_gear: Query<(&PlayerSprite, &PlayerGear)>,
    mut qi: Query<(&Inventory, &mut ImageNode), Without<InventoryNext>>,
    mut qs: Query<&mut Text, With<InventoryStats>>,
    mut qin: Query<(&InventoryNext, &mut ImageNode), Without<Inventory>>,
) {
    for (ps, playergear) in q_gear.iter() {
        if gc.player_id == ps.id {
            for (inv, mut imgnode) in qi.iter_mut() {
                let gear = playergear.get_hand(&inv.hand);
                let idx = gear.get_sprite_idx() as usize;
                if imgnode.texture_atlas.as_ref().unwrap().index != idx {
                    imgnode.texture_atlas.as_mut().unwrap().index = idx;
                }
            }
            let right_hand_status = playergear.right_hand.get_status();
            for mut txt in qs.iter_mut() {
                if txt.0 != right_hand_status {
                    txt.0.clone_from(&right_hand_status);
                }
            }
            for (inv, mut imgnode) in qin.iter_mut() {
                // There are 2 possible "None" here, the outside Option::None for when the idx is
                // out of bounds and the inner Gear::None when a slot is empty.
                let next = if let Some(idx) = inv.idx {
                    playergear.get_next(idx).unwrap_or_default()
                } else {
                    playergear.get_next_non_empty().unwrap_or_default()
                };
                let idx = next.get_sprite_idx() as usize;
                if imgnode.texture_atlas.as_ref().unwrap().index != idx {
                    imgnode.texture_atlas.as_mut().unwrap().index = idx;
                }
            }
        }
    }
}
