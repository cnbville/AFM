import type { Level } from './player.js';

/**
 * A level rating says how good a player is against everyone he could line up
 * with or against, on a 1-99 scale. The pro game uses the absolute rating
 * itself; high school and college get this translation so that a 60-rated
 * junior reads as the force he is against high school opponents.
 *
 * - High school: compared with every high schooler of the same age, all
 *   positions together, like a national recruiting ranking.
 * - College: compared with every FBS player, all ages and positions together,
 *   like EA Sports College Football. FCS and small-college players are put on
 *   the same FBS scale.
 *
 * The scale is anchored to national ranks, using how rare top ratings are in
 * EA College Football (about 11,000 players a season): 90+ is roughly the top
 * 100 (an All-American; for high schoolers, an All-American Bowl player), 95+
 * the top 8, and only a player about 5 points clear of the No. 2 reaches 99. EA rated
 * nobody 99 at launch in College Football 25 or 26, and one player in 27.
 */
export interface LevelScale {
  /** (overall, level rating) points, ascending in overall. */
  points: [number, number][];
}

/** Rating points per overall point above the No. 2 player in the pool. */
const ABOVE_TOP_SLOPE = 0.4;

/** Players in one real-world pool: a season of FBS rosters, or a national high school age group. */
export const LEVEL_REFERENCE_POOL = { College: 11_000, HS: 250_000 } as const;

type Pool = keyof typeof LEVEL_REFERENCE_POOL;

/** Anchors: [quantile from the bottom] or [rank from the top in a reference-sized pool], and the rating there. */
const ANCHORS: Record<Pool, { quantile: [number, number][]; rank: [number, number][] }> = {
  College: {
    quantile: [[0.01, 55], [0.5, 72]],
    rank: [[700, 85], [100, 90], [8, 95], [2, 96.5]],
  },
  HS: {
    quantile: [[0.01, 5], [0.5, 50]],
    rank: [[2000, 80], [100, 90], [8, 95], [2, 96.5]],
  },
};

export function levelGroupKey(level: Level, age: number): string | null {
  if (level === 'NFL') return null;
  return level === 'HS' ? `HS:${age}` : 'College';
}

/** Whether a season sets the scale for its group (only FBS seasons set the college scale). */
export function setsLevelScale(level: Level): boolean {
  return level === 'HS' || level === 'FBS';
}

/**
 * Build the scale for one group from the overalls that set it. Ranks are
 * scaled by how big this pool is compared with the real-world one, so a
 * smaller simulated class still puts the same share of players at 90+.
 */
export function buildLevelScale(overalls: number[], pool: Pool): LevelScale {
  const s = [...overalls].sort((a, b) => b - a);
  const n = s.length;
  const scale = n / LEVEL_REFERENCE_POOL[pool];
  const anchors = ANCHORS[pool];
  const points: [number, number][] = [];
  for (const [q, rating] of anchors.quantile) points.push([s[Math.min(n - 1, Math.floor(n * (1 - q)))]!, rating]);
  for (const [rank, rating] of anchors.rank) points.push([s[Math.min(n - 1, Math.max(1, Math.round(rank * scale)) - 1)]!, rating]);
  points.sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  // Keep the curve strictly increasing in overall: in a tiny pool two anchors can share a value.
  const clean: [number, number][] = [];
  for (const p of points) {
    const last = clean[clean.length - 1];
    if (last && p[0] <= last[0]) last[1] = Math.max(last[1], p[1]);
    else clean.push([p[0], p[1]]);
  }
  return { points: clean };
}

export function levelRating(overall: number, level: Level, scale: LevelScale | undefined): number {
  if (level === 'NFL' || !scale || scale.points.length === 0) return clampRating(overall);
  const pts = scale.points;
  const top = pts[pts.length - 1]!;
  if (overall >= top[0]) return clampRating(top[1] + ABOVE_TOP_SLOPE * (overall - top[0]));
  const bottom = pts[0]!;
  if (overall <= bottom[0]) {
    const next = pts[1];
    const slope = next ? (next[1] - bottom[1]) / (next[0] - bottom[0]) : 1;
    return clampRating(bottom[1] - slope * (bottom[0] - overall));
  }
  for (let i = 1; i < pts.length; i++) {
    const [x1, y1] = pts[i]!;
    if (overall <= x1) {
      const [x0, y0] = pts[i - 1]!;
      return clampRating(y0 + ((overall - x0) / (x1 - x0)) * (y1 - y0));
    }
  }
  return clampRating(top[1]);
}

function clampRating(x: number): number {
  return Math.max(1, Math.min(99, Math.round(x)));
}
