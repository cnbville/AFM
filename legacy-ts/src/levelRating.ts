import type { Level } from './player.js';
import type { Position } from './positions.js';

/**
 * A level rating says how good a player is against everyone he could line up
 * with or against, on a 1-99 scale. The pro game uses the absolute rating
 * itself; high school and college get this translation so that a 60-rated
 * junior reads as the force he is against high school opponents.
 *
 * - High school: compared with every high schooler of the same age, like a
 *   national recruiting ranking.
 * - College: compared with every FBS player of any age, like EA Sports College
 *   Football. FCS and small-college players are put on the same FBS scale
 *   after the same discount NFL scouts give their tape.
 *
 * Positions mature at different ages: at 17 the best quarterbacks and
 * linemen rate lower than the best running backs and corners, though they are
 * just as far ahead of their peers. So a player is first placed against his
 * own position (see positionAligner) and then ranked nationally, the way EA
 * Sports College Football's top 100 spans every position.
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

type Pool = 'College' | 'HS';

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
 * Build the scale for one group from the overalls that set it. The top ranks
 * are national counts (the top 100 is the top 100 however big the pool is),
 * so they are only scaled when the simulated class is smaller than a real one.
 */
export function buildLevelScale(overalls: number[], pool: Pool, classShare = 1): LevelScale {
  const s = [...overalls].sort((a, b) => b - a);
  const n = s.length;
  const anchors = ANCHORS[pool];
  const points: [number, number][] = [];
  for (const [q, rating] of anchors.quantile) points.push([s[Math.min(n - 1, Math.floor(n * (1 - q)))]!, rating]);
  for (const [rank, rating] of anchors.rank) points.push([s[Math.min(n - 1, Math.max(1, Math.round(rank * classShare)) - 1)]!, rating]);
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

/**
 * Maps an overall at one position onto the all-position scale of the same
 * group, so that the same standing among his position's peers gives the same
 * value: each position's median and 99th percentile line up with the pool's.
 * Positions with too few players to measure are left as they are.
 */
export function positionAligner(samples: Iterable<[Position, number]>): (position: Position, overall: number) => number {
  const all: number[] = [];
  const byPosition = new Map<Position, number[]>();
  for (const [position, x] of samples) {
    all.push(x);
    let g = byPosition.get(position);
    if (!g) byPosition.set(position, (g = []));
    g.push(x);
  }
  const [allMid, allHigh] = quantiles(all, [0.5, 0.99]);
  const maps = new Map<Position, [number, number]>();
  for (const [position, xs] of byPosition) {
    if (xs.length < 30) continue;
    const [mid, high] = quantiles(xs, [0.5, 0.99]);
    if (high - mid < 0.5) continue;
    maps.set(position, [mid, (allHigh - allMid) / (high - mid)]);
  }
  return (position, overall) => {
    const m = maps.get(position);
    return m ? allMid + (overall - m[0]) * m[1] : overall;
  };
}

function quantiles(xs: number[], qs: [number, number]): [number, number] {
  const s = [...xs].sort((a, b) => a - b);
  const at = (q: number) => s[Math.min(s.length - 1, Math.floor(q * (s.length - 1)))] ?? 0;
  return [at(qs[0]), at(qs[1])];
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
