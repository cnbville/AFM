import type { Level } from './player.js';
import type { Position } from './positions.js';

/**
 * A level rating says how good a player is compared with players at the same
 * position, level and age, on a 1-99 scale. The pro game uses the absolute
 * rating itself; high school and college get this translation so that a
 * 60-rated junior reads as the force he is against high school opponents.
 *
 * 99 is reserved for the best in the nation at that position and age, among
 * high schoolers or among all college players.
 * The scale is anchored to how close a player is to the No. 1, not to his
 * percentile, so being in the top 1% of 250,000 high schoolers is not enough.
 */
export interface LevelScale {
  /** Overall of the single best player in the group. */
  best: number;
  median: number;
  /** Overall at the 1st percentile. */
  floor: number;
}

/**
 * High school players are compared with high schoolers; every college player
 * is compared with all college players nationally, so a D2 star is measured
 * against the FBS stars he would face on draft day.
 */
const TOP_CURVE = 1.4;

export function levelGroupKey(position: Position, level: Level, age: number): string {
  if (level === 'NFL') return 'NFL';
  const pool = level === 'HS' ? 'HS' : 'College';
  return `${pool}:${position}:${age}`;
}

export function buildLevelScale(overalls: number[]): LevelScale {
  const s = [...overalls].sort((a, b) => a - b);
  const n = s.length;
  return {
    best: s[n - 1]!,
    median: s[Math.floor(n / 2)]!,
    floor: s[Math.floor(n * 0.01)]!,
  };
}

export function levelRating(overall: number, level: Level, scale: LevelScale | undefined): number {
  if (level === 'NFL' || !scale) return clampRating(overall);
  if (overall >= scale.median) {
    // Convex above the median: the last few points before the very best are
    // the hardest to earn, so 90+ means genuinely close to No. 1.
    const span = Math.max(1, scale.best - scale.median);
    const closeness = Math.min(1, (overall - scale.median) / span);
    return clampRating(50 + 49 * Math.pow(closeness, TOP_CURVE));
  }
  const span = Math.max(1, scale.median - scale.floor);
  return clampRating((50 * (overall - scale.floor)) / span);
}

function clampRating(x: number): number {
  return Math.max(1, Math.min(99, Math.round(x)));
}
