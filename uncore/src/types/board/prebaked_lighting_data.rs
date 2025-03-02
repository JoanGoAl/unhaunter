//! Prebaked lighting data for optimized light propagation.
//!
//! This module defines structures to store precomputed lighting information,
//! allowing for significant performance improvements by reusing calculations
//! for static elements in the map.

use bevy::ecs::entity::Entity;

/// Holds precomputed lighting propagation data for a single tile.
///
/// This structure stores minimal information about light sources and wave edges,
/// supporting an efficient BFS-based light propagation algorithm that can be
/// resumed at runtime when dynamic elements (like doors) change state.
#[derive(Clone, Debug, Default)]
pub struct PrebakedLightingData {
    /// Base light data for this tile
    pub light_info: LightInfo,

    /// Indicates if this is a wave edge (point where light wave stopped propagating)
    /// These points can be used to continue light propagation at runtime
    /// for dynamic elements like doors
    pub is_wave_edge: bool,
}

/// Stores the base light information for a tile
#[derive(Clone, Debug, Default)]
pub struct LightInfo {
    /// The light source ID, or None if not a source
    pub source_id: Option<u32>,

    /// Amount of lux intensity in this tile
    pub lux: f32,

    /// Light color (r, g, b)
    pub color: (f32, f32, f32),
}

/// Stores general metadata useful for speeding up light rebuilds.
#[derive(Clone, Debug, Default)]
pub struct PrebakedMetadata {
    pub light_sources: Vec<(Entity, (usize, usize, usize))>,
}
