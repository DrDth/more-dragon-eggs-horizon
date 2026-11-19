# More Dragon Eggs for Horizon

A port of [More Dragon Eggs](https://github.com/Darkere/moredragoneggs) by Darkere to
[Horizon](https://github.com/horizon-rust/horizon), the Rust rewrite of Minecraft: Java Edition 26.3. Built
as a wasm mod from the original's 1.21 sources (5.0), the version All the Mods 10 ships.

- A dragon egg spawns on the exit portal's pillar every time the Ender Dragon dies, not only the first time.
- With `spawnHead = true` in `config/moredragoneggs-server.toml`, a dragon head appears on the pillar's side
  facing the nearest player too. Off by default, as in the original.

The mod is server side only: a server or a single-player world needs it, joining players don't.

## Differences from the original

- The setting lives in the game's `config/moredragoneggs-server.toml`, one for all worlds. The original keeps
  it in NeoForge's config of the same name.
- The egg on a later win sits where Horizon places the first egg, after the exit portal is lit. The original
  places it before the portal is rebuilt; the spot is the same.

## Building

The mod needs `dragon_egg_form` to fire on every won fight, added to Horizon in
[horizon-rust/horizon#18520](https://github.com/horizon-rust/horizon/issues/18520). Until a Horizon release has
it, build the mod inside a Horizon checkout that does:

```sh
mkdir -p <horizon>/.local/dev-mods/more_dragon_eggs
cp -R Cargo.toml horizon-mod.toml src LICENSE <horizon>/.local/dev-mods/more_dragon_eggs/
cd <horizon>/.local/dev-mods/more_dragon_eggs
cargo run --manifest-path ../../../Cargo.toml -p cargo-horizon-mod -- horizon-mod build --wasm
```

## License

MIT, as the original. Copyright (c) Darkere; the Horizon port by DrDth. See [LICENSE](LICENSE).
