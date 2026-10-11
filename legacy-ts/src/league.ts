import type { PlayingTime } from './player.js';
import { POSITIONS, type Position } from './positions.js';
import { Rng, clamp } from './rng.js';

/**
 * The pro league a class plays into: 32 teams with front offices, coaching
 * staffs and full depth charts. Only one high school class is simulated, so
 * everyone else on a roster is a background player: a rating and an age that
 * develop, age and leave like real veterans and are replaced by other classes'
 * draft picks and free agents. A class's players win and lose real depth chart
 * spots against them.
 */

export const NFL_TEAM_NAMES = [
  'Arizona Cardinals', 'Atlanta Falcons', 'Baltimore Ravens', 'Buffalo Bills', 'Carolina Panthers', 'Chicago Bears',
  'Cincinnati Bengals', 'Cleveland Browns', 'Dallas Cowboys', 'Denver Broncos', 'Detroit Lions', 'Green Bay Packers',
  'Houston Texans', 'Indianapolis Colts', 'Jacksonville Jaguars', 'Kansas City Chiefs', 'Las Vegas Raiders',
  'Los Angeles Chargers', 'Los Angeles Rams', 'Miami Dolphins', 'Minnesota Vikings', 'New England Patriots',
  'New Orleans Saints', 'New York Giants', 'New York Jets', 'Philadelphia Eagles', 'Pittsburgh Steelers',
  'San Francisco 49ers', 'Seattle Seahawks', 'Tampa Bay Buccaneers', 'Tennessee Titans', 'Washington Commanders',
] as const;

/**
 * Depth chart slots per position on a 53-man roster: starters, the extra
 * players who rotate in on most snaps (third receiver, nickel corner, a
 * rotational pass rusher), and the total kept on the roster. One kicker and
 * one punter.
 */
export const DEPTH_SLOTS: Record<Position, { starters: number; rotation: number; roster: number }> = {
  QB: { starters: 1, rotation: 0, roster: 3 },
  RB: { starters: 1, rotation: 1, roster: 4 },
  WR: { starters: 3, rotation: 1, roster: 6 },
  TE: { starters: 1, rotation: 1, roster: 3 },
  OT: { starters: 2, rotation: 0, roster: 4 },
  IOL: { starters: 3, rotation: 0, roster: 5 },
  EDGE: { starters: 2, rotation: 1, roster: 5 },
  DT: { starters: 2, rotation: 1, roster: 5 },
  LB: { starters: 2, rotation: 1, roster: 5 },
  CB: { starters: 2, rotation: 1, roster: 6 },
  S: { starters: 2, rotation: 0, roster: 4 },
  K: { starters: 1, rotation: 0, roster: 1 },
  P: { starters: 1, rotation: 0, roster: 1 },
};
/** Practice squad places per team. */
export const PRACTICE_SQUAD = 16;

/** A player on a depth chart: a background veteran, or one of the class (with its record id). */
export interface Member {
  rating: number;
  age: number;
  /** Set for a player from the simulated class. */
  id?: string;
  /** Role and depth chart spot from the last camp (class players only). */
  role?: PlayingTime;
  depth?: string;
}

export interface Team {
  id: number;
  name: string;
  /** Front office judgment (the GM), 1-20: how clearly the team sees players. */
  draftSkill: number;
  /** Owner, 1-20: patience, spending and stability. */
  owner: number;
  /** Head coach and staff, 1-20: how well players develop there. */
  coaching: number;
  depth: Record<Position, Member[]>;
  /** Years the current coaching staff has been in place. */
  staffYears: number;
}

/** How well run a team is, 1-20: GM, owner and coaching staff together. */
export function organisation(team: Team): number {
  return (team.draftSkill + team.owner + team.coaching) / 3;
}

/** Average rating of the team's starters: roughly how good the team is this year. */
export function teamStrength(team: Team): number {
  let sum = 0;
  let n = 0;
  for (const pos of POSITIONS) {
    const slots = DEPTH_SLOTS[pos].starters;
    const best = team.depth[pos].filter((m) => m.role !== 'practiceSquad').sort((a, b) => b.rating - a.rating).slice(0, slots);
    for (const m of best) {
      sum += m.rating;
      n++;
    }
  }
  return n ? sum / n : 0;
}

/** Weakest starter at a position (the spot a team most wants to fill) and his age. */
export function weakestStarter(team: Team, pos: Position): Member | undefined {
  const best = team.depth[pos].filter((m) => m.role !== 'practiceSquad').sort((a, b) => b.rating - a.rating).slice(0, DEPTH_SLOTS[pos].starters);
  return best[best.length - 1];
}

/** Age at which a background veteran at a position typically leaves the game. */
const BACKGROUND_EXIT_AGE: Record<Position, number> = {
  QB: 36, RB: 29, WR: 31, TE: 32, OT: 33, IOL: 33, EDGE: 32, DT: 32, LB: 31, CB: 31, S: 31, K: 38, P: 38,
};

/** Yearly rating change of a background player with age. */
function backgroundAgeing(pos: Position, age: number): number {
  const shift = pos === 'RB' ? -2 : pos === 'CB' || pos === 'S' ? -1 : pos === 'QB' ? 3 : pos === 'K' || pos === 'P' ? 5 : 0;
  const a = age - shift;
  if (a < 25) return 2;
  if (a < 28) return 0.6;
  if (a < 30) return -0.6;
  return -(1.4 + 0.7 * (a - 30));
}

function newBackground(rng: Rng, pos: Position, slot: number, talent: number, young: boolean): Member {
  const { starters, rotation } = DEPTH_SLOTS[pos];
  const specialist = pos === 'K' || pos === 'P';
  const mean = slot < starters ? 78 + 3 * talent : slot < starters + rotation ? 73 + 2 * talent : 69 + 1.5 * talent;
  const sd = slot < starters ? 4.5 : 3.5;
  const age = young ? rng.normal(23.5, 1.5) : specialist ? rng.normal(30, 4) : rng.normal(27, 3);
  return { rating: clamp(rng.normal(mean - (young ? 2 : 0), sd), 55, 97), age: Math.round(clamp(age, 21, 38)) };
}

export function makeLeague(rng: Rng): Team[] {
  return NFL_TEAM_NAMES.map((name, id) => {
    const team: Team = {
      id,
      name,
      draftSkill: clamp(Math.round(rng.normal(10.5, 3.5)), 1, 20),
      owner: clamp(Math.round(rng.normal(10.5, 3.5)), 1, 20),
      coaching: clamp(Math.round(rng.normal(13, 3)), 1, 20),
      depth: {} as Record<Position, Member[]>,
      staffYears: rng.int(0, 5),
    };
    const talent = talentOf(team, rng);
    for (const pos of POSITIONS) {
      team.depth[pos] = Array.from({ length: DEPTH_SLOTS[pos].roster }, (_, slot) => newBackground(rng, pos, slot, talent, false));
    }
    return team;
  });
}

/** Roster talent a team tends to acquire: well-run teams find better players, with plenty of luck. */
function talentOf(team: Team, rng: Rng): number {
  return 0.5 * rng.normal() + 0.5 * ((organisation(team) - 11) / 3);
}

/**
 * One offseason for the background players: they age, develop or decline,
 * and leave (retirement, released, signed elsewhere); their places are filled
 * by other classes' rookies and free agents. Staffs get fired, more often
 * after bad years.
 */
export function advanceLeague(teams: Team[], rng: Rng): void {
  const strengths = teams.map(teamStrength).sort((a, b) => a - b);
  const bottom = strengths[7]!;
  for (const team of teams) {
    const weak = teamStrength(team) <= bottom;
    team.staffYears++;
    if (rng.chance(weak ? 0.3 : 0.1)) {
      team.coaching = clamp(Math.round(rng.normal(13, 3)), 1, 20);
      team.staffYears = 0;
    }
    if (rng.chance(weak ? 0.15 : 0.05)) team.draftSkill = clamp(Math.round(rng.normal(10.5, 3.5)), 1, 20);
    const talent = talentOf(team, rng);
    for (const pos of POSITIONS) {
      const kept: Member[] = [];
      const open: number[] = [];
      const ranked = [...team.depth[pos]].sort((x, y) => y.rating - x.rating);
      ranked.forEach((m, slot) => {
        if (m.id) {
          kept.push(m);
          return;
        }
        m.age++;
        m.rating = clamp(m.rating + backgroundAgeing(pos, m.age) + rng.normal(0, 2), 40, 97);
        const leaves = m.age > BACKGROUND_EXIT_AGE[pos] + rng.normal(0, 2) || m.rating < 60 || rng.chance(0.12);
        if (leaves) open.push(slot);
        else kept.push(m);
      });
      // Fill back to a full depth chart. A team that loses a starter often
      // replaces him with someone a level down.
      const roster = DEPTH_SLOTS[pos].roster;
      // Practice squad players don't hold a place on the 53.
      const onRoster = () => kept.filter((m) => m.role !== 'practiceSquad').length;
      for (let i = 0; onRoster() < roster; i++) {
        const slot = open[i] ?? roster - 1;
        kept.push(newBackground(rng, pos, rng.chance(0.65) ? slot : Math.min(slot + 1, roster - 1), talent, rng.chance(0.6)));
      }
      team.depth[pos] = kept;
    }
  }
}
