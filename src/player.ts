import {
  ATTRIBUTE_KEYS,
  ATTRIBUTE_PROFILES,
  MAX_RATING,
  MIN_RATING,
  emptyAttributes,
  type Attributes,
} from './attributes.js';
import { POSITIONS, POSITION_PROFILES, eliteShapeOverall, eliteShapeValue, overallFor, type Position } from './positions.js';
import { Rng, clamp } from './rng.js';

export type Level = 'HS' | 'FBS' | 'FCS' | 'SmallCollege' | 'NFL';

/** Hidden personality traits on Football Manager's 1-20 scale. */
export interface Personality {
  /** Effort in training: speeds growth, slows decline, pushes potential up. */
  workEthic: number;
  /** How well instruction sticks: amplifies good coaching and protects against bad. */
  coachability: number;
  /** Drive to beat the man across from him: helps breakouts and protects against stagnation. */
  competitiveness: number;
}

export interface SeasonRecord {
  age: number;
  level: Level;
  playingTime: PlayingTime;
  /** Rating going into the season: what he played the season at, and what his role was picked from. */
  overallStart: number;
  /** Rating at the end of the year, after the season and the offseason that follows. */
  overall: number;
  potential: number;
  /**
   * On-field level shown that season: his rating during the season plus form,
   * slumps and breakouts. Null for a season without games (a redshirt year or
   * one lost to suspension or dismissal).
   */
  performance: number | null;
  /**
   * 1-99 for the rating he played the season at, placed against his own
   * position and then ranked nationally: high schoolers against everyone their
   * age, college players against all FBS players, pros simply their overall
   * (filled in by a cohort simulation).
   */
  levelRating?: number;
  /** Where his rating at the end of the year ranks against everyone else's at the end of theirs. */
  levelRatingEnd?: number;
  /** Quality of the coaching he had that season, 1-20. */
  coaching: number;
  events: DevelopmentEvent[];
}

export type PlayingTime = 'starter' | 'rotation' | 'backup' | 'redshirt';

export type DevelopmentEvent =
  | 'growthSpurt'
  /** His body stopped developing: physical ceilings settle where they are and don't rise again. */
  | 'bodyStalled'
  | 'breakout'
  | 'stagnation'
  | 'lostSeason'
  /** Suspended or academically ineligible for the season. */
  | 'suspended'
  /** Dismissed from his college team. */
  | 'dismissed'
  | 'slump'
  | 'minorInjury'
  | 'moderateInjury'
  | 'majorInjury';

export interface Player {
  id: string;
  position: Position;
  age: number;
  /** Current ability, visible on the field. */
  attributes: Attributes;
  /** Hidden ceiling per attribute. Moves over time; never shown directly. */
  ceilings: Attributes;
  /** Potential when the player was generated, kept to show how much was gained or lost. */
  originalPotential: number;
  /** Years the player's growth window is shifted: negative = early bloomer, positive = late bloomer. */
  bloom: number;
  /** Years added to (or removed from) every decline age. */
  longevity: number;
  /** 1-20, higher means more injuries. */
  injuryProneness: number;
  personality: Personality;
  history: SeasonRecord[];
}

/** Current overall rating at the player's position. */
export function overall(p: Player): number {
  return overallFor(p.position, p.attributes);
}

/**
 * True potential: the overall the player would have if every attribute reached
 * its current ceiling. Hidden from the user, who only sees scouted estimates.
 */
export function potential(p: Player): number {
  return overallFor(p.position, p.ceilings);
}

/**
 * Talent at maturity for a random high school player. A normal core with a
 * stretched upper tail: most kids top out as decent high school players, a few
 * per thousand have a college ceiling, and a handful per national class have
 * an elite pro ceiling.
 */
export function talentFromZ(z: number): number {
  const tail = Math.max(0, z - 1.6);
  return 45 + 6.5 * z + 2.6 * tail * tail;
}

export interface ProspectOptions {
  id?: string;
  position?: Position;
  age?: number;
  /** Override the latent talent draw (standard normal z-score). */
  talentZ?: number;
}

let nextId = 1;

export function generateProspect(rng: Rng, opts: ProspectOptions = {}): Player {
  const position = opts.position ?? rng.weighted(POSITIONS.map((p) => [p, POSITION_PROFILES[p].share] as const));
  const age = opts.age ?? 14;
  const talent = clamp(talentFromZ(opts.talentZ ?? rng.normal()), 20, 99);
  const scale = talent / eliteShapeOverall(position);

  const bloom = clamp(rng.normal(0, 1.1), -2.5, 3);
  const ceilings = emptyAttributes();
  const attributes = emptyAttributes();
  for (const key of ATTRIBUTE_KEYS) {
    const ceiling = clamp(eliteShapeValue(position, key) * scale + rng.normal(0, 4), MIN_RATING, MAX_RATING);
    ceilings[key] = ceiling;
    const profile = ATTRIBUTE_PROFILES[key];
    // Early bloomers arrive more physically mature; late bloomers less so.
    const maturity = clamp(profile.maturityAt14 - 0.035 * bloom + 0.03 * (age - 14) + rng.normal(0, 0.05), 0.2, 0.98);
    // Even the most gifted 14-year-old is still a boy: soften the top end.
    const raw = ceiling * maturity;
    attributes[key] = clamp(raw > 40 ? 40 + (raw - 40) * 0.5 : raw, MIN_RATING, ceiling);
  }

  return {
    originalPotential: overallFor(position, ceilings),
    id: opts.id ?? `p${nextId++}`,
    position,
    age,
    attributes,
    ceilings,
    bloom,
    longevity: clamp(rng.normal(0, 1.3), -3, 3.5),
    injuryProneness: clamp(Math.round(rng.normal(10, 3.5)), 1, 20),
    personality: {
      workEthic: clamp(Math.round(rng.normal(10, 3.5)), 1, 20),
      coachability: clamp(Math.round(rng.normal(10, 3.5)), 1, 20),
      competitiveness: clamp(Math.round(rng.normal(10, 3.5)), 1, 20),
    },
    history: [],
  };
}
