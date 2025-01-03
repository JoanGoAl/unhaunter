use bevy::prelude::*;
use bevy::utils::{HashMap, Instant};
use fastapprox::faster;
use rand::Rng;
use std::f32::consts::PI;

pub use uncore::components::board::boardposition::BoardPosition;
pub use uncore::components::board::direction::Direction;
pub use uncore::components::board::position::Position;
pub use uncore::resources::roomdb::RoomDB;
pub use uncore::utils::light::compute_color_exposure;
pub use unstd::board::spritedb::SpriteDB;
pub use unstd::board::tiledata::{MapTileComponents, PreMesh, TileSpriteBundle};

use uncore::behavior::Behavior;
use uncore::types::board::light::LightData;
use uncore::{
    resources::boarddata::BoardData,
    types::board::fielddata::{CollisionFieldData, LightFieldData},
};

#[derive(Clone, Debug, Default, Event)]
pub struct BoardDataToRebuild {
    pub lighting: bool,
    pub collision: bool,
}

#[derive(Clone, Debug, Resource, Default)]
pub struct VisibilityData {
    pub visibility_field: HashMap<BoardPosition, f32>,
}

#[derive(Clone, Debug)]
pub struct LightFieldSector {
    field: Vec<LightFieldData>,
    min_x: i64,
    min_y: i64,
    _min_z: i64,
    sz_x: usize,
    sz_y: usize,
    _sz_z: usize,
}

// FIXME: This has exactly the same computation as HashMap, at least for the part
// that it matters.
impl LightFieldSector {
    pub fn new(min_x: i64, min_y: i64, min_z: i64, max_x: i64, max_y: i64, max_z: i64) -> Self {
        let sz_x = (max_x - min_x + 1).max(0) as usize;
        let sz_y = (max_y - min_y + 1).max(0) as usize;
        let sz_z = (max_z - min_z + 1).max(0) as usize;
        let light_field: Vec<LightFieldData> =
            vec![LightFieldData::default(); sz_x * sz_y * sz_z + 15000];
        Self {
            field: light_field,
            min_x,
            min_y,
            _min_z: min_z,
            sz_x,
            sz_y,
            _sz_z: sz_z,
        }
    }

    #[inline]
    fn vec_coord(&self, x: i64, y: i64, _z: i64) -> usize {
        let x = x - self.min_x;
        let y = y - self.min_y;

        // let z = z - self.min_z; These are purposefully allowing overflow and clamping
        // to an out of bounds value.
        let x = (x as usize).min(self.sz_x);
        let y = (y as usize).min(self.sz_y);

        // let z = (z as usize).min(self.sz_z);
        //
        // * z * self.sz_x * self.sz_y
        x + y * self.sz_x
        // (x & 0xF) | ((y & 0xF) << 4) | ((x & 0xFFFFF0) << 4) | ((y & 0xFFFFF0) << 8)
    }

    pub fn get_mut(&mut self, x: i64, y: i64, z: i64) -> Option<&mut LightFieldData> {
        let xyz = self.vec_coord(x, y, z);
        self.field.get_mut(xyz)
    }

    pub fn get_pos(&self, p: &BoardPosition) -> Option<&LightFieldData> {
        self.get(p.x, p.y, p.z)
    }

    pub fn get_mut_pos(&mut self, p: &BoardPosition) -> Option<&mut LightFieldData> {
        self.get_mut(p.x, p.y, p.z)
    }

    #[inline]
    pub fn get(&self, x: i64, y: i64, z: i64) -> Option<&LightFieldData> {
        let xyz = self.vec_coord(x, y, z);
        self.field.get(xyz)
    }

    /// get_pos_unchecked: Does not seem to be any faster.
    // #[inline] pub unsafe fn get_pos_unchecked(&self, p: &BoardPosition) ->
    // &LightFieldData { // let xyz = self.vec_coord(p.x, p.y, p.z); let xyz = (p.x -
    // self.min_x) as usize + (p.y - self.min_y) as usize * self.sz_x;
    // self.field.get_unchecked(xyz) }
    pub fn insert(&mut self, x: i64, y: i64, z: i64, lfd: LightFieldData) {
        let xyz = self.vec_coord(x, y, z);
        self.field[xyz] = lfd;
    }
}

#[derive(Debug, Clone)]
struct CachedBoardPos {
    dist: [[f32; Self::SZ]; Self::SZ],
    angle: [[usize; Self::SZ]; Self::SZ],
    angle_range: [[(i64, i64); Self::SZ]; Self::SZ],
}

impl CachedBoardPos {
    const CENTER: i64 = 32;
    const SZ: usize = (Self::CENTER * 2 + 1) as usize;

    /// Perimeter of the circle for indexing.
    const TAU_I: usize = 48 * 2;

    fn new() -> Self {
        let mut r = Self {
            dist: [[0.0; Self::SZ]; Self::SZ],
            angle: [[0; Self::SZ]; Self::SZ],
            angle_range: [[(0, 0); Self::SZ]; Self::SZ],
        };
        r.compute_angle();
        r.compute_dist();
        r
    }

    fn compute_dist(&mut self) {
        for (x, xv) in self.dist.iter_mut().enumerate() {
            for (y, yv) in xv.iter_mut().enumerate() {
                let x: f32 = x as f32 - Self::CENTER as f32;
                let y: f32 = y as f32 - Self::CENTER as f32;
                let dist: f32 = (x * x + y * y).sqrt();
                *yv = dist;
            }
        }
    }

    fn compute_angle(&mut self) {
        for (x, xv) in self.angle.iter_mut().enumerate() {
            for (y, yv) in xv.iter_mut().enumerate() {
                let x: f32 = x as f32 - Self::CENTER as f32;
                let y: f32 = y as f32 - Self::CENTER as f32;
                let dist: f32 = (x * x + y * y).sqrt();
                let x = x / dist;
                let y = y / dist;
                let angle = x.acos() * y.signum() * Self::TAU_I as f32 / PI / 2.0;
                let angle_i = (angle.round() as i64).rem_euclid(Self::TAU_I as i64);
                *yv = angle_i as usize;
            }
        }
        for y in Self::CENTER - 3..=Self::CENTER + 3 {
            let mut v: Vec<usize> = vec![];
            for x in Self::CENTER - 3..=Self::CENTER + 3 {
                v.push(self.angle[x as usize][y as usize]);
            }
        }
        for (x, xv) in self.angle_range.iter_mut().enumerate() {
            for (y, yv) in xv.iter_mut().enumerate() {
                let orig_angle = self.angle[x][y];

                // if angle < Self::TAU_I / 4 || angle > Self::TAU_I - Self::TAU_I / 4 { // Angles
                // closer to zero need correction to avoid looking on the wrong place }
                let mut min_angle: i64 = 0;
                let mut max_angle: i64 = 0;
                let x: f32 = x as f32 - Self::CENTER as f32;
                let y: f32 = y as f32 - Self::CENTER as f32;
                for x1 in [x - 0.5, x + 0.5] {
                    for y1 in [y - 0.5, y + 0.5] {
                        let dist: f32 = (x1 * x1 + y1 * y1).sqrt();
                        let x1 = x1 / dist;
                        let y1 = y1 / dist;
                        let angle = x1.acos() * y1.signum() * Self::TAU_I as f32 / PI / 2.0;
                        let mut angle_i = angle.round() as i64 - orig_angle as i64;
                        if angle_i.abs() > Self::TAU_I as i64 / 2 {
                            angle_i -= Self::TAU_I as i64 * angle_i.signum();
                        }
                        min_angle = min_angle.min(angle_i);
                        max_angle = max_angle.max(angle_i);
                    }
                }
                *yv = (min_angle, max_angle);
            }
        }
        for y in Self::CENTER - 3..=Self::CENTER + 3 {
            let mut v: Vec<(i64, i64)> = vec![];
            for x in Self::CENTER - 3..=Self::CENTER + 3 {
                v.push(self.angle_range[x as usize][y as usize]);
            }
        }
    }

    fn bpos_dist(&self, s: &BoardPosition, d: &BoardPosition) -> f32 {
        let x = (d.x - s.x + Self::CENTER) as usize;
        let y = (d.y - s.y + Self::CENTER) as usize;

        // self.dist[x][y]
        unsafe { *self.dist.get_unchecked(x).get_unchecked(y) }
    }

    fn bpos_angle(&self, s: &BoardPosition, d: &BoardPosition) -> usize {
        let x = (d.x - s.x + Self::CENTER) as usize;
        let y = (d.y - s.y + Self::CENTER) as usize;

        // self.angle[x][y]
        unsafe { *self.angle.get_unchecked(x).get_unchecked(y) }
    }

    fn bpos_angle_range(&self, s: &BoardPosition, d: &BoardPosition) -> (i64, i64) {
        let x = (d.x - s.x + Self::CENTER) as usize;
        let y = (d.y - s.y + Self::CENTER) as usize;

        // self.angle_range[x][y]
        unsafe { *self.angle_range.get_unchecked(x).get_unchecked(y) }
    }
}

pub fn boardfield_update(
    mut bf: ResMut<BoardData>,
    mut ev_bdr: EventReader<BoardDataToRebuild>,
    qt: Query<(&Position, &Behavior)>,
) {
    let mut rng = rand::thread_rng();

    // Here we will recreate the field (if needed? - not sure how to detect that) ...
    // maybe add a timer since last update.
    let mut bdr = BoardDataToRebuild::default();

    // Merge all the incoming events into a single one.
    for b in ev_bdr.read() {
        if b.collision {
            bdr.collision = true;
        }
        if b.lighting {
            bdr.lighting = true;
        }
    }
    if bdr.collision {
        // info!("Collision rebuild");
        bf.collision_field.clear();
        for (pos, _behavior) in qt.iter().filter(|(_p, b)| b.p.movement.walkable) {
            let pos = pos.to_board_position();
            let colfd = CollisionFieldData {
                player_free: true,
                ghost_free: true,
                see_through: false,
            };
            bf.collision_field.insert(pos, colfd);
        }
        for (pos, behavior) in qt.iter().filter(|(_p, b)| b.p.movement.player_collision) {
            let pos = pos.to_board_position();
            let colfd = CollisionFieldData {
                player_free: false,
                ghost_free: !behavior.p.movement.ghost_collision,
                see_through: behavior.p.light.see_through,
            };
            bf.collision_field.insert(pos, colfd);
        }
    }

    // Create temperature field - only missing data
    let valid_k: Vec<_> = bf.collision_field.keys().cloned().collect();
    let ambient_temp = bf.ambient_temp;
    let mut added_temps: Vec<BoardPosition> = vec![];

    // Randomize initial temperatures so the player cannot exploit the fact that the
    // data is "flat" at the beginning
    for pos in valid_k.into_iter() {
        let missing = bf.temperature_field.get(&pos).is_none();
        if missing {
            let ambient = ambient_temp + rng.gen_range(-10.0..10.0);
            added_temps.push(pos.clone());
            bf.temperature_field.insert(pos, ambient);
        }
    }

    // Smoothen after first initialization so it is not as jumpy.
    for _ in 0..16 {
        for pos in added_temps.iter() {
            let nbors = pos.xy_neighbors(1);
            let mut t_temp = 0.0;
            let mut count = 0.0;
            let free_tot = bf
                .collision_field
                .get(pos)
                .map(|x| x.player_free)
                .unwrap_or(true);
            for npos in &nbors {
                let free = bf
                    .collision_field
                    .get(npos)
                    .map(|x| x.player_free)
                    .unwrap_or(true);
                if free {
                    t_temp += bf
                        .temperature_field
                        .get(npos)
                        .copied()
                        .unwrap_or(ambient_temp);
                    count += 1.0;
                }
            }
            if free_tot {
                t_temp /= count;
                bf.temperature_field
                    .entry(pos.clone())
                    .and_modify(|x| *x = t_temp);
            }
        }
    }
    if bdr.lighting {
        // Rebuild lighting field since it has changed info!("Lighting rebuild");
        let build_start_time = Instant::now();
        let cbp = CachedBoardPos::new();
        bf.exposure_lux = 1.0;
        bf.light_field.clear();

        // Dividing by 4 so later we don't get an overflow if there's no map.
        let first_p = qt
            .iter()
            .next()
            .map(|(p, _b)| p.to_board_position())
            .unwrap_or_default();
        let mut min_x = first_p.x;
        let mut min_y = first_p.y;
        let mut min_z = first_p.z;
        let mut max_x = first_p.x;
        let mut max_y = first_p.y;
        let mut max_z = first_p.z;
        for (pos, behavior) in qt.iter() {
            let pos = pos.to_board_position();
            min_x = min_x.min(pos.x);
            min_y = min_y.min(pos.y);
            min_z = min_z.min(pos.z);
            max_x = max_x.max(pos.x);
            max_y = max_y.max(pos.y);
            max_z = max_z.max(pos.z);
            let src = bf.light_field.get(&pos).cloned().unwrap_or(LightFieldData {
                lux: 0.0,
                transmissivity: 1.0,
                additional: LightData::default(),
            });
            let lightdata = LightFieldData {
                lux: behavior.p.light.emmisivity_lumens() + src.lux,
                transmissivity: behavior.p.light.transmissivity_factor() * src.transmissivity
                    + 0.0001,
                additional: src.additional.add(&behavior.p.light.additional_data()),
            };
            bf.light_field.insert(pos, lightdata);
        }

        // info!( "Collecting time: {:?} - sz: {}", build_start_time.elapsed(),
        // bf.light_field.len() );
        let mut lfs = LightFieldSector::new(min_x, min_y, min_z, max_x, max_y, max_z);
        for (k, v) in bf.light_field.iter() {
            lfs.insert(k.x, k.y, k.z, v.clone());
        }
        let mut nbors_buf = Vec::with_capacity(52 * 52);

        // let mut lfs_clone_time_total = Duration::ZERO; let mut shadows_time_total =
        // Duration::ZERO; let mut store_lfs_time_total = Duration::ZERO;
        for step in 0..3 {
            // let lfs_clone_time = Instant::now();
            let src_lfs = lfs.clone();

            // lfs_clone_time_total += lfs_clone_time.elapsed();
            let size = match step {
                0 => 26,
                1 => 8,
                2 => 6,
                3 => 3,
                _ => 6,
            };
            for x in min_x..=max_x {
                for y in min_y..=max_y {
                    for z in min_z..=max_z {
                        let Some(src) = src_lfs.get(x, y, z) else {
                            continue;
                        };

                        // if src.transmissivity < 0.5 && step > 0 && size > 1 { // Reduce light spread
                        // through walls // FIXME: If the light is on the wall, this breaks (and this is
                        // possible since the wall is really 1/3rd of the tile) continue; }
                        let root_pos = BoardPosition { x, y, z };
                        let mut src_lux = src.lux;
                        let min_lux = match step {
                            0 => 0.001,
                            1 => 0.000001,
                            _ => 0.0000000001,
                        };
                        let max_lux = match step {
                            0 => f32::MAX,
                            1 => 10000.0,
                            2 => 1000.0,
                            3 => 0.1,
                            _ => 0.01,
                        };
                        if src_lux < min_lux {
                            continue;
                        }
                        if src_lux > max_lux {
                            continue;
                        }

                        // Optimize next steps by only looking to harsh differences.
                        root_pos.xy_neighbors_buf_clamped(
                            1,
                            &mut nbors_buf,
                            min_x,
                            max_x,
                            min_y,
                            max_y,
                        );
                        let nbors = &nbors_buf;
                        if step > 0 {
                            let ldata_iter = nbors.iter().filter_map(|b| {
                                lfs.get_pos(b).map(|l| {
                                    (
                                        ordered_float::OrderedFloat(l.lux),
                                        ordered_float::OrderedFloat(l.transmissivity),
                                    )
                                })
                            });
                            let mut min_lux = ordered_float::OrderedFloat(f32::MAX);
                            let mut min_trans = ordered_float::OrderedFloat(2.0);
                            for (lux, trans) in ldata_iter {
                                min_lux = min_lux.min(lux);
                                min_trans = min_trans.min(trans);
                            }

                            // For smoothing steps only:
                            if *min_trans > 0.7 && src_lux / (*min_lux + 0.0001) < 1.9 {
                                // If there are no walls nearby, we don't reflect light.
                                continue;
                            }
                        }

                        // This controls how harsh is the light
                        if step > 0 {
                            src_lux /= 5.5;
                        } else {
                            src_lux /= 1.01;
                        }

                        // let shadows_time = Instant::now(); This takes time to process:
                        root_pos.xy_neighbors_buf_clamped(
                            size,
                            &mut nbors_buf,
                            min_x,
                            max_x,
                            min_y,
                            max_y,
                        );
                        let nbors = &nbors_buf;

                        // reset the light value for this light, so we don't count double.
                        lfs.get_mut_pos(&root_pos).unwrap().lux -= src_lux;
                        let mut shadow_dist = [(size + 1) as f32; CachedBoardPos::TAU_I];

                        // Compute shadows
                        for pillar_pos in nbors.iter() {
                            // 60% of the time spent in compute shadows is obtaining this:
                            let Some(lf) = lfs.get_pos(pillar_pos) else {
                                continue;
                            };

                            // let lf = unsafe { lfs.get_pos_unchecked(pillar_pos) }; t_x += lf.lux; continue;
                            let consider_opaque = lf.transmissivity < 0.5;
                            if !consider_opaque {
                                continue;
                            }
                            let min_dist = cbp.bpos_dist(&root_pos, pillar_pos);
                            let angle = cbp.bpos_angle(&root_pos, pillar_pos);
                            let angle_range = cbp.bpos_angle_range(&root_pos, pillar_pos);
                            for d in angle_range.0..=angle_range.1 {
                                let ang = (angle as i64 + d)
                                    .rem_euclid(CachedBoardPos::TAU_I as i64)
                                    as usize;
                                shadow_dist[ang] = shadow_dist[ang].min(min_dist);
                            }
                        }

                        // shadows_time_total += shadows_time.elapsed(); FIXME: Possibly we want to smooth
                        // shadow_dist here - a convolution with a gaussian or similar where we preserve
                        // the high values but smooth the transition to low ones.
                        if src.transmissivity < 0.5 {
                            // Reduce light spread through walls
                            shadow_dist.iter_mut().for_each(|x| *x = 0.0);
                        }

                        // let size = shadow_dist .iter() .map(|d| (d + 1.5).round() as u32) .max()
                        // .unwrap() .min(size); let nbors = root_pos.xy_neighbors(size);
                        let light_height = 4.0;

                        // let mut total_lux = 0.1; for neighbor in nbors.iter() { let dist =
                        // cbp.bpos_dist(&root_pos, neighbor); let dist2 = dist + light_height; let angle
                        // = cbp.bpos_angle(&root_pos, neighbor); let sd = shadow_dist[angle]; let f =
                        // (faster::tanh(sd - dist - 0.5) + 1.0) / 2.0; total_lux += f / dist2 / dist2; }
                        // let store_lfs_time = Instant::now();
                        let total_lux = 2.0;

                        // new shadow method
                        for neighbor in nbors.iter() {
                            let dist = cbp.bpos_dist(&root_pos, neighbor);

                            // let dist = root_pos.fast_distance_xy(neighbor);
                            let dist2 = dist + light_height;
                            let angle = cbp.bpos_angle(&root_pos, neighbor);
                            let sd = shadow_dist[angle];
                            let lux_add = src_lux / dist2 / dist2 / total_lux;
                            if dist - 3.0 < sd {
                                // FIXME: f here controls the bleed through walls.
                                if let Some(lf) = lfs.get_mut_pos(neighbor) {
                                    // 0.5 is too low, it creates un-evenness.
                                    const BLEED_TILES: f32 = 0.8;
                                    let f = (faster::tanh((sd - dist - 0.5) * BLEED_TILES.recip())
                                        + 1.0)
                                        / 2.0;

                                    // let f = 1.0;
                                    lf.lux += lux_add * f;
                                }
                            }
                        }
                        // store_lfs_time_total += store_lfs_time.elapsed();
                    }
                }
            }
            // info!( "Light step {}: {:?}; per size: {:?}", step, step_time.elapsed(),
            // step_time.elapsed() / size );
        }
        for (k, v) in bf.light_field.iter_mut() {
            v.lux = lfs.get_pos(k).unwrap().lux;
        }

        // let's get an average of lux values
        let mut total_lux = 0.0;
        for (_, v) in bf.light_field.iter() {
            total_lux += v.lux;
        }
        let avg_lux = total_lux / bf.light_field.len() as f32;
        bf.exposure_lux = (avg_lux + 2.0) / 2.0;

        // dbg!(lfs_clone_time_total); dbg!(shadows_time_total);
        // dbg!(store_lfs_time_total);
        info!(
            "Lighting rebuild - complete: {:?}",
            build_start_time.elapsed()
        );
    }
}

/// Main system of board that moves the tiles to their correct place in the screen
/// following the isometric perspective.
pub fn apply_perspective(mut q: Query<(&Position, &mut Transform)>) {
    for (pos, mut transform) in q.iter_mut() {
        transform.translation = pos.to_screen_coord();
    }
}

pub struct UnhaunterBoardPlugin;

impl Plugin for UnhaunterBoardPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BoardData>()
            .init_resource::<VisibilityData>()
            .init_resource::<SpriteDB>()
            .init_resource::<RoomDB>()
            .add_systems(Update, apply_perspective)
            .add_systems(PostUpdate, boardfield_update)
            .add_event::<BoardDataToRebuild>();
    }
}
