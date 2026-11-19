//! More Dragon Eggs for Horizon: a dragon egg forms every time the Ender Dragon dies, not only
//! the first time, and with `spawnHead` on, a dragon head beside the exit portal's pillar too.
//!
//! A port of Darkere's More Dragon Eggs (MIT), from its 1.21 sources (5.0). The original injects
//! at the head of `EnderDragonFight.setDragonKilled`. The port answers Horizon's
//! `dragon_egg_form`, which on a later win arrives cancelled (vanilla places the first egg
//! only), and clears the cancel, so the game places the egg where it places the first.
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use mod_api::{
    BlockUpdateFlags, ConfigPath, DragonEggFormEvent, Flow, Hooks, LevelHost, LevelRef,
    ServerHost, ServerStartedEvent, ServerTickEvent, ServerTickPhase,
};

/// The original's config, `ServerConfig`, kept in the game's `config/` folder.
const CONFIG: &str = "moredragoneggs-server.toml";
const DEFAULT_CONFIG: &str = "#Also Spawn a Dragon head on the Pedestal.\nspawnHead = false\n";

#[derive(Default)]
pub struct MoreDragonEggs {
    /// `ServerConfig.spawnHead`, read when the server starts.
    spawn_head: AtomicBool,
    /// The won fight's egg position, waiting for its head: the original places the head on the
    /// server's next tick (`TickTask(0)`).
    head: Mutex<Option<(String, [i32; 3])>>,
}

/// The four sides the head may face, in `Direction.getClockWise` order from north.
#[derive(Clone, Copy)]
enum Side {
    North,
    East,
    South,
    West,
}

impl Side {
    fn name(self) -> &'static str {
        match self {
            Side::North => "north",
            Side::East => "east",
            Side::South => "south",
            Side::West => "west",
        }
    }
    fn clockwise(self) -> Self {
        match self {
            Side::North => Side::East,
            Side::East => Side::South,
            Side::South => Side::West,
            Side::West => Side::North,
        }
    }
    fn step(self, [x, y, z]: [i32; 3]) -> [i32; 3] {
        match self {
            Side::North => [x, y, z - 1],
            Side::East => [x + 1, y, z],
            Side::South => [x, y, z + 1],
            Side::West => [x - 1, y, z],
        }
    }
    /// `Direction.getNearest(x, 0, z, NORTH)`: the longer horizontal axis, north on a tie.
    fn nearest(x: i32, z: i32) -> Self {
        let (ax, az) = (x.abs(), z.abs());
        if ax > az && ax > 0 {
            if x < 0 { Side::West } else { Side::East }
        } else if az > ax && az > 0 {
            if z < 0 { Side::North } else { Side::South }
        } else {
            Side::North
        }
    }
}

/// `spawnHead` from the config's text, NeoForge's TOML: false unless set to true.
fn spawn_head(config: &str) -> bool {
    config.lines().any(|line| {
        let mut parts = line.split('=').map(str::trim);
        parts.next() == Some("spawnHead") && parts.next() == Some("true")
    })
}

fn block_is(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3], id: &str) -> bool {
    level
        .block_state(host, pos)
        .and_then(|state| host.state_block(state))
        .is_some_and(|block| block == id)
}

/// `MoreDragonEggs.PlaceDragonHead`: down from the egg's spot to the pillar's bedrock, then a
/// dragon head on the first side, turning clockwise from the one facing the nearest player,
/// whose block (past a bedrock rim) is air.
fn place_dragon_head(host: &mut dyn ServerHost, level: LevelRef, mut pos: [i32; 3]) {
    while !block_is(host, level, pos, "minecraft:bedrock") && pos[1] > 0 {
        pos[1] -= 1;
    }
    // `Level.getNearestPlayer(x, y, z, -1, null)`: any distance, the first of the nearest.
    let at = pos.map(f64::from);
    let nearest = host
        .players()
        .server_players()
        .into_iter()
        .filter(|player| player.entity().level(host) == Some(level))
        .filter_map(|player| player.entity().position(host))
        .map(|p| (p, (p[0] - at[0]).powi(2) + (p[1] - at[1]).powi(2) + (p[2] - at[2]).powi(2)))
        .min_by(|a, b| a.1.total_cmp(&b.1));
    let mut side = nearest.map_or(Side::North, |(p, _)| {
        Side::nearest((p[0] - at[0]) as i32, (p[2] - at[2]) as i32)
    });
    for _ in 0..4 {
        let mut head = side.step(pos);
        // Room for end podiums with a rim, as BetterEnd Reforked builds them.
        if block_is(host, level, head, "minecraft:bedrock") {
            head = side.step(head);
        }
        if block_is(host, level, head, "minecraft:air") {
            let name = format!("minecraft:dragon_wall_head[facing={}]", side.name());
            if let Some(state) = host.block_state_id(&name) {
                // `setBlockAndUpdate`.
                let flags = BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS;
                let _ = level.set_block(host, head, state, flags);
            }
            break;
        }
        side = side.clockwise();
    }
}

#[mod_api::horizon_mod]
impl Hooks for MoreDragonEggs {
    fn server_started(&self, host: &mut dyn ServerHost, _: &mut ServerStartedEvent) {
        let Ok(path) = ConfigPath::new(CONFIG) else {
            return;
        };
        let files = host.config_files();
        let text = match files.read(&path) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(_) => {
                // NeoForge writes the defaults when the file is missing.
                let _ = files.write(&path, DEFAULT_CONFIG.as_bytes());
                DEFAULT_CONFIG.to_owned()
            }
        };
        self.spawn_head.store(spawn_head(&text), Ordering::Relaxed);
    }

    // First of all mods, as the original's injection runs before the fight places anything:
    // the cancel it finds is the game's.
    #[listener(priority = 1000, receive_cancelled)]
    fn dragon_egg_form(&self, _: &dyn LevelHost, event: &mut DragonEggFormEvent) -> Flow {
        event.cancelled = false;
        if self.spawn_head.load(Ordering::Relaxed) {
            let pos = [event.pos.x, event.pos.y, event.pos.z];
            *self.head.lock().unwrap() = Some((event.dimension.key().id().to_owned(), pos));
        }
        Flow::Continue
    }

    fn server_tick(&self, host: &mut dyn ServerHost, event: &mut ServerTickEvent<'_>) {
        if !matches!(event.phase, ServerTickPhase::ServerTickStart { .. }) {
            return;
        }
        let Some((dimension, pos)) = self.head.lock().unwrap().take() else {
            return;
        };
        if let Some(level) = host.level(&dimension) {
            place_dragon_head(host, level, pos);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_head_faces_the_longer_axis_and_north_on_a_tie() {
        assert_eq!(Side::nearest(5, 2).name(), "east");
        assert_eq!(Side::nearest(-5, 2).name(), "west");
        assert_eq!(Side::nearest(1, -3).name(), "north");
        assert_eq!(Side::nearest(1, 3).name(), "south");
        assert_eq!(Side::nearest(3, 3).name(), "north");
        assert_eq!(Side::nearest(0, 0).name(), "north");
    }

    #[test]
    fn spawn_head_is_off_unless_the_config_turns_it_on() {
        assert!(!spawn_head(DEFAULT_CONFIG));
        assert!(spawn_head("#c\nspawnHead = true\n"));
        assert!(!spawn_head("spawnHead=maybe"));
    }
}
