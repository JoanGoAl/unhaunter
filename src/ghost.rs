use crate::{
    board::{BoardPosition, Position},
    ghost_definitions::GhostType,
    player::PlayerSprite,
    summary, utils,
};
use bevy::prelude::*;
use rand::Rng;

const DEBUG_HUNTS: bool = false;

#[derive(Component, Debug)]
pub struct GhostSprite {
    pub class: GhostType,
    pub spawn_point: BoardPosition,
    pub target_point: Option<Position>,
    pub repellent_hits: i64,
    pub repellent_misses: i64,
    pub breach_id: Option<Entity>,
    pub rage: f32,
    pub hunting: f32,
    pub hunt_target: bool,
}

#[derive(Component, Debug)]
pub struct GhostBreach;

impl GhostSprite {
    pub fn new(spawn_point: BoardPosition) -> Self {
        let mut rng = rand::thread_rng();
        let ghost_types: Vec<_> = GhostType::all().collect();
        let idx = rng.gen_range(0..ghost_types.len());
        let class = ghost_types[idx];
        warn!("Ghost type: {:?} - {:?}", class, class.evidences());
        GhostSprite {
            class,
            spawn_point,
            target_point: None,
            repellent_hits: 0,
            repellent_misses: 0,
            breach_id: None,
            rage: 0.0,
            hunting: 0.0,
            hunt_target: false,
        }
    }
    pub fn with_breachid(self, breach_id: Entity) -> Self {
        Self {
            breach_id: Some(breach_id),
            ..self
        }
    }
}

pub fn ghost_movement(
    mut q: Query<(&mut GhostSprite, &mut Position, Entity), Without<PlayerSprite>>,
    qp: Query<&Position, With<PlayerSprite>>,
    roomdb: Res<crate::board::RoomDB>,
    mut summary: ResMut<summary::SummaryData>,
    bf: Res<crate::board::BoardData>,
    mut commands: Commands,
    time: Res<Time>,
) {
    let dt = time.delta_seconds() * 60.0;
    for (mut ghost, mut pos, entity) in q.iter_mut() {
        if let Some(target_point) = ghost.target_point {
            let mut delta = target_point.delta(*pos);
            let dlen = delta.distance();
            if dlen > 1.0 {
                delta.dx /= dlen.sqrt();
                delta.dy /= dlen.sqrt();
            }
            pos.x += delta.dx / 200.0 * dt;
            pos.y += delta.dy / 200.0 * dt;
            let mut finalize = false;
            if ghost.hunt_target {
                pos.x += delta.dx / 70.0 * dt;
                pos.y += delta.dy / 70.0 * dt;
                ghost.hunting -= dt / 60.0;
                if ghost.hunting < 0.0 {
                    ghost.hunting = 0.0;
                    ghost.hunt_target = false;
                    finalize = true;
                    warn!("Hunt finished");
                }
            }
            if dlen < 0.5 {
                finalize = true;
                if ghost.hunt_target {
                    warn!("Hunt leg over");
                }
            }
            if finalize {
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
            let mut hunt = false;
            target_point.x = (target_point.x + pos.x * wander) / (1.0 + wander) + dx / dd;
            target_point.y = (target_point.y + pos.y * wander) / (1.0 + wander) + dy / dd;
            let ghbonus = if ghost.hunt_target { 1000.0 } else { 0.0001 };
            if rng.gen_range(0.0..(ghost.hunting * 10.0 + ghbonus).sqrt() * 10.0) > 10.0 {
                let player_pos_l: Vec<&Position> = qp.iter().collect();
                let idx = rng.gen_range(0..player_pos_l.len());
                let ppos = player_pos_l[idx];
                target_point.x = ppos.x;
                target_point.y = ppos.y;
                hunt = true;
            }

            let bpos = target_point.to_board_position();
            if roomdb.room_tiles.get(&bpos).is_some()
                && bf
                    .collision_field
                    .get(&bpos)
                    .map(|x| x.ghost_free)
                    .unwrap_or_default()
            {
                if hunt {
                    ghost.hunting /= 1.2;
                    warn!("Hunting player for {:.1}s", ghost.hunting);
                } else if ghost.hunt_target {
                    warn!("Hunt temporarily ended (remaining) {:.1}s", ghost.hunting);
                }

                ghost.target_point = Some(target_point);
                ghost.hunt_target = hunt;
            }
        }
        if ghost.repellent_hits > 100 {
            summary.ghosts_unhaunted += 1;
            if let Some(breach) = ghost.breach_id {
                commands.entity(breach).despawn_recursive();
            }
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn ghost_enrage(
    time: Res<Time>,
    mut timer: Local<utils::PrintingTimer>,
    mut avg_angry: Local<utils::MeanValue>,
    mut qg: Query<(&mut GhostSprite, &Position)>,
    qp: Query<(&PlayerSprite, &Position)>,
) {
    timer.tick(time.delta());
    let dt = time.delta_seconds();

    for (mut ghost, gpos) in &mut qg {
        if ghost.hunt_target {
            continue;
        }
        let mut total_angry2 = 0.0;
        for (player, ppos) in &qp {
            let sanity = player.sanity();
            let dist2 = gpos.distance2(ppos) * (0.01 + sanity) + 0.1 + sanity / 100.0;
            let angry2 = dist2.recip() * 1000000.0 / sanity * player.mean_sound;
            total_angry2 += angry2;
        }
        let angry = total_angry2.sqrt();
        ghost.rage /= 1.005_f32.powf(dt);
        if DEBUG_HUNTS {
            ghost.rage += angry * dt * 10.0;
        }
        ghost.rage += angry * dt / 10.0;
        avg_angry.push_len(angry, dt);
        if timer.just_finished() {
            dbg!(&avg_angry.avg(), ghost.rage);
        }
        let rage_limit = if DEBUG_HUNTS { 40.0 } else { 120.0 };
        if ghost.rage > rage_limit {
            let prev_rage = ghost.rage;
            ghost.rage /= 1.2;
            ghost.hunting += (prev_rage - ghost.rage) / 4.0;
        }
    }
}

pub fn app_setup(app: &mut App) {
    app.add_systems(Update, (ghost_movement, ghost_enrage));
}
