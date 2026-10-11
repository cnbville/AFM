import { describe, expect, it } from 'vitest';
import { buildLevelScale, levelRating } from '../src/levelRating.js';
import { Rng } from '../src/rng.js';

describe('level rating', () => {
  const rng = new Rng('level-rating');
  const pool = Array.from({ length: 11_000 }, () => rng.normal(56, 9));
  const sorted = [...pool].sort((a, b) => b - a);
  const scale = buildLevelScale(pool, 'College');
  const rate = (x: number) => levelRating(x, 'FBS', scale);

  it('rises with overall and stays on the 1-99 scale', () => {
    let last = 0;
    for (let x = 0; x <= 100; x += 0.5) {
      const r = rate(x);
      expect(r).toBeGreaterThanOrEqual(last);
      expect(r).toBeGreaterThanOrEqual(1);
      expect(r).toBeLessThanOrEqual(99);
      last = r;
    }
  });

  it('puts about a hundred players at 90+ and saves 99 for a clear No. 1', () => {
    const ninetyPlus = pool.filter((x) => rate(x) >= 90).length;
    expect(ninetyPlus).toBeGreaterThan(80);
    expect(ninetyPlus).toBeLessThan(130);
    expect(rate(sorted[1]!)).toBeLessThan(98);
    expect(rate(sorted[1]! + 1)).toBeLessThan(99);
    expect(rate(sorted[1]! + 6)).toBe(99);
  });

  it('leaves the pro game on the absolute scale', () => {
    expect(levelRating(73.4, 'NFL', scale)).toBe(73);
  });
});
