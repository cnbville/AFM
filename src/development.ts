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
  /** Off-field or academic trouble that costs him this season's games. */
  trouble?: 'suspended' | 'dismissed';
}

/** Potential is reported in bands of this many points (one half-star). */
export const POTENTIAL_BAND = 5;
/**
 * Most the yearly drift may move a player's true potential in one checkpoint,
 * up or down. A major injury, a body that stops developing and a proven
 * breakout can move it further, and a ceiling also rises when a player
 * outgrows it.
 */
export const MAX_POTENTIAL_SHIFT = POTENTIAL_BAND;

const LEVEL_INJURY_RATE: Record<Level, number> = {
  HS: 0.13,
  SmallCollege: 0.16,
  FCS: 0.17,
  FBS: 0.19,
  NFL: 0.24,
};

/** Points per unit of growth rate added on top of proportional growth. */
const FINISHING_STEP = 1.6;
const MAX_YEARLY_GAIN = 12;
/** Yearly chance, at 18-20, that an average bloomer's body stops developing (higher for early bloomers). */
const STALL_CHANCE = 0.025;

const PROFILE_LIST = ATTRIBUTE_KEYS.map((k) => ATTRIBUTE_PROFILES[k]);
const ceilingDeltaBuffer = new Float64Array(ATTRIBUTE_KEYS.length);
const settlingBuffer = new Uint8Array(ATTRIBUTE_KEYS.length);
const deltaAttrs = emptyAttributes(0);

/**
 * The quality of the man across from you. A D2 star dominates but is rarely
 * pushed, so what he learns from games (reading defenses, technique) grows
 * more slowly. This barely touches physical growth: weight rooms and strength
 * staffs come through the coaching rating, which averages lower at D2.
 */
const LEVEL_GROWTH: Record<Level, number> = {
  HS: 1,
  SmallCollege: 0.72,
  FCS: 0.92,
  FBS: 1,
  NFL: 1,
};

const PLAYING_TIME_GROWTH: Record<PlayingTime, number> = {
  starter: 1.15,
  rotation: 1.0,
  backup: 0.8,
  // A redshirt year is a full year of practice and weight room without game snaps.
  redshirt: 0.85,
};

const PLAYING_TIME_POTENTIAL: Record<PlayingTime, number> = {
  starter: 0.15,
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
    // A suspended or dismissed player is at most at practice, like a redshirt.
    (ctx.trouble ? 0.5 : ctx.playingTime === 'starter' ? 1.15 : ctx.playingTime === 'redshirt' ? 0.5 : 0.9);
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

  // --- A body that stops growing ----------------------------------------------
  // High school stars who were simply bigger and faster than boys their age
  // often stop growing in college while everyone else catches up. Early
  // bloomers are the most likely to be done.
  // Once stalled, his physical ceilings stay where they settled.
  let stalled = player.history.some((h) => h.events.includes('bodyStalled'));
  if (age >= 18 && age <= 20 && !stalled && rng.chance(STALL_CHANCE * Math.exp(-0.7 * player.bloom))) {
    stalled = true;
    events.push('bodyStalled');
    for (let i = 0; i < ATTRIBUTE_KEYS.length; i++) {
      if (PROFILE_LIST[i]!.group !== 'physical') continue;
      const key = ATTRIBUTE_KEYS[i]!;
      player.ceilings[key] = Math.min(player.ceilings[key], player.attributes[key] + rng.range(0, 2));
    }
  }

  // --- Form for the year: breakouts, slumps and lost seasons ----------------
  // Development is lumpy: a single season can be worth three normal ones, and a
  // good year is often followed by a dip as opponents adjust.
  let form = rng.logNormal(0.38);
  let setback = 0;
  let performance = rng.normal(0, 2);
  let breakoutLift = 0;
  const last = player.history[player.history.length - 1];
  const before = player.history[player.history.length - 2];
  const hotLastYear = !!last && (last.events.includes('breakout') || (!!before && last.overall - before.overall >= 6));
  const youngStarter = ctx.playingTime === 'starter' && age <= 22;
  const breakoutChance =
    (age <= 23 ? 0.035 : age <= 27 ? 0.02 : 0.01) *
    (1 + (personality.competitiveness - 10) * 0.05 + (personality.workEthic - 10) * 0.05) *
    (youngStarter ? 1.5 : ctx.playingTime === 'backup' ? 0.6 : 1);
  const slumpChance = hotLastYear ? 0.16 : 0.06;
  const stagnationChance = 0.05 - (personality.workEthic - 10) * 0.003 - (personality.competitiveness - 10) * 0.0015;
  // Complacency, a bad scheme fit, a coach who never trusts him: the quiet reasons prospects bust.
  const lostSeasonChance =
    age >= 17 && age <= 27
      ? Math.max(0.005, 0.03 + (10 - personality.workEthic) * 0.004 + (10 - personality.competitiveness) * 0.002)
      : 0;
  if (ctx.trouble) {
    // A year away from games and from the staff's trust: like a lost season, and it shows on the field.
    events.push(ctx.trouble);
    form *= 0.4;
    setback = 1.8;
    performance -= rng.range(3, 7);
  } else if (ctx.playingTime !== 'redshirt' && snapsFactor >= 0.65 && rng.chance(breakoutChance)) {
    // (No breakout in a redshirt year or one mostly lost to injury.)
    events.push('breakout');
    form *= 2.2;
    performance += rng.range(3, 7);
    // A proven breakout by a player already pressing against his ceiling shows
    // the ceiling was wrong: it can rise past the normal yearly limit, by less
    // as a player gets older. Further from the ceiling, a breakout is growth.
    const headroom = potentialBefore - overallBefore;
    if (headroom < 6) breakoutLift = rng.range(2, 8) * (age <= 23 ? 1 : age <= 27 ? 0.6 : 0.35);
  } else if (rng.chance(lostSeasonChance)) {
    events.push('lostSeason');
    form *= 0.4;
    setback = 1.8;
    performance -= rng.range(3, 7);
  } else if (ctx.playingTime !== 'redshirt' && rng.chance(slumpChance)) {
    // Production falls hard; the ratings dip a little as confidence and timing go.
    events.push('slump');
    form *= 0.6;
    performance -= rng.range(4, 9);
    for (let i = 0; i < ATTRIBUTE_KEYS.length; i++) {
      if (PROFILE_LIST[i]!.group !== 'physical') player.attributes[ATTRIBUTE_KEYS[i]!] -= rng.range(0.5, 3);
    }
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
    const shared = drive * 1.7 * plasticity;
    const spurt = age <= 17 && rng.chance(0.09);
    if (spurt) events.push('growthSpurt');
    const delta = ceilingDeltaBuffer;
    for (let i = 0; i < ATTRIBUTE_KEYS.length; i++) {
      const key = ATTRIBUTE_KEYS[i]!;
      let d = shared + rng.normal(0, 1.2 * plasticity);
      if (spurt && PROFILE_LIST[i]!.group === 'physical') d += rng.range(2, 6) * (key === 'strength' ? 1.3 : 0.8);
      if (stalled && d > 0 && PROFILE_LIST[i]!.group === 'physical') d = 0;
      const current = player.ceilings[key];
      delta[i] = clamp(current + d, MIN_RATING, MAX_RATING) - current;
      deltaAttrs[key] = delta[i]!;
    }
    // A potential shift can move at most one band per checkpoint: no overnight stars.
    const shift = overallFor(player.position, deltaAttrs);
    const scale = Math.abs(shift) > MAX_POTENTIAL_SHIFT ? MAX_POTENTIAL_SHIFT / Math.abs(shift) : 1;
    for (let i = 0; i < ATTRIBUTE_KEYS.length; i++) player.ceilings[ATTRIBUTE_KEYS[i]!] += delta[i]! * scale;
  }
  // Reading defenses, poise and route timing keep deepening with real snaps
  // well after the body has stopped growing: veterans earn new ceiling and
  // most of it shows up on the field straight away.
  if (age >= 21 && age <= 31 && !ctx.trouble) {
    const snapsShare = ctx.playingTime === 'starter' ? 1 : ctx.playingTime === 'rotation' ? 0.6 : 0.2;
    for (let i = 0; i < ATTRIBUTE_KEYS.length; i++) {
      const profile = PROFILE_LIST[i]!;
      if (profile.experience < 0.6) continue;
      const learned = rng.range(0, 1.2) * profile.experience * snapsShare * snapsFactor;
      player.ceilings[ATTRIBUTE_KEYS[i]!] += learned;
      player.attributes[ATTRIBUTE_KEYS[i]!] += learned * 0.8;
    }
  }
  if (breakoutLift > 0) {
    const weights = POSITION_PROFILES[player.position].weights;
    for (let i = 0; i < ATTRIBUTE_KEYS.length; i++) {
      const key = ATTRIBUTE_KEYS[i]!;
      if (!weights[key] || (stalled && PROFILE_LIST[i]!.group === 'physical')) continue;
      player.ceilings[key] = Math.min(MAX_RATING, player.ceilings[key] + breakoutLift * rng.range(0.6, 1.2));
    }
  }

  // --- Growth towards the ceiling --------------------------------------------
  const coachingMult = 0.75 + ctx.coaching * 0.025; // 10 -> 1.0, 20 -> 1.25
  const coachabilityLean = 0.6 + personality.coachability * 0.04; // how much of coaching sticks
  const workMult = 0.75 + personality.workEthic * 0.025;
  const snapsMult = PLAYING_TIME_GROWTH[ctx.playingTime] * (0.7 + 0.3 * snapsFactor) * LEVEL_GROWTH[ctx.level];
  const developmentAge = age - player.bloom;
  // Coachability scales how far coaching moves him from average, good or bad.
  const coachingBoost = 1 + (coachingMult - 1) * coachabilityLean;
  for (let i = 0; i < ATTRIBUTE_KEYS.length; i++) {
    const key = ATTRIBUTE_KEYS[i]!;
    const profile = PROFILE_LIST[i]!;
    const ageFactor = growthAgeFactor(developmentAge, profile.growthWindow);
    settlingBuffer[i] = ageFactor <= 0.01 ? 1 : 0;
    const gap = player.ceilings[key] - player.attributes[key];
    if (gap <= 0 || ageFactor <= 0.01) continue;
    const coaching = Math.pow(coachingBoost, profile.coachability);
    const snaps = Math.pow(snapsMult, profile.experience);
    const redshirtLift = ctx.playingTime === 'redshirt' && profile.group === 'physical' ? 1.35 : 1;
    const rate = clamp(profile.growthRate * ageFactor * coaching * snaps * workMult * form * growthMultiplier * redshirtLift, 0, 0.5);
    // Proportional growth plus a fixed step, so a well-developed player can
    // actually reach his ceiling instead of creeping towards it forever.
    // Even a monster year adds at most about a dozen points to one attribute.
    player.attributes[key] += Math.min(gap, gap * rate + FINISHING_STEP * rate, MAX_YEARLY_GAIN) + rng.normal(0, 0.5);
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
    settlingBuffer[i] = 1;
    const loss = profile.declineRate * yearsPast * declineResistance * rng.logNormal(0.3);
    player.attributes[key] -= loss;
    player.ceilings[key] -= loss;
  }

  // --- Tidy up ---------------------------------------------------------------
  for (let i = 0; i < ATTRIBUTE_KEYS.length; i++) {
    const key = ATTRIBUTE_KEYS[i]!;
    // Once an attribute has stopped growing or started to decline, the ceiling
    // above it is out of reach: veterans don't keep upside they can't use.
    if (settlingBuffer[i] && player.ceilings[key] > player.attributes[key]) {
      player.ceilings[key] = player.attributes[key] + (player.ceilings[key] - player.attributes[key]) / 2;
    }
    player.ceilings[key] = clamp(player.ceilings[key], MIN_RATING, MAX_RATING);
    player.attributes[key] = clamp(player.attributes[key], MIN_RATING, MAX_RATING);
    // Over-performing a ceiling means the ceiling was wrong, not the player.
    if (player.attributes[key] > player.ceilings[key]) player.ceilings[key] = player.attributes[key];
  }

  const overallAfter = overallFor(player.position, player.attributes);
  const potentialAfter = potential(player);
  // A big year on the field from a player whose rating still fell is a career year, not a breakout.
  if (overallAfter < overallBefore && events.includes('breakout')) events.splice(events.indexOf('breakout'), 1);
  // He improves (or fades) through the year, so he plays the season between his two ratings.
  const noGames = ctx.playingTime === 'redshirt' || !!ctx.trouble;
  player.history.push({
    age,
    level: ctx.level,
    playingTime: ctx.playingTime,
    overallStart: overallBefore,
    overall: overallAfter,
    potential: potentialAfter,
    performance: noGames ? null : clamp((overallBefore + overallAfter) / 2 + performance, MIN_RATING, MAX_RATING),
    coaching: ctx.coaching,
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
