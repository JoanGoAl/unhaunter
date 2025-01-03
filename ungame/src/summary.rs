use crate::{uncore_difficulty::CurrentDifficulty, player::PlayerSprite, uncore_root, utils};
use bevy::{color::palettes::css, prelude::*};
use uncore::platform::plt::{FONT_SCALE, UI_SCALE};
use uncore::types::ghost::types::GhostType;

#[derive(Debug, Component, Clone)]
pub struct SCamera;
#[derive(Debug, Component, Clone)]
pub struct SummaryUI;

#[derive(Debug, Component, Clone)]
pub enum SummaryUIType {
    GhostList,
    TimeTaken,
    GhostUnhaunted,
    RepellentUsed,
    AvgSanity,
    PlayersAlive,
    FinalScore,
}

#[derive(Debug, Clone, Resource, Default)]
pub struct SummaryData {
    pub time_taken_secs: f32,
    pub ghost_types: Vec<GhostType>,
    pub repellent_used_amt: u32,
    pub ghosts_unhaunted: u32,
    pub final_score: i64,
    pub difficulty: CurrentDifficulty,
    pub average_sanity: f32,
    pub player_count: usize,
    pub alive_count: usize,
}

impl SummaryData {
    pub fn new(ghost_types: Vec<GhostType>, difficulty: CurrentDifficulty) -> Self {
        Self {
            ghost_types,
            difficulty,
            ..default()
        }
    }

    pub fn calculate_score(&self) -> i64 {
        let mut score = (250.0 * self.ghosts_unhaunted as f64)
            / (1.0 + self.repellent_used_amt as f64)
            / (1.0 + (self.ghost_types.len() as u32 - self.ghosts_unhaunted) as f64);

        // Sanity modifier
        score *= (self.average_sanity as f64 + 30.0) / 50.0;

        // Apply difficulty multiplier
        score *= self.difficulty.0.difficulty_score_multiplier;
        if self.player_count == self.alive_count {
            // Apply time bonus multiplier
            score *= 1.0 + 360.0 / (60.0 + self.time_taken_secs as f64);
        } else {
            score *= self.alive_count as f64 / (self.player_count as f64 + 1.0);
        }

        // Ensure score is within a reasonable range
        score.clamp(0.0, 1000000.0) as i64
    }
}

pub fn setup(mut commands: Commands) {
    // ui camera
    commands.spawn(Camera2d).insert(SCamera);
    info!("Summary camera setup");
}

pub fn cleanup(
    mut commands: Commands,
    qc: Query<Entity, With<SCamera>>,
    qu: Query<Entity, With<SummaryUI>>,
) {
    // Despawn old camera if exists
    for cam in qc.iter() {
        commands.entity(cam).despawn_recursive();
    }

    // Despawn UI if not used
    for ui_entity in qu.iter() {
        commands.entity(ui_entity).despawn_recursive();
    }
}

pub fn update_time(
    time: Res<Time>,
    mut sd: ResMut<SummaryData>,
    game_state: Res<State<uncore_root::GameState>>,
    mut app_next_state: ResMut<NextState<uncore_root::State>>,
    qp: Query<&PlayerSprite>,
    difficulty: Res<CurrentDifficulty>,
) {
    if *game_state == uncore_root::GameState::Pause {
        return;
    }
    sd.difficulty = difficulty.clone();
    sd.time_taken_secs += time.delta_secs();
    let total_sanity: f32 = qp.iter().map(|x| x.sanity()).sum();
    let player_count = qp.iter().count();
    let alive_count = qp.iter().filter(|x| x.health > 0.0).count();
    sd.player_count = player_count;
    sd.alive_count = alive_count;
    if player_count > 0 {
        sd.average_sanity = total_sanity / player_count as f32;
    }
    if alive_count == 0 {
        app_next_state.set(uncore_root::State::Summary);
    }
}

pub fn keyboard(
    app_state: Res<State<uncore_root::State>>,
    mut app_next_state: ResMut<NextState<uncore_root::State>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if *app_state.get() != uncore_root::State::Summary {
        return;
    }
    if keyboard_input.just_pressed(KeyCode::Escape)
        | keyboard_input.just_pressed(KeyCode::NumpadEnter)
        | keyboard_input.just_pressed(KeyCode::Enter)
    {
        app_next_state.set(uncore_root::State::MainMenu);
    }
}
pub fn setup_ui(mut commands: Commands, handles: Res<uncore_root::GameAssets>) {
    let main_color = Color::Srgba(Srgba {
        red: 0.2,
        green: 0.2,
        blue: 0.2,
        alpha: 0.05,
    });
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            flex_direction: FlexDirection::Column,
            padding: UiRect {
                left: Val::Percent(10.0),
                right: Val::Percent(10.0),
                top: Val::Percent(5.0),
                bottom: Val::Percent(5.0),
            },
            flex_grow: 1.0,
            ..default()
        })
        .insert(BackgroundColor(main_color))
        .insert(SummaryUI)
        .with_children(|parent| {
            parent
                .spawn(Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(20.0),
                    min_width: Val::Px(0.0),
                    min_height: Val::Px(64.0 * UI_SCALE),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::FlexStart,
                    ..default()
                })
                .with_children(|parent| {
                    // logo
                    parent
                        .spawn(ImageNode {
                            image: handles.images.title.clone(),
                            ..default()
                        })
                        .insert(Node {
                            aspect_ratio: Some(130.0 / 17.0),
                            width: Val::Percent(80.0),
                            height: Val::Auto,
                            max_width: Val::Percent(80.0),
                            max_height: Val::Percent(100.0),
                            flex_shrink: 1.0,
                            ..default()
                        });
                });
            parent.spawn(Node {
                width: Val::Percent(100.0),
                height: Val::Percent(20.0),
                ..default()
            });
            parent
                .spawn(Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(60.0),
                    justify_content: JustifyContent::SpaceEvenly,
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Column,
                    ..default()
                })
                .insert(BackgroundColor(main_color))
                .with_children(|parent| {
                    // text
                    parent
                        .spawn(Text::new("Summary"))
                        .insert(TextFont {
                            font: handles.fonts.londrina.w300_light.clone(),
                            font_size: 38.0 * FONT_SCALE,
                            font_smoothing: bevy::text::FontSmoothing::AntiAliased,
                        })
                        .insert(TextColor(Color::WHITE));
                    parent
                        .spawn(Text::new("Ghost list"))
                        .insert(TextFont {
                            font: handles.fonts.londrina.w300_light.clone(),
                            font_size: 38.0 * FONT_SCALE,
                            font_smoothing: bevy::text::FontSmoothing::AntiAliased,
                        })
                        .insert(TextColor(css::GRAY.into()))
                        .insert(SummaryUIType::GhostList);
                    parent.spawn(Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(10.0),
                        ..default()
                    });
                    parent
                        .spawn(Text::new("Time taken: 00.00.00"))
                        .insert(TextFont {
                            font: handles.fonts.londrina.w300_light.clone(),
                            font_size: 38.0 * FONT_SCALE,
                            font_smoothing: bevy::text::FontSmoothing::AntiAliased,
                        })
                        .insert(TextColor(css::GRAY.into()))
                        .insert(SummaryUIType::TimeTaken);
                    parent
                        .spawn(Text::new("Average Sanity: 00"))
                        .insert(TextFont {
                            font: handles.fonts.londrina.w300_light.clone(),
                            font_size: 38.0 * FONT_SCALE,
                            font_smoothing: bevy::text::FontSmoothing::AntiAliased,
                        })
                        .insert(TextColor(css::GRAY.into()))
                        .insert(SummaryUIType::AvgSanity);
                    parent
                        .spawn(Text::new("Ghosts unhaunted: 0/1"))
                        .insert(TextFont {
                            font: handles.fonts.londrina.w300_light.clone(),
                            font_size: 38.0 * FONT_SCALE,
                            font_smoothing: bevy::text::FontSmoothing::AntiAliased,
                        })
                        .insert(TextColor(css::GRAY.into()))
                        .insert(SummaryUIType::GhostUnhaunted);
                    parent
                        .spawn(Text::new("Repellent charges used: 0"))
                        .insert(TextFont {
                            font: handles.fonts.londrina.w300_light.clone(),
                            font_size: 38.0 * FONT_SCALE,
                            font_smoothing: bevy::text::FontSmoothing::AntiAliased,
                        })
                        .insert(TextColor(css::GRAY.into()))
                        .insert(SummaryUIType::RepellentUsed);
                    parent
                        .spawn(Text::new("Players Alive: 0/0"))
                        .insert(TextFont {
                            font: handles.fonts.londrina.w300_light.clone(),
                            font_size: 38.0 * FONT_SCALE,
                            font_smoothing: bevy::text::FontSmoothing::AntiAliased,
                        })
                        .insert(TextColor(css::GRAY.into()))
                        .insert(SummaryUIType::PlayersAlive);
                    parent
                        .spawn(Text::new("Final Score: 0"))
                        .insert(TextFont {
                            font: handles.fonts.londrina.w300_light.clone(),
                            font_size: 38.0 * FONT_SCALE,
                            font_smoothing: bevy::text::FontSmoothing::AntiAliased,
                        })
                        .insert(TextColor(css::GRAY.into()))
                        .insert(SummaryUIType::FinalScore);
                    parent.spawn(Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(20.0),
                        ..default()
                    });
                    parent
                        .spawn(Text::new("[ - Press enter to continue - ]"))
                        .insert(TextFont {
                            font: handles.fonts.londrina.w300_light.clone(),
                            font_size: 38.0 * FONT_SCALE,
                            font_smoothing: bevy::text::FontSmoothing::AntiAliased,
                        })
                        .insert(TextColor(css::ORANGE_RED.into()));
                });
            parent.spawn(Node {
                width: Val::Percent(100.0),
                height: Val::Percent(20.0),
                ..default()
            });
        });
    info!("Main menu loaded");
}

pub fn update_ui(mut qui: Query<(&SummaryUIType, &mut Text)>, rsd: Res<SummaryData>) {
    for (sui, mut text) in &mut qui {
        match &sui {
            SummaryUIType::GhostList => {
                text.0 = format!(
                    "Ghost: {}",
                    rsd.ghost_types
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            SummaryUIType::TimeTaken => {
                text.0 = format!("Time taken: {}", utils::format_time(rsd.time_taken_secs))
            }
            SummaryUIType::AvgSanity => {
                text.0 = format!("Average Sanity: {:.1}%", rsd.average_sanity)
            }
            SummaryUIType::GhostUnhaunted => {
                text.0 = format!(
                    "Ghosts unhaunted: {}/{}",
                    rsd.ghosts_unhaunted,
                    rsd.ghost_types.len()
                )
            }
            SummaryUIType::PlayersAlive => {
                text.0 = format!("Players Alive: {}/{}", rsd.alive_count, rsd.player_count)
            }
            SummaryUIType::RepellentUsed => {
                text.0 = format!("Repellent charges used: {}", rsd.repellent_used_amt)
            }
            SummaryUIType::FinalScore => text.0 = format!("Final Score: {}", rsd.final_score),
        }
    }
}

pub fn update_score(mut sd: ResMut<SummaryData>, app_state: Res<State<uncore_root::State>>) {
    if *app_state != uncore_root::State::Summary {
        return;
    }
    let desired_score = sd.calculate_score();
    let max_delta = desired_score - sd.final_score;
    let delta = (max_delta / 200).max(10).min(max_delta);
    sd.final_score += delta;
}

pub fn app_setup(app: &mut App) {
    app.init_resource::<SummaryData>()
        .add_systems(OnEnter(uncore_root::State::Summary), (setup, setup_ui))
        .add_systems(OnExit(uncore_root::State::Summary), cleanup)
        .add_systems(
            FixedUpdate,
            update_time.run_if(in_state(uncore_root::State::InGame)),
        )
        .add_systems(
            Update,
            (keyboard, update_ui, update_score).run_if(in_state(uncore_root::State::Summary)),
        );
}
