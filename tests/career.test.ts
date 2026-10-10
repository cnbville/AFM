import { beforeAll, describe, expect, it } from 'vitest';
import { simulateCohort, type CohortResult } from '../src/career.js';
import { summarize } from './stats.js';

/**
 * Simulates one full US high school class (250,000 freshmen) from age 14 to
 * the end of every pro career, and checks the outcomes against real-world
 * rates. Bands are wide enough to be seed-stable but tight enough to fail if
 * the model drifts into fantasy.
 */
describe('a national class, freshman year to retirement', () => {
  let result: CohortResult;
  let s: ReturnType<typeof summarize>;
  beforeAll(() => {
    result = simulateCohort({ size: 250_000, seed: 'national-class' });
    s = summarize(result);
  });

  it('develops high schoolers steadily', () => {
    expect(s.hs.freshmanOverall.mean).toBeGreaterThan(15);
    expect(s.hs.freshmanOverall.mean).toBeLessThan(32);
    expect(s.hs.seniorOverall.mean).toBeGreaterThan(s.hs.freshmanOverall.mean + 5);
    // Five-star seniors are far ahead of their class but not yet pro-ready.
    expect(s.hs.fiveStarSeniorOverall).toBeGreaterThan(s.hs.seniorOverall.p99);
    expect(s.hs.fiveStarSeniorOverall).toBeLessThan(s.draft.rookieOverall.median);
  });

  it('drafts recruits at realistic rates by star rating', () => {
    const r = s.draftRateByStars;
    expect(r[5]).toBeGreaterThan(0.4);
    expect(r[5]).toBeLessThan(0.85);
    expect(r[4]).toBeGreaterThan(0.12);
    expect(r[4]).toBeLessThan(0.4);
    expect(r[3]).toBeGreaterThan(0.02);
    expect(r[3]).toBeLessThan(0.1);
    expect(r[2]).toBeLessThan(r[3]);
    expect(r[0]).toBeLessThan(r[2]);
  });

  it('still finds a share of draftees among low-rated and small-school players', () => {
    const d = s.draftedShareByStars;
    expect(d[5]).toBeGreaterThan(0.03);
    expect(d[5]).toBeLessThan(0.15);
    expect(d[4]).toBeGreaterThan(0.2);
    expect(d[4]).toBeLessThan(0.45);
    expect(d.lowOrUnrated).toBeGreaterThan(0.1);
    expect(d.lowOrUnrated).toBeLessThan(0.4);
    expect(d.fromSmallSchools).toBeGreaterThan(0.05);
    expect(d.fromSmallSchools).toBeLessThan(0.3);
  });

  it('produces realistic pro careers', () => {
    const c = s.nfl.careerLength;
    expect(c.allPros).toBeGreaterThan(2.5);
    expect(c.allPros).toBeLessThan(5.5);
    expect(c.drafted).toBeGreaterThan(4);
    expect(c.drafted).toBeLessThan(8.5);
    expect(c.firstRound).toBeGreaterThan(c.lateRound + 3);
    expect(c.udfa).toBeLessThan(c.drafted);
  });

  it('peaks at realistic ages by position', () => {
    const p = s.nfl.medianPeakAge;
    expect(p.all).toBeGreaterThanOrEqual(25);
    expect(p.all).toBeLessThanOrEqual(28);
    expect(p.RB).toBeLessThanOrEqual(26);
    expect(p.QB).toBeGreaterThanOrEqual(28);
    expect(p.QB).toBeGreaterThan(p.RB);
  });

  it('keeps elite pros rare', () => {
    const o = s.nfl.seasonOverall;
    expect(o.mean).toBeGreaterThan(68);
    expect(o.mean).toBeLessThan(80);
    expect(o.share90plus).toBeGreaterThan(0.005);
    expect(o.share90plus).toBeLessThan(0.06);
    expect(s.draft.rookieOverall.median).toBeGreaterThan(62);
    expect(s.draft.rookieOverall.median).toBeLessThan(76);
  });

  it('treats potential as dynamic, not fixed at birth', () => {
    const p = s.potential;
    expect(p.initialVsExitCorrelationDrafted).toBeGreaterThan(0.3);
    expect(p.initialVsExitCorrelationDrafted).toBeLessThan(0.85);
    expect(p.draftedRoseBy8Plus).toBeGreaterThan(0.3);
    expect(p.collegeFellBy8Plus).toBeGreaterThan(0.005);
    expect(p.youngRegressionShare).toBeGreaterThan(0.003);
    expect(p.youngRegressionShare).toBeLessThan(0.15);
  });

  it('is reproducible from a seed', () => {
    const a = summarize(simulateCohort({ size: 3000, seed: 'repeat' }));
    const b = summarize(simulateCohort({ size: 3000, seed: 'repeat' }));
    expect(a).toEqual(b);
  });
});
