// ------------ Bevy map loading utils --------------------
use crate::materials::CustomMaterial1;
use bevy::{prelude::*, utils::HashMap};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use uncore::types::tiledmap::map::{MapLayer, MapLayerGroup};

use super::load::load_tile_layer_iter;

#[derive(Debug, Clone)]
pub enum AtlasData {
    Sheet((Handle<TextureAtlasLayout>, CustomMaterial1)),
    Tiles(Vec<(Handle<Image>, CustomMaterial1)>),
}

#[derive(Debug, Clone)]
pub struct MapTileSet {
    pub tileset: Arc<tiled::Tileset>,
    pub data: AtlasData,
    pub y_anchor: f32,
}

#[derive(Debug, Clone, Default, Resource)]
pub struct MapTileSetDb {
    pub db: HashMap<String, MapTileSet>,
}

#[cfg(not(target_arch = "wasm32"))]
mod arch {
    pub fn map_loader(path: impl AsRef<std::path::Path>) -> tiled::Map {
        let mut loader = tiled::Loader::new();
        loader.load_tmx_map(path).unwrap()
    }
}

#[cfg(target_arch = "wasm32")]
mod arch {
    use std::io::Cursor;

    /// Basic example reader impl that just keeps a few resources in memory
    struct MemoryReader;

    impl tiled::ResourceReader for MemoryReader {
        type Resource = Cursor<&'static [u8]>;
        type Error = std::io::Error;

        fn read_from(
            &mut self,
            path: &std::path::Path,
        ) -> std::result::Result<Self::Resource, Self::Error> {
            let path = path.to_str().unwrap();
            match path {
                "assets/maps/tut01_basics.tmx" => Ok(Cursor::new(include_bytes!(
                    "../../../assets/maps/tut01_basics.tmx"
                ))),
                "assets/maps/tut02_glass_house.tmx" => Ok(Cursor::new(include_bytes!(
                    "../../../assets/maps/tut02_glass_house.tmx"
                ))),
                "assets/maps/map_house1.tmx" => Ok(Cursor::new(include_bytes!(
                    "../../../assets/maps/map_house1.tmx"
                ))),
                "assets/maps/map_house2.tmx" => Ok(Cursor::new(include_bytes!(
                    "../../../assets/maps/map_house2.tmx"
                ))),
                "assets/maps/map_school1.tmx" => Ok(Cursor::new(include_bytes!(
                    "../../../assets/maps/map_school1.tmx"
                ))),
                "assets/maps/unhaunter_custom_tileset.tsx" => Ok(Cursor::new(include_bytes!(
                    "../../../assets/maps/unhaunter_custom_tileset.tsx"
                ))),
                "assets/maps/unhaunter_spritesheet2.tsx" => Ok(Cursor::new(include_bytes!(
                    "../../../assets/maps/unhaunter_spritesheet2.tsx"
                ))),
                "assets/maps/unhaunter_spritesheetA_3x3x3.tsx" => Ok(Cursor::new(include_bytes!(
                    "../../../assets/maps/unhaunter_spritesheetA_3x3x3.tsx"
                ))),
                "assets/maps/unhaunter_spritesheetA_6x6x10.tsx" => Ok(Cursor::new(include_bytes!(
                    "../../../assets/maps/unhaunter_spritesheetA_6x6x10.tsx"
                ))),
                _ => Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "file not found",
                )),
            }
        }
    }

    pub fn map_loader(path: impl AsRef<std::path::Path>) -> tiled::Map {
        let mut loader =
            tiled::Loader::<tiled::DefaultResourceCache, MemoryReader>::with_cache_and_reader(
                tiled::DefaultResourceCache::new(),
                MemoryReader,
            );
        loader.load_tmx_map(path).unwrap()
    }
}

/// Helps trimming the extra assets/ folder for Bevy
pub fn resolve_tiled_image_path(img_path: &Path) -> PathBuf {
    use normalize_path::NormalizePath;

    img_path
        .strip_prefix("assets/")
        .unwrap_or(img_path)
        .normalize()
        .to_owned()
}

pub fn bevy_load_map(
    path: impl AsRef<std::path::Path>,
    asset_server: &AssetServer,
    texture_atlases: &mut ResMut<Assets<TextureAtlasLayout>>,
    tilesetdb: &mut ResMut<MapTileSetDb>,
) -> (tiled::Map, Vec<(usize, MapLayer)>) {
    // Parse Tiled file:
    let path = path.as_ref();
    let map = arch::map_loader(path);

    // Preload all tilesets referenced:
    for tileset in map.tilesets().iter() {
        // If an image is included, this is a tilemap. If no image is included this is a
        // sprite collection. Sprite collections are not supported right now.
        let data = if let Some(image) = &tileset.image {
            let img_src = resolve_tiled_image_path(&image.source);

            // FIXME: When the images are loaded onto the GPU it seems that we need at least 1
            // pixel of empty space .. so that the GPU can sample surrounding pixels properly.
            // .. This contrasts with how Tiled works, as it assumes a perfect packing if
            // possible.
            const MARGIN: u32 = 1;

            // TODO: Ideally we would prefer to preload, upscale by nearest to 2x or 4x, and
            // add a 2px margin. Recreating .. the texture on the fly.
            let texture: Handle<Image> = asset_server.load(img_src);
            let rows = tileset.tilecount / tileset.columns;
            let atlas1 = TextureAtlasLayout::from_grid(
                UVec2::new(
                    tileset.tile_width + tileset.spacing - MARGIN,
                    tileset.tile_height + tileset.spacing - MARGIN,
                ),
                tileset.columns,
                rows,
                Some(UVec2::new(MARGIN, MARGIN)),
                Some(UVec2::new(0, 0)),
            );
            let mut cmat = CustomMaterial1::from_texture(texture);
            cmat.data.sheet_rows = rows;
            cmat.data.sheet_cols = tileset.columns;
            cmat.data.sheet_idx = 0;
            cmat.data.sprite_width = tileset.tile_width as f32 + tileset.spacing as f32;
            cmat.data.sprite_height = tileset.tile_height as f32 + tileset.spacing as f32;
            let atlas1_handle = texture_atlases.add(atlas1);
            AtlasData::Sheet((atlas1_handle.clone(), cmat))
        } else {
            let mut images: Vec<(Handle<Image>, CustomMaterial1)> = vec![];
            for (_tileid, tile) in tileset.tiles() {
                // tile.collision
                if let Some(image) = &tile.image {
                    let img_src = resolve_tiled_image_path(&image.source);
                    dbg!(&img_src);
                    let img_handle: Handle<Image> = asset_server.load(img_src);
                    let cmat = CustomMaterial1::from_texture(img_handle.clone());
                    images.push((img_handle, cmat));
                }
            }
            AtlasData::Tiles(images)
        };

        // NOTE: tile.offset_x/y is used when drawing, instead we want the center point.
        let anchor_bottom_px = tileset.properties.get("Anchor::bottom_px").and_then(|x| {
            if let tiled::PropertyValue::IntValue(n) = x {
                Some(n)
            } else {
                None
            }
        });
        let y_anchor: f32 = if let Some(n) = anchor_bottom_px {
            // find the fraction from the total image:
            let f = *n as f32 / (tileset.tile_height + tileset.spacing) as f32;

            // from the center:
            f - 0.5
        } else {
            -0.25
        };
        let mts = MapTileSet {
            tileset: tileset.clone(),
            data,
            y_anchor,
        };

        // Store the tileset in memory in case we need to do anything with it later on.
        if tilesetdb.db.insert(tileset.name.to_string(), mts).is_some() {
            eprintln!(
                "ERROR: Already existing tileset loaded with name {:?} - make sure you don't have the same tileset loaded twice",
                tileset.name.to_string()
            );
            // panic!();
        }
    }
    let map_layers = load_tile_layer_iter(map.layers());
    let grp = MapLayerGroup { layers: map_layers };
    let layers: Vec<(usize, MapLayer)> = grp
        .iter()
        .filter(|x| x.visible)
        .enumerate()
        .map(|(n, l)| (n, l.clone()))
        .collect();
    (map, layers)
    // let tile_size: (f32, f32) = (map.tile_width as f32, map.tile_height as f32);
    // bevy_load_layers(&layers, tile_size, &mut tilesetdb)
}
