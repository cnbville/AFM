# AFM: American Football Manager

A deep, realistic American football management sim in the spirit of Football Manager: realistic ratings, hidden and dynamic potential, and players who develop from high school freshmen to the pros.

The engine is a Rust library with no UI. A desktop app will call it later.

- `crates/afm-engine`: the engine
  - `attributes.rs`, `positions.rs`: ratings and position-specific overalls
  - `player.rs`: players, hidden ceilings and personality, prospect generation
  - `development.rs`: the yearly development checkpoint (growth, dynamic potential, injuries, ageing)
  - `scouting.rs`: what staff can see (reports, and the vague potential outlook the game shows)
  - `level_rating.rs`: level-relative ratings ("89 among seniors")
  - `league.rs`: the 32 NFL teams, front offices and depth charts
  - `career.rs`: a whole national class from freshman year to retirement
  - `debug.rs`: true potential, for tooling only (`debug` feature)
- `crates/afm-tools`: calibration tooling and the national-class tests
- `legacy-ts/`: the original TypeScript model, kept for reference

The plan is in [docs/roadmap.md](docs/roadmap.md). The player model rules are in [docs/development-model.md](docs/development-model.md), and every decision in [docs/decisions.md](docs/decisions.md).

## Commands

```sh
cargo test --workspace                                # all tests, including a full 250,000-player class
cargo run --release --bin calibrate -- 250000 seed    # print real-world calibration stats
cargo clippy --workspace --all-targets && cargo fmt --check
```
