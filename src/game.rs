use crate::behavior::component::{Interactive, RoomState};
use crate::behavior::Behavior;
use crate::board::{Bdl, BoardPosition, Direction, MapTileComponents, Position, SpriteDB};
use crate::materials::CustomMaterial1;
use crate::root::QuadCC;
use crate::tiledmap::{AtlasData, MapLayerType};
use crate::{behavior, gear, tiledmap};
use crate::{
    board::{self, BoardDataToRebuild},
    root,
};
use bevy::core_pipeline::clear_color::ClearColorConfig;
use bevy::ecs::system::SystemParam;
use bevy::render::view::RenderLayers;
use bevy::sprite::{Anchor, MaterialMesh2dBundle};
use bevy::utils::hashbrown::HashMap;
use bevy::{prelude::*, render::camera::ScalingMode};
use rand::Rng;
use std::time::Duration;

#[derive(Component)]
pub struct GCameraArena;

#[derive(Component)]
pub struct GCameraUI;

#[derive(Component, Debug)]
pub struct GameUI;

#[derive(Component, Debug)]
pub struct GameSprite;

#[derive(Component, Debug)]
pub struct GameSound {
    pub class: SoundType,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SoundType {
    BackgroundHouse,
    BackgroundStreet,
}
#[derive(Component, Debug)]
pub struct PlayerSprite {
    pub id: usize,
    pub controls: ControlKeys,
}

#[derive(Clone, Debug, Default, Event)]
pub struct RoomChangedEvent;
/// Resource to know basic stuff of the game.
#[derive(Debug, Resource)]
pub struct GameConfig {
    /// Which player should the camera and lighting follow
    pub player_id: usize,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self { player_id: 1 }
    }
}

impl PlayerSprite {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            controls: Self::default_controls(id),
        }
    }
    pub fn default_controls(id: usize) -> ControlKeys {
        match id {
            1 => ControlKeys::WASD,
            2 => ControlKeys::IJKL,
            _ => ControlKeys::NONE,
        }
    }
}

#[derive(Component, Debug)]
pub struct GhostSprite {
    pub spawn_point: BoardPosition,
    pub target_point: Option<Position>,
}

impl GhostSprite {
    pub fn new(spawn_point: BoardPosition) -> Self {
        GhostSprite {
            spawn_point,
            target_point: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ControlKeys {
    pub up: KeyCode,
    pub down: KeyCode,
    pub left: KeyCode,
    pub right: KeyCode,

    /// Interaction key (open doors, switches, etc).
    pub activate: KeyCode,
    /// Grab stuff from the ground.
    pub grab: KeyCode,
    /// Drop stuff to the ground.
    pub drop: KeyCode,
    /// Trigger the left-hand item.
    pub torch: KeyCode,
    /// Trigger the right-hand item.
    pub trigger: KeyCode,
    /// Cycle through the items on the inventory.
    pub cycle: KeyCode,
    /// Swap the left hand item with the right hand one.
    pub swap: KeyCode,
}

impl ControlKeys {
    pub const WASD: Self = ControlKeys {
        up: KeyCode::W,
        down: KeyCode::S,
        left: KeyCode::A,
        right: KeyCode::D,
        activate: KeyCode::E,
        trigger: KeyCode::R,
        torch: KeyCode::T,
        cycle: KeyCode::Q,
        swap: KeyCode::Tab,
        drop: KeyCode::G,
        grab: KeyCode::F,
    };
    pub const IJKL: Self = ControlKeys {
        up: KeyCode::I,
        down: KeyCode::K,
        left: KeyCode::J,
        right: KeyCode::L,
        activate: KeyCode::O,
        torch: KeyCode::T,
        cycle: KeyCode::Unlabeled,
        swap: KeyCode::Unlabeled,
        grab: KeyCode::Unlabeled,
        drop: KeyCode::Unlabeled,
        trigger: KeyCode::Unlabeled,
    };
    pub const NONE: Self = ControlKeys {
        up: KeyCode::Unlabeled,
        down: KeyCode::Unlabeled,
        left: KeyCode::Unlabeled,
        right: KeyCode::Unlabeled,
        activate: KeyCode::Unlabeled,
        torch: KeyCode::Unlabeled,
        cycle: KeyCode::Unlabeled,
        swap: KeyCode::Unlabeled,
        grab: KeyCode::Unlabeled,
        drop: KeyCode::Unlabeled,
        trigger: KeyCode::Unlabeled,
    };
}

pub fn setup(
    mut commands: Commands,
    qc: Query<Entity, With<GCameraArena>>,
    qc2: Query<Entity, With<GCameraUI>>,
) {
    // Despawn old camera if exists
    for cam in qc.iter() {
        commands.entity(cam).despawn_recursive();
    }
    for cam in qc2.iter() {
        commands.entity(cam).despawn_recursive();
    }
    // 2D orthographic camera - Arena
    let mut cam = Camera2dBundle::default();
    cam.projection.scaling_mode = ScalingMode::FixedVertical(200.0);
    commands
        .spawn(cam)
        .insert(GCameraArena)
        .insert(RenderLayers::from_layers(&[0, 1]));

    // 2D orthographic camera - UI
    let cam = Camera2dBundle {
        camera_2d: Camera2d {
            // no "background color", we need to see the main camera's output
            clear_color: ClearColorConfig::None,
        },
        camera: Camera {
            // renders after / on top of the main camera
            order: 1,
            ..default()
        },
        ..default()
    };
    commands
        .spawn(cam)
        .insert(GCameraUI)
        .insert(RenderLayers::from_layers(&[2, 3]));
    info!("Game camera setup");
}

pub fn cleanup(
    mut commands: Commands,
    qc: Query<Entity, With<GCameraArena>>,
    qc2: Query<Entity, With<GCameraUI>>,
    qg: Query<Entity, With<GameUI>>,
    qgs: Query<Entity, With<GameSprite>>,
    qs: Query<Entity, With<GameSound>>,
) {
    // Despawn old camera if exists
    for cam in qc.iter() {
        commands.entity(cam).despawn_recursive();
    }
    for cam in qc2.iter() {
        commands.entity(cam).despawn_recursive();
    }
    // Despawn game UI if not used
    for gui in qg.iter() {
        commands.entity(gui).despawn_recursive();
    }
    // Despawn game sprites if not used
    for gs in qgs.iter() {
        commands.entity(gs).despawn_recursive();
    }
    // Despawn game sound
    for gs in qs.iter() {
        commands.entity(gs).despawn_recursive();
    }
}

pub fn setup_ui(
    mut commands: Commands,
    handles: Res<root::GameAssets>,
    mut ev_load: EventWriter<LoadLevelEvent>,
) {
    const DEBUG_BCOLOR: BorderColor = BorderColor(Color::rgba(0.0, 1.0, 1.0, 0.0003));
    const INVENTORY_STATS_COLOR: Color = Color::rgba(0.7, 0.7, 0.7, 0.6);
    const PANEL_BGCOLOR: Color = Color::rgba(0.1, 0.1, 0.1, 0.3);
    // Spawn game UI
    commands
        .spawn(NodeBundle {
            border_color: DEBUG_BCOLOR,

            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::FlexStart,
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(1.0)),
                padding: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            ..default()
        })
        .insert(GameUI)
        .with_children(|parent| {
            // Top row (Game title)
            parent
                .spawn(NodeBundle {
                    border_color: DEBUG_BCOLOR,

                    style: Style {
                        border: UiRect::all(Val::Px(1.0)),
                        padding: UiRect::all(Val::Px(1.0)),
                        width: Val::Percent(20.0),
                        height: Val::Percent(5.0),
                        min_width: Val::Px(0.0),
                        min_height: Val::Px(16.0),
                        justify_content: JustifyContent::FlexStart,
                        align_items: AlignItems::FlexStart,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    // logo
                    parent.spawn(ImageBundle {
                        style: Style {
                            aspect_ratio: Some(130.0 / 17.0),
                            width: Val::Percent(80.0),
                            height: Val::Auto,
                            max_width: Val::Percent(80.0),
                            max_height: Val::Percent(100.0),
                            flex_shrink: 1.0,
                            ..default()
                        },
                        image: handles.images.title.clone().into(),
                        ..default()
                    });
                });

            // Main game viewport - middle
            parent.spawn(NodeBundle {
                border_color: DEBUG_BCOLOR,
                style: Style {
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(1.0)),
                    flex_grow: 1.0,
                    min_height: Val::Px(2.0),
                    ..Default::default()
                },
                ..Default::default()
            });

            // Bottom side - inventory and stats
            parent
                .spawn(NodeBundle {
                    border_color: DEBUG_BCOLOR,
                    style: Style {
                        border: UiRect::all(Val::Px(1.0)),
                        padding: UiRect::all(Val::Px(1.0)),
                        height: Val::Px(100.0),
                        width: Val::Percent(99.9),
                        flex_direction: FlexDirection::Row,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with_children(|parent| {
                    // Split for the bottom side in three regions

                    // Left side
                    parent
                        .spawn(NodeBundle {
                            border_color: DEBUG_BCOLOR,
                            style: Style {
                                border: UiRect::all(Val::Px(1.0)),
                                padding: UiRect::all(Val::Px(1.0)),
                                flex_grow: 1.0,
                                align_content: AlignContent::Center,
                                align_items: AlignItems::Center,
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .with_children(|parent| {
                            // For now a reminder of the keys:
                            let text_bundle = TextBundle::from_section(
                                "Movement: WASD - Interact: E\nToggle Aux: T - Toggle Main: R\nCycle Inv: Q - Swap: TAB",
                                TextStyle {
                                    font: handles.fonts.londrina.w100_thin.clone(),
                                    font_size: 20.0,
                                    color: INVENTORY_STATS_COLOR,
                                },
                            );

                            parent.spawn(text_bundle);
                        });

                    // Mid side
                    parent.spawn(NodeBundle {
                        border_color: DEBUG_BCOLOR,
                        style: Style {
                            border: UiRect::all(Val::Px(1.0)),
                            padding: UiRect::all(Val::Px(1.0)),
                            flex_grow: 1.0,
                            ..Default::default()
                        },
                        ..Default::default()
                    });

                    // Right side
                    parent
                        .spawn(NodeBundle {
                            border_color: DEBUG_BCOLOR,
                            background_color: BackgroundColor(PANEL_BGCOLOR),
                            style: Style {
                                border: UiRect::all(Val::Px(1.0)),
                                padding: UiRect::all(Val::Px(1.0)),
                                flex_grow: 1.0,
                                max_width: Val::Percent(33.3),
                                align_items: AlignItems::Center, // Vertical alignment
                                align_content: AlignContent::Start, // Horizontal alignment - start from the left.
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .with_children(|parent| {
                            // Right side panel - inventory
                            parent
                                .spawn(AtlasImageBundle {
                                    texture_atlas: handles.images.gear.clone(),
                                    texture_atlas_image: UiTextureAtlasImage {
                                        index: gear::GearSpriteID::Flashlight2 as usize,
                                        ..Default::default()
                                    },
                                    ..default()
                                })
                                .insert(gear::playergear::Inventory::new_left());
                            parent
                                .spawn(AtlasImageBundle {
                                    texture_atlas: handles.images.gear.clone(),
                                    texture_atlas_image: UiTextureAtlasImage {
                                        index: gear::GearSpriteID::IonMeter2 as usize,
                                        ..Default::default()
                                    },
                                    ..default()
                                })
                                .insert(gear::playergear::Inventory::new_right());
                            let mut text_bundle = TextBundle::from_section(
                                "IonDetector: ON\nReading: ION 2 - 30V/m\nBattery: 40%",
                                TextStyle {
                                    font: handles.fonts.londrina.w300_light.clone(),
                                    font_size: 26.0,
                                    color: INVENTORY_STATS_COLOR,
                                },
                            );
                            text_bundle.style = Style {
                                // width: Val::Px(200.0),
                                flex_grow: 1.0,
                                ..Default::default()
                            };
                            // text_bundle.background_color = BackgroundColor(PANEL_BGCOLOR);

                            parent.spawn(text_bundle).insert(gear::playergear::InventoryStats);
                        });
                });
        });
    info!("Game UI loaded");
    ev_load.send(LoadLevelEvent {
        map_filepath: "default.json".to_string(),
    });
}

pub fn keyboard(
    app_state: Res<State<root::State>>,
    mut app_next_state: ResMut<NextState<root::State>>,
    keyboard_input: Res<Input<KeyCode>>,
    mut camera: Query<&mut Transform, With<GCameraArena>>,
    gc: Res<GameConfig>,
    pc: Query<(&PlayerSprite, &Transform, &board::Direction), Without<GCameraArena>>,
) {
    if *app_state.get() != root::State::InGame {
        return;
    }
    if keyboard_input.just_pressed(KeyCode::Escape) {
        app_next_state.set(root::State::MainMenu);
    }
    for mut transform in camera.iter_mut() {
        for (player, p_transform, p_dir) in pc.iter() {
            if player.id != gc.player_id {
                continue;
            }
            // Camera movement
            let mut ref_point = p_transform.translation;
            let sc_dir = p_dir.to_screen_coord();
            const CAMERA_AHEAD_FACTOR: f32 = 0.11;
            ref_point.y += 20.0 + sc_dir.y * CAMERA_AHEAD_FACTOR;
            ref_point.x += sc_dir.x * CAMERA_AHEAD_FACTOR;
            ref_point.z = transform.translation.z;
            let dist = (transform.translation.distance(ref_point) - 1.0).max(0.00001);
            let mut delta = ref_point - transform.translation;
            delta.z = 0.0;
            const RED: f32 = 120.0;
            const MEAN_DIST: f32 = 120.0;
            let vector = delta.normalize() * ((dist / MEAN_DIST).powf(2.2) * MEAN_DIST);
            transform.translation += vector / RED;
        }
        if keyboard_input.pressed(KeyCode::Right) {
            transform.translation.x += 2.0;
        }
        if keyboard_input.pressed(KeyCode::Left) {
            transform.translation.x -= 2.0;
        }
        if keyboard_input.pressed(KeyCode::Up) {
            transform.translation.y += 2.0;
        }
        if keyboard_input.pressed(KeyCode::Down) {
            transform.translation.y -= 2.0;
        }
        if keyboard_input.pressed(KeyCode::NumpadAdd) {
            transform.scale.x /= 1.02;
            transform.scale.y /= 1.02;
        }
        if keyboard_input.pressed(KeyCode::NumpadSubtract) {
            transform.scale.x *= 1.02;
            transform.scale.y *= 1.02;
        }
    }
}

#[derive(SystemParam)]
pub struct CollisionHandler<'w> {
    bf: Res<'w, board::BoardData>,
}

impl<'w> CollisionHandler<'w> {
    const ENABLE_COLLISION: bool = true;
    const PILLAR_SZ: f32 = 0.3;
    const PLAYER_SZ: f32 = 0.5;

    fn delta(&self, pos: &Position) -> Vec3 {
        let bpos = pos.to_board_position();
        let mut delta = Vec3::ZERO;
        for npos in bpos.xy_neighbors(1) {
            let cf = self
                .bf
                .collision_field
                .get(&npos)
                .copied()
                .unwrap_or_default();
            if !cf.player_free && Self::ENABLE_COLLISION {
                let dpos = npos.to_position().to_vec3() - pos.to_vec3();
                let mut dapos = dpos.abs();
                dapos.x -= Self::PILLAR_SZ;
                dapos.y -= Self::PILLAR_SZ;
                dapos.x = dapos.x.max(0.0);
                dapos.y = dapos.y.max(0.0);
                let ddist = dapos.distance(Vec3::ZERO);
                if ddist < Self::PLAYER_SZ {
                    if dpos.x < 0.0 {
                        dapos.x *= -1.0;
                    }
                    if dpos.y < 0.0 {
                        dapos.y *= -1.0;
                    }
                    let fix_dist = (Self::PLAYER_SZ - ddist).powi(2);
                    let dpos_fix = dapos / (ddist + 0.000001) * fix_dist;
                    delta += dpos_fix;
                }
            }
        }
        delta
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn keyboard_player(
    keyboard_input: Res<Input<KeyCode>>,
    mut players: Query<(
        &mut board::Position,
        &mut board::Direction,
        &mut PlayerSprite,
        &mut AnimationTimer,
    )>,
    colhand: CollisionHandler,
    interactables: Query<
        (
            Entity,
            &board::Position,
            &Interactive,
            &Behavior,
            Option<&RoomState>,
        ),
        Without<PlayerSprite>,
    >,
    mut interactive_stuff: InteractiveStuff,
    mut ev_room: EventWriter<RoomChangedEvent>,
) {
    const PLAYER_SPEED: f32 = 0.04;
    const DIR_MIN: f32 = 5.0;
    const DIR_MAX: f32 = 80.0;
    const DIR_STEPS: f32 = 15.0;
    const DIR_MAG2: f32 = DIR_MAX / DIR_STEPS;
    const DIR_RED: f32 = 1.001;
    for (mut pos, mut dir, player, mut anim) in players.iter_mut() {
        let col_delta = colhand.delta(&pos);
        pos.x -= col_delta.x;
        pos.y -= col_delta.y;

        let mut d = Direction {
            dx: 0.0,
            dy: 0.0,
            dz: 0.0,
        };

        if keyboard_input.pressed(player.controls.up) {
            d.dy += 1.0;
        }
        if keyboard_input.pressed(player.controls.down) {
            d.dy -= 1.0;
        }
        if keyboard_input.pressed(player.controls.left) {
            d.dx -= 1.0;
        }
        if keyboard_input.pressed(player.controls.right) {
            d.dx += 1.0;
        }

        d = d.normalized();
        let col_delta_n = (col_delta * 100.0).clamp_length_max(1.0);
        let col_dotp = (d.dx * col_delta_n.x + d.dy * col_delta_n.y).clamp(0.0, 1.0);
        d.dx -= col_delta_n.x * col_dotp;
        d.dy -= col_delta_n.y * col_dotp;

        let delta = d / 0.1 + dir.normalized() / DIR_MAG2 / 1000.0;
        let dscreen = delta.to_screen_coord();
        anim.set_range(CharacterAnimation::from_dir(dscreen.x, dscreen.y * 2.0).to_vec());

        // d.dx /= 1.5; // Compensate for the projection

        pos.x += PLAYER_SPEED * d.dx;
        pos.y += PLAYER_SPEED * d.dy;
        dir.dx += DIR_MAG2 * d.dx;
        dir.dy += DIR_MAG2 * d.dy;

        let dir_dist = (dir.dx.powi(2) + dir.dy.powi(2)).sqrt();
        if dir_dist > DIR_MAX {
            dir.dx *= DIR_MAX / dir_dist;
            dir.dy *= DIR_MAX / dir_dist;
        } else if dir_dist > DIR_MIN {
            dir.dx /= DIR_RED;
            dir.dy /= DIR_RED;
        }

        // ----
        if keyboard_input.just_pressed(player.controls.activate) {
            // let d = dir.normalized();
            let mut max_dist = 1.4;
            let mut selected_entity = None;
            for (entity, item_pos, interactive, behavior, _) in interactables.iter() {
                let cp_delta = interactive.control_point_delta(behavior);
                // let old_dist = pos.delta(*item_pos);
                let item_pos = Position {
                    x: item_pos.x + cp_delta.x,
                    y: item_pos.y + cp_delta.y,
                    z: item_pos.z + cp_delta.z,
                    global_z: item_pos.global_z,
                };
                let new_dist = pos.delta(item_pos);
                // let new_dist_norm = new_dist.normalized();
                // let dot_p = (new_dist_norm.dx * -d.dx + new_dist_norm.dy * -d.dy).clamp(0.0, 1.0);
                // let dref = new_dist + (&d * (new_dist.distance().min(1.0) * dot_p));
                let dref = new_dist;
                let dist = dref.distance();
                // if dist < 1.5 {
                //     dbg!(cp_delta, old_dist, new_dist, dref, dist);
                // }
                if dist < max_dist {
                    max_dist = dist + 0.00001;
                    selected_entity = Some(entity);
                }
            }
            if let Some(entity) = selected_entity {
                for (entity, item_pos, interactive, behavior, rs) in
                    interactables.iter().filter(|(e, _, _, _, _)| *e == entity)
                {
                    if interactive_stuff.execute_interaction(
                        entity,
                        item_pos,
                        Some(interactive),
                        behavior,
                        rs,
                        InteractionExecutionType::ChangeState,
                    ) {
                        ev_room.send(RoomChangedEvent);
                    }
                }
            }
        }
    }
}

#[derive(SystemParam)]
pub struct InteractiveStuff<'w, 's> {
    bf: Res<'w, board::SpriteDB>,
    commands: Commands<'w, 's>,
    materials1: ResMut<'w, Assets<CustomMaterial1>>,
    asset_server: Res<'w, AssetServer>,
    roomdb: ResMut<'w, board::RoomDB>,
}

impl<'w, 's> InteractiveStuff<'w, 's> {
    fn execute_interaction(
        &mut self,
        entity: Entity,
        item_pos: &Position,
        interactive: Option<&Interactive>,
        behavior: &Behavior,
        room_state: Option<&RoomState>,
        ietype: InteractionExecutionType,
    ) -> bool {
        let item_bpos = item_pos.to_board_position();
        let tuid = behavior.key_tuid();
        let cvo = behavior.key_cvo();
        let mut e_commands = self.commands.get_entity(entity).unwrap();
        for other_tuid in self.bf.cvo_idx.get(&cvo).unwrap().iter() {
            if *other_tuid == tuid {
                continue;
            }
            let other = self.bf.map_tile.get(other_tuid).unwrap();

            let mut beh = other.behavior.clone();
            beh.flip(behavior.p.flip);

            // In case it is connected to a room, we need to change room state.
            if let Some(room_state) = room_state {
                let item_roombpos = BoardPosition {
                    x: item_bpos.x + room_state.room_delta.x,
                    y: item_bpos.y + room_state.room_delta.y,
                    z: item_bpos.z + room_state.room_delta.z,
                };
                let room_name = self
                    .roomdb
                    .room_tiles
                    .get(&item_roombpos)
                    .cloned()
                    .unwrap_or_default();
                dbg!(&room_state, &item_roombpos);
                dbg!(&room_name);
                match ietype {
                    InteractionExecutionType::ChangeState => {
                        if let Some(main_room_state) = self.roomdb.room_state.get_mut(&room_name) {
                            *main_room_state = beh.state();
                        }
                    }
                    InteractionExecutionType::ReadRoomState => {
                        if let Some(main_room_state) = self.roomdb.room_state.get(&room_name) {
                            if *main_room_state != beh.state() {
                                continue;
                            }
                        }
                    }
                }
            }

            match other.bundle.clone() {
                Bdl::Mmb(b) => {
                    let mat = self.materials1.get(b.material).unwrap().clone();
                    let mat = self.materials1.add(mat);

                    e_commands.insert(mat);
                }
                Bdl::Sb(b) => {
                    e_commands.insert(b);
                }
            };

            e_commands.insert(beh);
            if ietype == InteractionExecutionType::ChangeState {
                if let Some(interactive) = interactive {
                    let sound_file = interactive.sound_for_moving_into_state(&other.behavior);
                    self.commands.spawn(AudioBundle {
                        source: self.asset_server.load(sound_file),
                        settings: PlaybackSettings {
                            mode: bevy::audio::PlaybackMode::Once,
                            volume: bevy::audio::Volume::Relative(bevy::audio::VolumeLevel::new(
                                1.0,
                            )),
                            speed: 1.0,
                            paused: false,
                            spatial: false,
                        },
                    });
                }
            }

            return true;
        }
        false
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CharacterAnimationDirection {
    NN,
    NW,
    WW,
    SW,
    SS,
    SE,
    EE,
    NE,
}

impl CharacterAnimationDirection {
    fn from_dir(dx: f32, dy: f32) -> Self {
        let dst = (dx * dx + dy * dy).sqrt() + 0.0000000001;
        let dx = (dx / dst).round() as i32;
        let dy = (dy / dst).round() as i32;
        match dx {
            1 => match dy {
                1 => Self::NE,
                -1 => Self::SE,
                _ => Self::EE,
            },
            0 => match dy {
                1 => Self::NN,
                -1 => Self::SS,
                _ => Self::SS,
            },
            -1 => match dy {
                1 => Self::NW,
                -1 => Self::SW,
                _ => Self::WW,
            },
            _ => Self::EE,
        }
    }
    fn to_delta_idx(self) -> usize {
        match self {
            Self::NN => 0,
            Self::NW => 1,
            Self::WW => 2,
            Self::SW => 3,
            Self::SS => 16,
            Self::SE => 17,
            Self::EE => 18,
            Self::NE => 19,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CharacterAnimationState {
    Standing,
    Walking,
}

impl CharacterAnimationState {
    fn to_delta_idx(self) -> usize {
        match self {
            Self::Standing => 32,
            Self::Walking => 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CharacterAnimation {
    state: CharacterAnimationState,
    dir: CharacterAnimationDirection,
}

impl CharacterAnimation {
    fn from_dir(dx: f32, dy: f32) -> Self {
        let dst = (dx * dx + dy * dy).sqrt() + 0.001;
        let state = if dst > 1.0 {
            CharacterAnimationState::Walking
        } else {
            CharacterAnimationState::Standing
        };
        let dir = CharacterAnimationDirection::from_dir(dx, dy);
        Self { state, dir }
    }
    fn to_vec(self) -> Vec<usize> {
        let i = self.state.to_delta_idx() + self.dir.to_delta_idx();
        vec![i, i + 4, i + 8, i + 12]
    }
}

#[derive(Component)]
pub struct AnimationTimer {
    timer: Timer,
    // range: RangeInclusive<usize>,
    frames: Vec<usize>,
    idx: usize,
}

impl AnimationTimer {
    pub fn from_range<I: IntoIterator<Item = usize>>(timer: Timer, range: I) -> Self {
        let frames: Vec<usize> = range.into_iter().collect();
        AnimationTimer {
            timer,
            frames,
            idx: 0,
        }
    }
    pub fn set_range<I: IntoIterator<Item = usize>>(&mut self, range: I) {
        self.frames = range.into_iter().collect();
    }
    pub fn tick(&mut self, delta: Duration) -> Option<usize> {
        self.timer.tick(delta);
        if !self.timer.just_finished() {
            return None;
        }
        self.idx = (self.idx + 1) % self.frames.len();
        Some(self.frames[self.idx])
    }
}

pub fn animate_sprite(
    time: Res<Time>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    mut query: Query<(
        &mut AnimationTimer,
        &mut TextureAtlasSprite,
        &Handle<TextureAtlas>,
    )>,
) {
    for (mut anim, mut sprite, texture_atlas_handle) in query.iter_mut() {
        if let Some(idx) = anim.tick(time.delta()) {
            let texture_atlas = texture_atlases.get(texture_atlas_handle).unwrap();
            sprite.index = idx;
            if sprite.index >= texture_atlas.textures.len() {
                error!(
                    "sprite index {} out of range [0..{}]",
                    sprite.index,
                    texture_atlas.textures.len()
                );
            }
        }
    }
}

pub fn player_coloring(
    mut players: Query<(&mut TextureAtlasSprite, &PlayerSprite, &board::Position)>,
    bf: Res<board::BoardData>,
) {
    for (mut tas, player, position) in players.iter_mut() {
        let color: Color = match player.id {
            1 => Color::WHITE,
            2 => Color::GOLD,
            _ => Color::ORANGE_RED,
        };
        let bpos = position.to_board_position();
        // mapping of... distance vs rel_lux
        let mut tot_rel_lux = 0.0000001;
        let mut n_rel_lux = 0.0000001;
        for npos in bpos.xy_neighbors(2) {
            if let Some(lf) = bf.light_field.get(&npos) {
                let npos = npos.to_position();
                let dist = npos.distance(position);
                let f = (1.0 - dist).clamp(0.0, 1.0);
                let rel_lux = lf.lux / bf.current_exposure;
                n_rel_lux += f;
                tot_rel_lux += rel_lux * f;
            }
        }
        let rel_lux = tot_rel_lux / n_rel_lux;
        tas.color = board::compute_color_exposure(rel_lux, 0.0, board::DARK_GAMMA, color);
    }
}

#[derive(Debug, Clone, Event)]
pub struct LoadLevelEvent {
    map_filepath: String,
}

#[allow(clippy::too_many_arguments)]
pub fn load_level(
    mut ev: EventReader<LoadLevelEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut bf: ResMut<board::BoardData>,
    mut materials1: ResMut<Assets<CustomMaterial1>>,
    qgs: Query<Entity, With<GameSprite>>,
    mut ev_room: EventWriter<RoomChangedEvent>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut tilesetdb: ResMut<tiledmap::MapTileSetDb>,
    mut sdb: ResMut<SpriteDB>,
    handles: Res<root::GameAssets>,
    mut roomdb: ResMut<board::RoomDB>,
) {
    let Some(load_event) = ev.read().next() else {
        return;
    };

    for gs in qgs.iter() {
        commands.entity(gs).despawn_recursive();
    }
    // TODO: Ambient temp should probably come from either the map or be influenced by weather.
    bf.ambient_temp = 6.0;

    // Remove all pre-existing data for environment
    bf.temperature_field.clear();

    commands
        .spawn(AudioBundle {
            source: asset_server.load("sounds/background-noise-house-1.ogg"),
            settings: PlaybackSettings {
                mode: bevy::audio::PlaybackMode::Loop,
                volume: bevy::audio::Volume::Relative(bevy::audio::VolumeLevel::new(0.00001)),
                speed: 1.0,
                paused: false,
                spatial: false,
            },
        })
        .insert(GameSound {
            class: SoundType::BackgroundHouse,
        });
    commands
        .spawn(AudioBundle {
            source: asset_server.load("sounds/ambient-clean.ogg"),
            settings: PlaybackSettings {
                mode: bevy::audio::PlaybackMode::Loop,
                volume: bevy::audio::Volume::Relative(bevy::audio::VolumeLevel::new(0.00001)),
                speed: 1.0,
                paused: false,
                spatial: false,
            },
        })
        .insert(GameSound {
            class: SoundType::BackgroundStreet,
        });
    dbg!(&load_event.map_filepath);
    commands.init_resource::<board::BoardData>();

    info!("Load Level");

    // ---------- NEW MAP LOAD ----------
    let (_map, layers) = tiledmap::bevy_load_map(
        "assets/maps/map_house1_3x.tmx",
        &asset_server,
        &mut texture_atlases,
        &mut tilesetdb,
    );
    let mut player_spawn_points: Vec<board::Position> = vec![];
    let mut ghost_spawn_points: Vec<board::Position> = vec![];

    let mut mesh_tileset = HashMap::<String, Handle<Mesh>>::new();
    sdb.clear();

    // Load the tileset sprites first:
    for (tset_name, tileset) in tilesetdb.db.iter() {
        for (tileuid, tiled_tile) in tileset.tileset.tiles() {
            let anchor = Anchor::Custom(Vec2::new(0.0, tileset.y_anchor));
            let sprite_config =
                behavior::SpriteConfig::from_tiled_auto(tset_name.clone(), tileuid, &tiled_tile);
            let behavior = behavior::Behavior::from_config(sprite_config);
            let visibility = if behavior.p.display.disable {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
            let bundle = match &tileset.data {
                AtlasData::Sheet((handle, cmat)) => {
                    let mut cmat = cmat.clone();
                    let tatlas = texture_atlases.get(handle).unwrap();
                    let mesh_handle = mesh_tileset
                        .entry(tset_name.to_string())
                        .or_insert_with(|| {
                            let sprite_size = Vec2::new(
                                tatlas.size.x / cmat.data.sheet_cols as f32 * 1.005,
                                tatlas.size.y / cmat.data.sheet_rows as f32 * 1.005,
                            );
                            let sprite_anchor = Vec2::new(
                                sprite_size.x / 2.0,
                                sprite_size.y * (0.5 - tileset.y_anchor),
                            );
                            let base_quad = Mesh::from(QuadCC::new(sprite_size, sprite_anchor));
                            meshes.add(base_quad)
                        })
                        .clone();

                    cmat.data.sheet_idx = tileuid;
                    let mat = materials1.add(cmat);
                    let transform = Transform::from_xyz(-10000.0, -10000.0, -1000.0);
                    Bdl::Mmb(MaterialMesh2dBundle {
                        mesh: mesh_handle.into(),
                        material: mat.clone(),
                        transform,
                        visibility,
                        ..Default::default()
                    })
                }
                AtlasData::Tiles(v_img) => Bdl::Sb(SpriteBundle {
                    texture: v_img[tileuid as usize].0.clone(),
                    sprite: Sprite {
                        anchor,
                        ..default()
                    },
                    visibility,
                    transform: Transform::from_xyz(-10000.0, -10000.0, -1000.0),
                    ..default()
                }),
            };

            let key_tuid = behavior.key_tuid();
            sdb.cvo_idx
                .entry(behavior.key_cvo())
                .or_default()
                .push(key_tuid.clone());

            let mt = MapTileComponents { bundle, behavior };
            sdb.map_tile.insert(key_tuid, mt);
        }
    }
    // ----

    // We will need a 2nd pass load to sync some data
    // ----
    let mut c: f32 = 0.0;
    for maptiles in layers.iter().filter_map(|(_, layer)| {
        // filter only the tile layers and extract that directly
        if let MapLayerType::Tiles(tiles) = &layer.data {
            Some(tiles)
        } else {
            None
        }
    }) {
        for tile in &maptiles.v {
            let mt = sdb
                .map_tile
                .get(&(tile.tileset.clone(), tile.tileuid))
                .expect("Map references non-existent tileset+tileuid");
            // Spawn the base entity
            let mut entity = match &mt.bundle {
                Bdl::Mmb(b) => {
                    let mut b = b.clone();
                    if tile.flip_x {
                        b.transform.scale.x = -1.0;
                    }
                    let mat = materials1.get(b.material).unwrap().clone();
                    let mat = materials1.add(mat);

                    b.material = mat;
                    commands.spawn(b)
                }
                Bdl::Sb(b) => {
                    let mut b = b.clone();
                    if tile.flip_x {
                        b.transform.scale.x = -1.0;
                    }
                    commands.spawn(b.clone())
                }
            };

            let mut pos = board::Position {
                x: tile.pos.x as f32,
                y: -tile.pos.y as f32,
                z: 0.0,
                global_z: 0.0,
            };

            c += 0.000000001;
            pos.global_z = f32::from(mt.behavior.p.display.global_z) + c;
            match &mt.behavior.p.util {
                behavior::Util::PlayerSpawn => {
                    player_spawn_points.push(Position {
                        global_z: 0.0001,
                        ..pos
                    });
                }
                behavior::Util::GhostSpawn => {
                    ghost_spawn_points.push(Position {
                        global_z: 0.0001,
                        ..pos
                    });
                }
                behavior::Util::RoomDef(name) => {
                    roomdb
                        .room_tiles
                        .insert(pos.to_board_position(), name.to_owned());
                    roomdb.room_state.insert(name.clone(), behavior::State::Off);
                }
                behavior::Util::Van => {}
                behavior::Util::None => {}
            }
            mt.behavior.default_components(&mut entity);
            let mut beh = mt.behavior.clone();
            beh.flip(tile.flip_x);

            entity.insert(beh).insert(GameSprite).insert(pos);
        }
    }

    use rand::seq::SliceRandom;
    use rand::thread_rng;
    player_spawn_points.shuffle(&mut thread_rng());
    if player_spawn_points.is_empty() {
        error!("No player spawn points found!! - that will probably not display the map because the player will be out of bounds");
    }
    // Spawn Player 1
    commands
        .spawn(SpriteSheetBundle {
            texture_atlas: handles.images.character1.clone(),
            sprite: TextureAtlasSprite {
                anchor: Anchor::Custom(handles.anchors.grid1x1x4),
                ..Default::default()
            },
            transform: Transform::from_xyz(-1000.0, -1000.0, -1000.0)
                .with_scale(Vec3::new(0.5, 0.5, 0.5)),
            ..default()
        })
        .insert(GameSprite)
        .insert(gear::playergear::PlayerGear::new())
        .insert(PlayerSprite::new(1))
        .insert(player_spawn_points.pop().unwrap())
        .insert(board::Direction::default())
        .insert(AnimationTimer::from_range(
            Timer::from_seconds(0.20, TimerMode::Repeating),
            CharacterAnimation::from_dir(0.5, 0.5).to_vec(),
        ));

    // Spawn Player 2
    // commands
    //     .spawn(SpriteSheetBundle {
    //         texture_atlas: handles.images.character1.clone(),
    //         sprite: TextureAtlasSprite {
    //             anchor: TileSprite::Character.anchor(&tb),
    //             ..Default::default()
    //         },
    //         ..default()
    //     })
    //     .insert(GameSprite)
    //     .insert(PlayerSprite::new(2))
    //     .insert(board::Direction::default())
    //     .insert(Position::new_i64(1, 0, 0).into_global_z(0.0005))
    //     .insert(AnimationTimer::from_range(
    //         Timer::from_seconds(0.20, TimerMode::Repeating),
    //         OldCharacterAnimation::Walking.animation_range(),
    //     ));

    ghost_spawn_points.shuffle(&mut thread_rng());
    if ghost_spawn_points.is_empty() {
        error!("No ghost spawn points found!! - that will probably break the gameplay as the ghost will spawn out of bounds");
    }
    let ghost_spawn = ghost_spawn_points.pop().unwrap();
    commands
        .spawn(SpriteBundle {
            texture: asset_server.load("img/ghost.png"),
            transform: Transform::from_xyz(-1000.0, -1000.0, -1000.0),
            sprite: Sprite {
                anchor: Anchor::Custom(handles.anchors.grid1x1x4),
                ..default()
            },
            ..default()
        })
        .insert(GameSprite)
        .insert(GhostSprite::new(ghost_spawn.to_board_position()))
        .insert(ghost_spawn);

    ev_room.send(RoomChangedEvent);
}

pub fn roomchanged_event(
    mut ev_bdr: EventWriter<BoardDataToRebuild>,
    mut ev_room: EventReader<RoomChangedEvent>,
    mut interactive_stuff: InteractiveStuff,
    interactables: Query<(Entity, &board::Position, &Behavior, &RoomState), Without<PlayerSprite>>,
) {
    if ev_room.read().next().is_none() {
        return;
    }

    for (entity, item_pos, behavior, room_state) in interactables.iter() {
        let changed = interactive_stuff.execute_interaction(
            entity,
            item_pos,
            None,
            behavior,
            Some(room_state),
            InteractionExecutionType::ReadRoomState,
        );

        if changed {
            // dbg!(&behavior);
        }
    }
    ev_bdr.send(BoardDataToRebuild {
        lighting: true,
        collision: true,
    });
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum InteractionExecutionType {
    ChangeState,
    ReadRoomState,
}

pub fn ghost_movement(
    mut q: Query<(&mut GhostSprite, &mut Position)>,
    roomdb: Res<board::RoomDB>,
    bf: Res<board::BoardData>,
) {
    for (mut ghost, mut pos) in q.iter_mut() {
        if let Some(target_point) = ghost.target_point {
            let mut delta = target_point.delta(*pos);
            let dlen = delta.distance();
            if dlen > 1.0 {
                delta.dx /= dlen.sqrt();
                delta.dy /= dlen.sqrt();
            }
            pos.x += delta.dx / 200.0;
            pos.y += delta.dy / 200.0;
            if dlen < 0.5 {
                ghost.target_point = None;
            }
        } else {
            let mut target_point = ghost.spawn_point.to_position();
            let mut rng = rand::thread_rng();
            let wander: f32 = rng.gen_range(0.0..1.0_f32).powf(6.0) * 12.0 + 0.5;
            let dx: f32 = (0..5).map(|_| rng.gen_range(-1.0..1.0)).sum();
            let dy: f32 = (0..5).map(|_| rng.gen_range(-1.0..1.0)).sum();
            let dist: f32 = (0..5).map(|_| rng.gen_range(0.2..wander)).sum();
            let dd = (dx * dx + dy * dy).sqrt() / dist;

            target_point.x = (target_point.x + pos.x * wander) / (1.0 + wander) + dx / dd;
            target_point.y = (target_point.y + pos.y * wander) / (1.0 + wander) + dy / dd;

            let bpos = target_point.to_board_position();
            if roomdb.room_tiles.get(&bpos).is_some()
                && bf
                    .collision_field
                    .get(&bpos)
                    .map(|x| x.ghost_free)
                    .unwrap_or_default()
            {
                ghost.target_point = Some(target_point);
            }
        }
    }
}
