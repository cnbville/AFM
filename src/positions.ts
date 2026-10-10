import { ATTRIBUTE_KEYS, type AttributeKey, type Attributes } from './attributes.js';

export const POSITIONS = ['QB', 'RB', 'WR', 'TE', 'OT', 'IOL', 'EDGE', 'DT', 'LB', 'CB', 'S', 'K', 'P'] as const;
export type Position = (typeof POSITIONS)[number];

type Partial99 = Partial<Record<AttributeKey, number>>;

export interface PositionProfile {
  /** Relative importance of each attribute to the position's overall rating. */
  weights: Partial99;
  /**
   * Attribute shape of an elite player at the position. Ceilings are drawn by
   * scaling this shape to the player's talent, so a great guard is strong and
   * slow while a great corner is fast and light.
   */
  eliteShape: Partial99;
  /** Years added to every attribute's decline age (QBs and specialists last longer). */
  agingOffset: number;
  /** Relative chance of injury per season (1 = league average). */
  injuryRisk: number;
  /** Share of high school players listed at the position. */
  share: number;
}

const DEFAULT_SHAPE = 45;

export const POSITION_PROFILES: Record<Position, PositionProfile> = {
  QB: {
    weights: { throwAccuracy: 5, throwPower: 3, awareness: 4, poise: 3, vision: 1, agility: 0.5, speed: 0.5 },
    eliteShape: { throwAccuracy: 96, throwPower: 95, awareness: 96, poise: 95, vision: 85, speed: 72, acceleration: 74, agility: 75, strength: 60, stamina: 80, carrying: 65, jumping: 60 },
    agingOffset: 3,
    injuryRisk: 0.7,
    share: 0.06,
  },
  RB: {
    weights: { speed: 3, acceleration: 3, agility: 3, elusiveness: 3, vision: 3, carrying: 2, strength: 1.5, catching: 1, passBlocking: 0.5, awareness: 1 },
    eliteShape: { speed: 94, acceleration: 95, agility: 95, elusiveness: 95, vision: 94, carrying: 92, strength: 80, catching: 78, passBlocking: 65, awareness: 85, jumping: 80, stamina: 85, routeRunning: 65 },
    agingOffset: -1.5,
    injuryRisk: 1.4,
    share: 0.09,
  },
  WR: {
    weights: { catching: 4, routeRunning: 4, speed: 3, acceleration: 2, agility: 1.5, jumping: 1, elusiveness: 1, awareness: 1.5 },
    eliteShape: { catching: 95, routeRunning: 95, speed: 95, acceleration: 94, agility: 93, jumping: 90, elusiveness: 88, awareness: 88, strength: 62, stamina: 85, carrying: 75, vision: 80 },
    agingOffset: 0,
    injuryRisk: 1.0,
    share: 0.15,
  },
  TE: {
    weights: { catching: 3, routeRunning: 2, runBlocking: 2.5, passBlocking: 1, strength: 2, speed: 1.5, awareness: 1.5 },
    eliteShape: { catching: 92, routeRunning: 88, runBlocking: 88, passBlocking: 82, strength: 86, speed: 84, acceleration: 84, agility: 80, awareness: 90, jumping: 82, stamina: 85, carrying: 75 },
    agingOffset: 0.5,
    injuryRisk: 1.1,
    share: 0.06,
  },
  OT: {
    weights: { passBlocking: 5, runBlocking: 3, strength: 3, agility: 1.5, awareness: 2 },
    eliteShape: { passBlocking: 96, runBlocking: 92, strength: 94, agility: 75, awareness: 92, acceleration: 70, speed: 58, stamina: 85 },
    agingOffset: 1.5,
    injuryRisk: 0.9,
    share: 0.09,
  },
  IOL: {
    weights: { runBlocking: 5, passBlocking: 3, strength: 4, awareness: 2, agility: 0.5 },
    eliteShape: { runBlocking: 96, passBlocking: 92, strength: 96, awareness: 92, agility: 68, acceleration: 66, speed: 55, stamina: 85 },
    agingOffset: 1.5,
    injuryRisk: 0.9,
    share: 0.13,
  },
  EDGE: {
    weights: { passRush: 5, blockShedding: 3, acceleration: 2.5, speed: 2, strength: 2, tackling: 1.5, playRecognition: 1.5 },
    eliteShape: { passRush: 96, blockShedding: 92, acceleration: 92, speed: 86, strength: 88, tackling: 85, playRecognition: 88, agility: 82, stamina: 82, awareness: 82 },
    agingOffset: 0,
    injuryRisk: 1.0,
    share: 0.08,
  },
  DT: {
    weights: { blockShedding: 4, strength: 4.5, passRush: 2.5, tackling: 2, playRecognition: 1.5, acceleration: 1 },
    eliteShape: { blockShedding: 95, strength: 97, passRush: 90, tackling: 86, playRecognition: 88, acceleration: 80, speed: 66, agility: 66, stamina: 78, awareness: 84 },
    agingOffset: 0.5,
    injuryRisk: 1.0,
    share: 0.08,
  },
  LB: {
    weights: { tackling: 4, playRecognition: 4, zoneCoverage: 2, speed: 2, awareness: 2, blockShedding: 1.5, strength: 1 },
    eliteShape: { tackling: 95, playRecognition: 95, zoneCoverage: 86, speed: 88, awareness: 92, blockShedding: 85, strength: 80, acceleration: 88, agility: 84, manCoverage: 72, passRush: 75, stamina: 88 },
    agingOffset: 0,
    injuryRisk: 1.1,
    share: 0.1,
  },
  CB: {
    weights: { manCoverage: 4.5, zoneCoverage: 3, speed: 3.5, acceleration: 2, agility: 2, playRecognition: 2, jumping: 1, catching: 0.5 },
    eliteShape: { manCoverage: 96, zoneCoverage: 92, speed: 96, acceleration: 95, agility: 95, playRecognition: 90, jumping: 90, catching: 75, tackling: 70, strength: 60, stamina: 88, awareness: 86 },
    agingOffset: -0.5,
    injuryRisk: 1.0,
    share: 0.1,
  },
  S: {
    weights: { zoneCoverage: 4, playRecognition: 3.5, tackling: 2.5, speed: 2.5, manCoverage: 1.5, awareness: 1.5 },
    eliteShape: { zoneCoverage: 95, playRecognition: 95, tackling: 88, speed: 92, manCoverage: 85, awareness: 92, acceleration: 92, agility: 88, jumping: 86, catching: 72, strength: 70, stamina: 88 },
    agingOffset: 0.5,
    injuryRisk: 1.0,
    share: 0.08,
  },
  K: {
    weights: { kickAccuracy: 5, kickPower: 4, poise: 1 },
    eliteShape: { kickAccuracy: 97, kickPower: 96, poise: 90, awareness: 60, speed: 55, strength: 45 },
    agingOffset: 6,
    injuryRisk: 0.25,
    share: 0.02,
  },
  P: {
    weights: { kickPower: 5, kickAccuracy: 3, poise: 1 },
    eliteShape: { kickPower: 97, kickAccuracy: 93, poise: 88, awareness: 60, speed: 55, strength: 45 },
    agingOffset: 6,
    injuryRisk: 0.25,
    share: 0.02,
  },
};

// Normalised [attribute, weight] pairs per position, precomputed because
// overall ratings are evaluated millions of times in a cohort simulation.
const normalisedWeights = {} as Record<Position, [AttributeKey, number][]>;
for (const pos of POSITIONS) {
  const entries = Object.entries(POSITION_PROFILES[pos].weights) as [AttributeKey, number][];
  const total = entries.reduce((a, [, w]) => a + w, 0);
  normalisedWeights[pos] = entries.map(([k, w]) => [k, w / total]);
}

/** Position overall: weighted mean of the attributes that matter at the position. */
export function overallFor(position: Position, attrs: Attributes): number {
  let sum = 0;
  for (const [key, w] of normalisedWeights[position]) sum += attrs[key] * w;
  return sum;
}

export function eliteShapeValue(position: Position, key: AttributeKey): number {
  return POSITION_PROFILES[position].eliteShape[key] ?? DEFAULT_SHAPE;
}

/** Overall an "elite shape" player would have; used to scale talent into ceilings. */
export function eliteShapeOverall(position: Position): number {
  const shape = {} as Attributes;
  for (const k of ATTRIBUTE_KEYS) shape[k] = eliteShapeValue(position, k);
  return overallFor(position, shape);
}
