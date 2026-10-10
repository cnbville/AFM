import { ATTRIBUTE_KEYS, ATTRIBUTE_PROFILES, MAX_RATING, MIN_RATING, emptyAttributes } from './attributes.js';
import { POSITION_PROFILES, overallFor } from './positions.js';
import { potential, type DevelopmentEvent, type Level, type Player, type PlayingTime } from './player.js';
import { Rng, clamp } from './rng.js';

/**
 * One development checkpoint covers a full football year: the season plus the
 * offseason that follows. It is meant to run at the yearly potential review
 * (W1 of the AFM calendar), after which the player is a year older.
 */
export interface SeasonContext {
  level: Level;
  /** Quality of the player's coaching staff, 1-20. */
  coaching: number;
  playingTime: PlayingTime;
}

/** Potential is reported in bands of this many points (one half-star). */
export const POTENTIAL_BAND = 5;
/** Most a player's true potential may move in one checkpoint, up or down, outside of a major injury. */
export const MAX_POTENTIAL_SHIFT = POTENTIAL_BAND;

const LEVEL_INJURY_RATE: Record<Level, number> = {
  HS: 0.13,
  SmallCollege: 0.16,
  FCS: 0.17,
  FBS: 0.19,
  NFL: 0.24,
};

const PROFILE_LIST = ATTRIBUTE_KEYS.map((k) => ATTRIBUTE_PROFILES[k]);
const ceilingDeltaBuffer = new Float64Array(ATTRIBUTE_KEYS.length);
const deltaAttrs = emptyAttributes(0);

const PLAYING_TIME_GROWTH: Record<PlayingTime, number> = {
  starter: 1.15,
  rotation: 1.0,
  backup: 0.8,
  // A redshirt year is a full year of practice and weight room without game snaps.
  redshirt: 0.85,
};

const PLAYING_TIME_POTENTIAL: Record<PlayingTime, number> = {
  starter: 0.35,
  rotation: 0.1,
  backup: -0.25,
  redshirt: 0,
};

export interface SeasonReport {
  overallBefore: number;
  overallAfter: number;
  potentialBefore: number;
  potentialAfter: number;
  events: DevelopmentEvent[];
}

/** Develop a player through one year and age him by one. Mutates the player. */
export function developSeason(player: Player, ctx: SeasonContext, rng: Rng): SeasonReport {
  const { personality } = player;
  const overallBefore = overallFor(player.position, player.attributes);
  const potentialBefore = potential(player);
  const events: DevelopmentEvent[] = [];
  const age = player.age;

  // --- Injuries: the main source of setbacks -------------------------------
  let growthMultiplier = 1;
  let snapsFactor = 1;
  const injuryChance =
    LEVEL_INJURY_RATE[ctx.level] *
    POSITION_PROFILES[player.position].injuryRisk *
    (0.55 + player.injuryProneness * 0.045) *
    (ctx.playingTime === 'starter' ? 1.15 : ctx.playingTime === 'redshirt' ? 0.5 : 0.9);
  if (rng.chance(injuryChance)) {
    const roll = rng.next();
    const severity: DevelopmentEvent = roll < 0.62 ? 'minorInjury' : roll < 0.89 ? 'moderateInjury' : 'majorInjury';
    events.push(severity);
    if (severity === 'minorInjury') {
      growthMultiplier *= 0.95;
      snapsFactor = 0.9;
    } else if (severity === 'moderateInjury') {
      growthMultiplier *= 0.85;
      snapsFactor = 0.65;
    } else {
      growthMultiplier *= 0.5;
      snapsFactor = 0.2;
      // Torn ACLs and Achilles tendons take explosiveness, more so with age.
      const ageHit = age >= 28 ? 1.6 : age >= 24 ? 1.2 : 1;
      for (const key of ATTRIBUTE_KEYS) {
        if (!ATTRIBUTE_PROFILES[key].injurySensitive) continue;
        player.attributes[key] -= rng.range(1.5, 5) * ageHit;
        player.ceilings[key] -= rng.range(0.5, 4) * ageHit;
      }
      player.injuryProneness = Math.min(20, player.injuryProneness + 1);
    }
  }

  // --- Form for the year: breakouts, stagnation and lost seasons -------------
  let form = rng.logNormal(0.22);
  let setback = 0;
  const breakoutChance = age <= 25 ? 0.035 + (personality.competitiveness - 10) * 0.002 + (personality.workEthic - 10) * 0.002 : 0;
  const stagnationChance = 0.05 - (personality.workEthic - 10) * 0.003 - (personality.competitiveness - 10) * 0.0015;
  // Complacency, off-field trouble, a bad scheme fit: the quiet reasons prospects bust.
  const lostSeasonChance =
    age >= 17 && age <= 27
      ? Math.max(0.005, 0.03 + (10 - personality.workEthic) * 0.004 + (10 - personality.competitiveness) * 0.002)
      : 0;
  if (rng.chance(breakoutChance)) {
    events.push('breakout');
    form *= 1.7;
  } else if (rng.chance(lostSeasonChance)) {
    events.push('lostSeason');
    form *= 0.5;
    setback = 1.8;
  } else if (rng.chance(stagnationChance)) {
    events.push('stagnation');
    form *= 0.3;
  }

  // --- Dynamic potential -----------------------------------------------------
  // Ceilings move most in the mid-teens, settle through the early 20s and only
  // fall after that. Direction is driven by personality, coaching and snaps;
  // luck plays a smaller part.
  const plasticity = potentialPlasticity(age);
  if (plasticity > 0) {
    const drive =
      ((personality.workEthic - 10) / 10) * 0.75 +
      ((personality.coachability - 10) / 10) * 0.25 * ((ctx.coaching - 10) / 10 + 0.5) +
      ((ctx.coaching - 10) / 10) * 0.35 +
      PLAYING_TIME_POTENTIAL[ctx.playingTime] * snapsFactor -
      setback +
      rng.normal(0, 0.9);
    const shared = drive * 2.4 * plasticity;
    const spurt = age <= 17 && rng.chance(0.09);
    if (spurt) events.push('growthSpurt');
    const delta = ceilingDeltaBuffer;
    for (let i = 0; i < ATTRIBUTE_KEYS.length; i++) {
      const key = ATTRIBUTE_KEYS[i]!;
      let d = shared + rng.normal(0, 1.2 * plasticity);
      if (spurt && PROFILE_LIST[i]!.group === 'physical') d += rng.range(2, 6) * (key === 'strength' ? 1.3 : 0.8);
      const current = player.ceilings[key];
      delta[i] = clamp(current + d, MIN_RATING, MAX_RATING) - current;
      deltaAttrs[key] = delta[i]!;
    }
    // A potential shift can move at most one band per checkpoint: no overnight stars.
    const shift = overallFor(player.position, deltaAttrs);
    const scale = Math.abs(shift) > MAX_POTENTIAL_SHIFT ? MAX_POTENTIAL_SHIFT / Math.abs(shift) : 1;
    for (let i = 0; i < ATTRIBUTE_KEYS.length; i++) player.ceilings[ATTRIBUTE_KEYS[i]!] += delta[i]! * scale;
  }

  // --- Growth towards the ceiling --------------------------------------------
  const coachingMult = 0.75 + ctx.coaching * 0.025; // 10 -> 1.0, 20 -> 1.25
  const coachabilityLean = 0.6 + personality.coachability * 0.04; // how much of coaching sticks
  const workMult = 0.75 + personality.workEthic * 0.025;
  const snapsMult = PLAYING_TIME_GROWTH[ctx.playingTime] * (0.7 + 0.3 * snapsFactor);
  const developmentAge = age - player.bloom;
  const coachingBoost = coachingMult * (coachingMult >= 1 ? coachabilityLean : 1 / coachabilityLean);
  for (let i = 0; i < ATTRIBUTE_KEYS.length; i++) {
    const key = ATTRIBUTE_KEYS[i]!;
    const profile = PROFILE_LIST[i]!;
    const gap = player.ceilings[key] - player.attributes[key];
    if (gap <= 0) continue;
    const ageFactor = growthAgeFactor(developmentAge, profile.growthWindow);
    if (ageFactor <= 0.01) continue;
    const coaching = Math.pow(coachingBoost, profile.coachability);
    const snaps = Math.pow(snapsMult, profile.experience);
    const rate = clamp(profile.growthRate * ageFactor * coaching * snaps * workMult * form * growthMultiplier, 0, 0.75);
    player.attributes[key] += gap * rate + rng.normal(0, 0.5);
  }

  // --- Ageing ----------------------------------------------------------------
  const declineResistance = 1.15 - personality.workEthic * 0.015; // 10 -> 1.0, 20 -> 0.85
  const posOffset = POSITION_PROFILES[player.position].agingOffset;
  for (let i = 0; i < ATTRIBUTE_KEYS.length; i++) {
    const key = ATTRIBUTE_KEYS[i]!;
    const profile = PROFILE_LIST[i]!;
    const onset = profile.declineAge + player.longevity + (profile.group === 'physical' ? Math.min(posOffset, 1.5) : posOffset);
    const yearsPast = age + 1 - onset;
    if (yearsPast <= 0) continue;
    const loss = profile.declineRate * yearsPast * declineResistance * rng.logNormal(0.3);
    player.attributes[key] -= loss;
    player.ceilings[key] -= loss;
  }

  // --- Tidy up ---------------------------------------------------------------
  for (const key of ATTRIBUTE_KEYS) {
    player.ceilings[key] = clamp(player.ceilings[key], MIN_RATING, MAX_RATING);
    player.attributes[key] = clamp(player.attributes[key], MIN_RATING, MAX_RATING);
    // Over-performing a ceiling means the ceiling was wrong, not the player.
    if (player.attributes[key] > player.ceilings[key]) player.ceilings[key] = player.attributes[key];
  }

  const overallAfter = overallFor(player.position, player.attributes);
  const potentialAfter = potential(player);
  player.history.push({
    age,
    level: ctx.level,
    playingTime: ctx.playingTime,
    overall: overallAfter,
    potential: potentialAfter,
    events,
  });
  player.age += 1;
  return { overallBefore, overallAfter, potentialBefore, potentialAfter, events };
}

/** How freely a ceiling can still move at a given age (0 = only falls with age). */
export function potentialPlasticity(age: number): number {
  if (age <= 17) return 1;
  if (age <= 20) return 0.75;
  if (age <= 23) return 0.5;
  if (age <= 26) return 0.2;
  return 0;
}

/** Share of the normal growth rate available at a development age. */
export function growthAgeFactor(age: number, window: readonly [number, number]): number {
  const [start, end] = window;
  if (age < start) return 0.75;
  if (age <= end) return 1;
  return Math.pow(0.55, age - end);
}
