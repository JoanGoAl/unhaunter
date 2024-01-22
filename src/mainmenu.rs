use bevy::app::AppExit;
use bevy::prelude::*;

use crate::root;

const MENU_ITEM_COLOR_OFF: Color = Color::GRAY;
const MENU_ITEM_COLOR_ON: Color = Color::ORANGE_RED;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuID {
    NewGame,
    Options,
    Quit,
}

#[derive(Debug, Copy, Clone, Event)]
pub struct MenuEvent(MenuID);

#[derive(Component)]
pub struct Menu {
    pub selected: MenuID,
}

impl Menu {
    const ITEMS: [MenuID; 3] = [MenuID::NewGame, MenuID::Options, MenuID::Quit];
    pub fn item_idx(&self) -> i64 {
        for (n, item) in Menu::ITEMS.iter().enumerate() {
            if item == &self.selected {
                return n as i64;
            }
        }
        // We return zero for error which is the first item.
        error!("invalid item for item_idx - first item is assumed");
        0
    }
    pub fn idx_to_item(idx: i64) -> MenuID {
        let idx = idx.rem_euclid(Menu::ITEMS.len() as i64);
        Menu::ITEMS[idx as usize]
    }
    pub fn next_item(&mut self) {
        self.selected = Menu::idx_to_item(self.item_idx() + 1);
    }
    pub fn previous_item(&mut self) {
        self.selected = Menu::idx_to_item(self.item_idx() - 1);
    }
}

impl Default for Menu {
    fn default() -> Self {
        Self {
            selected: MenuID::NewGame,
        }
    }
}

#[derive(Component, Debug)]
pub struct MenuItem {
    identifier: MenuID,
    highlighted: bool,
}

impl MenuItem {
    pub fn new(identifier: MenuID) -> Self {
        MenuItem {
            identifier,
            highlighted: false,
        }
    }
}

#[derive(Component, Debug)]
pub struct MCamera;

#[derive(Component, Debug)]
pub struct MenuUI;

pub fn setup(mut commands: Commands) {
    // ui camera
    let cam = Camera2dBundle::default();
    commands.spawn(cam).insert(MCamera);
    info!("Main menu camera setup");
}

pub fn cleanup(mut commands: Commands, qc: Query<Entity, With<MCamera>>) {
    // Despawn old camera if exists
    for cam in qc.iter() {
        commands.entity(cam).despawn_recursive();
    }
}

#[allow(clippy::too_many_arguments)]
pub fn setup_ui(
    mut commands: Commands,
    handles: Res<root::GameAssets>,
    state: Res<State<root::State>>,
    qm: Query<Entity, With<MenuUI>>,
    _images: Res<Assets<Image>>,
    _fonts: Res<Assets<Font>>,
) {
    if *state.get() != root::State::MainMenu {
        // Despawn menu UI if not used
        for ui_entity in qm.iter() {
            commands.entity(ui_entity).despawn_recursive();
        }
        return;
    }
    if !qm.is_empty() {
        return;
    }

    let main_color = Color::Rgba {
        red: 0.2,
        green: 0.2,
        blue: 0.2,
        alpha: 0.05,
    };

    commands
        .spawn(NodeBundle {
            style: Style {
                //    align_self: AlignSelf::Center,
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

                ..default()
            },

            ..default()
        })
        .insert(MenuUI)
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Percent(20.0),
                        min_width: Val::Px(0.0),
                        min_height: Val::Px(64.0),
                        justify_content: JustifyContent::Center,
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
            parent.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(20.0),
                    ..default()
                },

                ..default()
            });

            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Percent(60.0),
                        justify_content: JustifyContent::SpaceEvenly,
                        align_items: AlignItems::Center,
                        flex_direction: FlexDirection::Column,

                        ..default()
                    },
                    background_color: main_color.into(),
                    ..default()
                })
                .insert(Menu::default())
                .with_children(|parent| {
                    // text
                    parent
                        .spawn(TextBundle::from_section(
                            "New Game",
                            TextStyle {
                                font: handles.fonts.londrina.w300_light.clone(),
                                font_size: 38.0,
                                color: MENU_ITEM_COLOR_OFF,
                            },
                        ))
                        .insert(MenuItem::new(MenuID::NewGame));
                    parent
                        .spawn(TextBundle::from_section(
                            "Options",
                            TextStyle {
                                font: handles.fonts.londrina.w300_light.clone(),
                                font_size: 38.0,
                                color: MENU_ITEM_COLOR_OFF,
                            },
                        ))
                        .insert(MenuItem::new(MenuID::Options));

                    parent
                        .spawn(TextBundle::from_section(
                            "Quit",
                            TextStyle {
                                font: handles.fonts.londrina.w300_light.clone(),
                                font_size: 38.0,
                                color: MENU_ITEM_COLOR_OFF,
                            },
                        ))
                        .insert(MenuItem::new(MenuID::Quit));
                });
            parent.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(20.0),
                    ..default()
                },

                ..default()
            });
        });
    info!("Main menu loaded");
}

pub fn item_logic(mut q: Query<(&mut MenuItem, &mut Text)>, qmenu: Query<&Menu>) {
    for (mut mitem, mut text) in q.iter_mut() {
        for menu in qmenu.iter() {
            mitem.highlighted = menu.selected == mitem.identifier;
        }
        for section in text.sections.iter_mut() {
            if mitem.highlighted {
                section.style.color = MENU_ITEM_COLOR_ON;
            } else {
                section.style.color = MENU_ITEM_COLOR_OFF;
            }
        }
    }
}

pub fn keyboard(
    keyboard_input: Res<Input<KeyCode>>,
    mut q: Query<&mut Menu>,
    mut ev_menu: EventWriter<MenuEvent>,
) {
    for mut menu in q.iter_mut() {
        if keyboard_input.just_pressed(KeyCode::Up) {
            menu.previous_item();
        } else if keyboard_input.just_pressed(KeyCode::Down) {
            menu.next_item();
        } else if keyboard_input.just_pressed(KeyCode::Return) {
            ev_menu.send(MenuEvent(menu.selected));
        }
    }
}

pub fn menu_event(
    mut ev_menu: EventReader<MenuEvent>,
    mut exit: EventWriter<AppExit>,
    mut app_next_state: ResMut<NextState<root::State>>,
) {
    for event in ev_menu.read() {
        match event.0 {
            MenuID::NewGame => app_next_state.set(root::State::InGame),
            MenuID::Options => {}
            MenuID::Quit => exit.send(AppExit),
        }
    }
}
