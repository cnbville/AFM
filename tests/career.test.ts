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
  });

  it('rarely drafts D2 players, and never before day 3', () => {
    const sm = s.smallSchools;
    expect(s.draftedShareByStars.fromD2).toBeLessThan(0.04);
    if (sm.d2Drafted > 0) expect(sm.d2EarliestRound).toBeGreaterThanOrEqual(4);
    expect(sm.d2Udfa).toBeGreaterThan(sm.d2Drafted);
    expect(s.draftedShareByStars.fromFCS).toBeGreaterThan(0.01);
    expect(s.draftedShareByStars.fromFCS).toBeLessThan(0.12);
    // Standouts move up through the transfer portal instead.
    expect(sm.transfersUp).toBeGreaterThan(100);
    expect(sm.draftedAfterTransfer).toBeGreaterThan(0);
  });

  it('makes a 99 level rating as rare as EA College Football does', () => {
    const l = s.levelRatings;
    // EA rated nobody 99 at launch in College Football 25 or 26 and one player in 27.
    expect(l.hsSeniors99).toBeLessThanOrEqual(2);
    expect(l.collegeSeasons99).toBeLessThanOrEqual(2);
    // About ten players a season at 95+, and about a hundred at 90+.
    expect(l.hsSeniors95plus).toBeGreaterThan(3);
    expect(l.hsSeniors95plus).toBeLessThan(25);
    expect(l.hsSeniors90plus).toBeGreaterThan(50);
    expect(l.hsSeniors90plus).toBeLessThan(200);
    expect(l.collegeSeasons95plus).toBeGreaterThan(5);
    expect(l.collegeSeasons95plus).toBeLessThan(35);
    expect(l.collegeSeasons90plus).toBeLessThan(300);
    // A typical FBS player rates in the low 70s, as on an EA roster.
    expect(l.fbsSeasonMedian).toBeGreaterThan(65);
    expect(l.fbsSeasonMedian).toBeLessThan(78);
    expect(l.fiveStarSeniorMedian).toBeGreaterThan(85);
    expect(l.fiveStarSeniorMedian).toBeGreaterThan(l.fourStarSeniorMedian);
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
    expect(o.share90plus).toBeLessThan(0.08);
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
