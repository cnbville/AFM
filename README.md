# AFM: American Football Manager

A deep, realistic American football management sim in the spirit of Football Manager: realistic ratings, hidden and dynamic potential, and players who develop from high school freshmen to the pros.

This repository currently holds the core player model, written as pure TypeScript logic with no UI or engine coupling.

- `src/attributes.ts`, `src/positions.ts`: ratings and position-specific overalls
- `src/player.ts`: players, hidden ceilings and personality, prospect generation
- `src/development.ts`: the yearly development checkpoint (growth, dynamic potential, injuries, ageing)
- `src/scouting.ts`: what staff can see
- `src/career.ts`: a whole national class from freshman year to retirement

The rules are explained in [docs/development-model.md](docs/development-model.md).

## Commands

```sh
npm install
npm test            # unit tests plus a full 250,000-player class simulation (~35s)
npm run typecheck
npm run calibrate   # print real-world calibration stats (optional args: size seed)
```
