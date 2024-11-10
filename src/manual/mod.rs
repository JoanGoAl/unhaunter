pub mod chapter1;
pub mod chapter2;
pub mod preplay_manual_ui;
pub mod user_manual_ui;
pub mod utils;

use bevy::prelude::*;
use enum_iterator::Sequence;
pub use preplay_manual_ui::preplay_manual_system;

use crate::root::GameAssets;

// TODO: Remove ManualPageObsolete
#[derive(Debug, Clone, Copy, PartialEq, Eq, Sequence, Resource, Default)]
pub enum ManualPageObsolete {
    #[default]
    MissionBriefing,
    EssentialControls,
    EMFAndThermometer,
    TruckJournal,
    ExpellingGhost,
}

#[derive(Debug, Clone)]
pub struct ManualPageData {
    pub title: String,
    pub subtitle: String,
    pub draw_fn: fn(&mut ChildBuilder, &GameAssets),
}

#[derive(Resource, Debug, Clone)]
pub struct Manual {
    pub chapters: Vec<ManualChapter>,
}

#[derive(Debug, Clone)]
pub struct ManualChapter {
    pub pages: Vec<ManualPageData>,
    pub name: String,
}

impl ManualChapter {
    pub fn index(&self, manuals: &Manual) -> usize {
        //Find the index of `self` in manuals.chapters
        manuals
            .chapters
            .iter()
            .position(|chapter| chapter.name == self.name)
            .unwrap_or_else(|| {
                //Panic if chapter not found in manuals.chapters. This is important to detect invalid states.
                panic!("Chapter {:?} not found in manual", self.name);
            })
    }
}

pub fn create_manual() -> Manual {
    Manual {
        chapters: vec![
            chapter1::create_manual_chapter(),
            chapter2::create_manual_chapter(),
        ],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Resource, Default)]
pub struct CurrentManualPage(pub usize, pub usize); // Chapter index, Page Index

pub fn draw_manual_page(
    parent: &mut ChildBuilder,
    handles: &GameAssets,
    manual: &Manual,
    current_page: &CurrentManualPage,
) {
    let mut chapter_index = current_page.0;
    let mut page_index = current_page.1;

    // --- Chapter Bounds Check ---
    let chapter_count = manual.chapters.len();
    if chapter_index >= chapter_count {
        warn!(
            "Chapter index out of bounds: {} (max: {})",
            chapter_index,
            chapter_count - 1
        );
        chapter_index = chapter_count - 1;
    }
    let chapter = &manual.chapters[chapter_index];

    // --- Page Bounds Check ---
    let page_count = chapter.pages.len();
    if page_index >= page_count {
        warn!(
            "Page index out of bounds: {} (max: {})",
            page_index,
            page_count - 1
        );
        page_index = page_count - 1;
    }
    let page = &chapter.pages[page_index];

    // --- Draw the Page ---
    (page.draw_fn)(parent, handles);
}

// Update ManualPage enum and its methods (see next step)

pub fn app_setup(app: &mut App) {
    user_manual_ui::app_setup(app);
    preplay_manual_ui::app_setup(app);

    app.insert_resource(create_manual());
}
