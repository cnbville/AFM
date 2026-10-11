import { potentialPlasticity, POTENTIAL_BAND } from './development.js';
import { overall, potential, type Level, type Player } from './player.js';
import { Rng, clamp } from './rng.js';

/**
 * What a staff member believes about a player. The user never sees true
 * ratings: only reports like this, as good as the scout who wrote them.
 */
export interface ScoutReport {
  overall: number;
  /** Central estimate of potential. */
  potential: number;
  /** Displayed range, rounded out to whole potential bands. */
  potentialLow: number;
  potentialHigh: number;
  /** Football Manager style stars (0.5-5) for current ability and potential. */
  currentStars: number;
  potentialStars: number;
}

/** How hard a level is to evaluate: small schools play weaker opponents and get less tape. */
const LEVEL_VISIBILITY: Record<Level, number> = {
  HS: 1.25,
  SmallCollege: 1.5,
  FCS: 1.2,
  FBS: 1,
  NFL: 0.7,
};

export interface ScoutOptions {
  /** Scout's judging ability, 1-20. */
  accuracy: number;
  level: Level;
}

export function scoutPlayer(player: Player, opts: ScoutOptions, rng: Rng): ScoutReport {
  const skill = 1.4 - opts.accuracy * 0.04; // 10 -> 1.0, 20 -> 0.6
  const visibility = LEVEL_VISIBILITY[opts.level];
  const trueOverall = overall(player);
  const truePotential = potential(player);

  const overallSd = 2.5 * skill * visibility;
  // Potential is hardest to read in young players whose ceilings still move.
  const potentialSd = (2.5 + 7 * potentialPlasticity(player.age)) * skill * visibility;

  const estOverall = clamp(trueOverall + rng.normal(0, overallSd), 1, 99);
  const estPotential = clamp(Math.max(estOverall, truePotential + rng.normal(0, potentialSd)), 1, 99);
  const halfWidth = potentialSd;
  return {
    overall: estOverall,
    potential: estPotential,
    potentialLow: Math.max(estOverall, Math.floor((estPotential - halfWidth) / POTENTIAL_BAND) * POTENTIAL_BAND),
    potentialHigh: Math.min(99, Math.ceil((estPotential + halfWidth) / POTENTIAL_BAND) * POTENTIAL_BAND),
    currentStars: toStars(estOverall),
    potentialStars: toStars(estPotential),
  };
}

/** Map a rating to half-stars on a pro scale: 50 = 1 star, 70 = 3 stars, 90 = 5 stars. */
export function toStars(rating: number): number {
  return clamp(Math.round((rating - 40) / 5) / 2, 0.5, 5);
}
