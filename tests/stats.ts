import type { CohortResult, CareerRecord } from '../src/career.js';
import type { Position } from '../src/positions.js';

export const mean = (xs: number[]) => (xs.length ? xs.reduce((a, b) => a + b, 0) / xs.length : NaN);
export const median = (xs: number[]) => quantile(xs, 0.5);
export function quantile(xs: number[], q: number): number {
  if (!xs.length) return NaN;
  const s = [...xs].sort((a, b) => a - b);
  const i = (s.length - 1) * q;
  const lo = Math.floor(i);
  const hi = Math.ceil(i);
  return s[lo]! + (s[hi]! - s[lo]!) * (i - lo);
}
export function correlation(xs: number[], ys: number[]): number {
  const mx = mean(xs);
  const my = mean(ys);
  let sxy = 0;
  let sxx = 0;
  let syy = 0;
  for (let i = 0; i < xs.length; i++) {
    sxy += (xs[i]! - mx) * (ys[i]! - my);
    sxx += (xs[i]! - mx) ** 2;
    syy += (ys[i]! - my) ** 2;
  }
  return sxy / Math.sqrt(sxx * syy);
}

const round = (x: number, d = 3) => Math.round(x * 10 ** d) / 10 ** d;

export const isDrafted = (r: CareerRecord) => r.draftPick !== null;
export const isPro = (r: CareerRecord) => r.nflSeasons > 0;

/** NFL peak: the season with the best overall among pro seasons. */
export function nflPeakAge(r: CareerRecord): number | null {
  let best = -1;
  let age: number | null = null;
  for (const s of r.player.history) {
    if (s.level === 'NFL' && s.overall > best) {
      best = s.overall;
      age = s.age;
    }
  }
  return age;
}

export function summarize({ records, draftSlots }: CohortResult) {
  const n = records.length;
  const drafted = records.filter(isDrafted);
  const pros = records.filter(isPro);
  const byStars = (s: number) => records.filter((r) => r.stars === s);
  const draftRate = (rs: CareerRecord[]) => (rs.length ? rs.filter(isDrafted).length / rs.length : NaN);
  const nflSeasons = records.flatMap((r) => r.player.history.filter((s) => s.level === 'NFL'));
  const freshman = records.map((r) => r.initialOverall);
  const senior = records.map((r) => r.player.history[3]!.overall);
  const peakAgesBy = (pos: Position[]) =>
    pros.filter((r) => pos.includes(r.player.position) && r.nflSeasons >= 4).map((r) => nflPeakAge(r)!);
  const firstRound = drafted.filter((r) => r.draftPick! <= draftSlots / 7);
  const lateRound = drafted.filter((r) => r.draftPick! > (draftSlots * 6) / 7);
  const youngSeasons = records.flatMap((r) =>
    r.player.history.filter((s, i) => s.age >= 18 && s.age <= 23 && i > 0),
  );
  const regressions = records.flatMap((r) =>
    r.player.history.filter((s, i) => i > 0 && s.age >= 18 && s.age <= 23 && s.overall < r.player.history[i - 1]!.overall),
  );
  const longPros = pros.filter((r) => r.nflSeasons >= 4);
  const reach = longPros.map((r) => {
    const peak = r.player.history.reduce((b, s) => (s.overall > b.overall ? s : b));
    return { gap: peak.potential - peak.overall, ratio: peak.overall / peak.potential };
  });
  const exitOverall = records
    .filter((r) => r.college)
    .map((r) => r.player.history.filter((s) => s.level !== 'NFL').at(-1)!.overall);
  const draftOverall = drafted.map((r) => r.player.history.filter((s) => s.level !== 'NFL').at(-1)!.overall);
  return {
    players: n,
    draftSlots,
    rates: {
      college: round(records.filter((r) => r.college).length / n, 4),
      fbs: round(records.filter((r) => r.college === 'FBS').length / n, 4),
      drafted: round(drafted.length / n, 5),
      pro: round(pros.length / n, 5),
    },
    draftRateByStars: {
      5: round(draftRate(byStars(5))),
      4: round(draftRate(byStars(4))),
      3: round(draftRate(byStars(3))),
      2: round(draftRate(byStars(2))),
      0: round(draftRate(byStars(0)), 5),
    },
    draftedShareByStars: {
      5: round(drafted.filter((r) => r.stars === 5).length / drafted.length),
      4: round(drafted.filter((r) => r.stars === 4).length / drafted.length),
      3: round(drafted.filter((r) => r.stars === 3).length / drafted.length),
      lowOrUnrated: round(drafted.filter((r) => r.stars <= 2).length / drafted.length),
      fromSmallSchools: round(drafted.filter((r) => r.college !== 'FBS').length / drafted.length),
      fromFCS: round(drafted.filter((r) => r.college === 'FCS').length / drafted.length),
      fromD2: round(drafted.filter((r) => r.college === 'SmallCollege').length / drafted.length),
    },
    hs: {
      freshmanOverall: { mean: round(mean(freshman), 1), p99: round(quantile(freshman, 0.99), 1) },
      seniorOverall: { mean: round(mean(senior), 1), p99: round(quantile(senior, 0.99), 1) },
      fiveStarSeniorOverall: round(mean(byStars(5).map((r) => r.player.history[3]!.overall)), 1),
    },
    draft: {
      rookieOverall: {
        p10: round(quantile(drafted.map((r) => r.player.history.find((s) => s.level === 'NFL')!.overall), 0.1), 1),
        median: round(median(drafted.map((r) => r.player.history.find((s) => s.level === 'NFL')!.overall)), 1),
        p90: round(quantile(drafted.map((r) => r.player.history.find((s) => s.level === 'NFL')!.overall), 0.9), 1),
      },
      age: round(mean(drafted.map((r) => r.player.history.find((s) => s.level === 'NFL')!.age)), 1),
    },
    nfl: {
      seasonOverall: {
        mean: round(mean(nflSeasons.map((s) => s.overall)), 1),
        p10: round(quantile(nflSeasons.map((s) => s.overall), 0.1), 1),
        p90: round(quantile(nflSeasons.map((s) => s.overall), 0.9), 1),
        share90plus: round(nflSeasons.filter((s) => s.overall >= 90).length / nflSeasons.length),
        share80plus: round(nflSeasons.filter((s) => s.overall >= 80).length / nflSeasons.length),
      },
      careerLength: {
        drafted: round(mean(drafted.map((r) => r.nflSeasons)), 2),
        allPros: round(mean(pros.map((r) => r.nflSeasons)), 2),
        firstRound: round(mean(firstRound.map((r) => r.nflSeasons)), 2),
        lateRound: round(mean(lateRound.map((r) => r.nflSeasons)), 2),
        udfa: round(mean(pros.filter((r) => r.udfa).map((r) => r.nflSeasons)), 2),
      },
      medianPeakAge: {
        all: median(peakAgesBy(['QB', 'RB', 'WR', 'TE', 'OT', 'IOL', 'EDGE', 'DT', 'LB', 'CB', 'S'])),
        RB: median(peakAgesBy(['RB'])),
        QB: median(peakAgesBy(['QB'])),
        KP: median(peakAgesBy(['K', 'P'])),
      },
    },
    smallSchools: {
      d2Drafted: drafted.filter((r) => r.college === 'SmallCollege').length,
      d2EarliestRound: Math.min(...drafted.filter((r) => r.college === 'SmallCollege').map((r) => Math.ceil((r.draftPick! / draftSlots) * 7))),
      d2Udfa: records.filter((r) => r.udfa && r.college === 'SmallCollege').length,
      fcsDrafted: drafted.filter((r) => r.college === 'FCS').length,
      transfersUp: records.filter((r) => r.transfers.length > 0).length,
      draftedAfterTransfer: drafted.filter((r) => r.transfers.length > 0).length,
      transferRoutes: records.flatMap((r) => r.transfers.map((t) => `${t.from}>${t.to}`)).reduce<Record<string, number>>((m, k) => ((m[k] = (m[k] ?? 0) + 1), m), {}),
      draftedSignedFCS: drafted.filter((r) => r.signedWith === 'FCS').length,
      draftedSignedD2: drafted.filter((r) => r.signedWith === 'SmallCollege').length,
      draftedAfterTransferFromD2: drafted.filter((r) => r.signedWith === 'SmallCollege' && r.transfers.length > 0).length,
      draftedAfterTransferDay2: drafted.filter((r) => r.transfers.length > 0 && r.draftPick! <= Math.round(draftSlots * 3 / 7)).length,
    },
    levelRatings: {
      hsSeniors99: records.filter((r) => r.player.history[3]!.levelRating! >= 99).length,
      hsSeniors95plus: records.filter((r) => r.player.history[3]!.levelRating! >= 95).length,
      hsSeniors90plus: records.filter((r) => r.player.history[3]!.levelRating! >= 90).length,
      fiveStarSeniorMedian: median(records.filter((r) => r.stars === 5).map((r) => r.player.history[3]!.levelRating!)),
      fourStarSeniorMedian: median(records.filter((r) => r.stars === 4).map((r) => r.player.history[3]!.levelRating!)),
      collegeSeasons99: records.flatMap((r) => r.player.history.filter((s) => s.level !== 'HS' && s.level !== 'NFL' && s.levelRating! >= 99)).length,
      hsSeasons99AnyAge: records.flatMap((r) => r.player.history.filter((s) => s.level === 'HS' && s.levelRating! >= 99)).length,
      collegeSeasons95plus: records.flatMap((r) => r.player.history.filter((s) => s.level !== 'HS' && s.level !== 'NFL' && s.levelRating! >= 95)).length,
      fbsSeasonMedian: median(records.flatMap((r) => r.player.history.filter((s) => s.level === 'FBS').map((s) => s.levelRating!))),
      bestD2Season: Math.max(...records.flatMap((r) => r.player.history.filter((s) => s.level === 'SmallCollege').map((s) => s.levelRating!))),
      collegeSeasons90plus: records.flatMap((r) => r.player.history.filter((s) => s.level !== 'HS' && s.level !== 'NFL' && s.levelRating! >= 90)).length,
      draftedFinalCollegeMedian: median(drafted.map((r) => r.player.history.filter((s) => s.level !== 'NFL').at(-1)!.levelRating!)),
    },
    levels: {
      collegeExitsRated65Plus: exitOverall.filter((o) => o >= 65).length,
      collegeExitsRated70Plus: exitOverall.filter((o) => o >= 70).length,
      draftedBelow65: round(draftOverall.filter((o) => o < 65).length / drafted.length),
      peakCounts: {
        '97+': records.filter((r) => r.peakOverall >= 97).length,
        '92+': records.filter((r) => r.peakOverall >= 92).length,
        '85+': records.filter((r) => r.peakOverall >= 85).length,
      },
    },
    reachingPotential: {
      within2: round(reach.filter((x) => x.gap <= 2).length / reach.length),
      reached90to98pct: round(reach.filter((x) => x.gap > 2 && x.ratio >= 0.9).length / reach.length),
      below90pct: round(reach.filter((x) => x.ratio < 0.9).length / reach.length),
    },
    potential: {
      initialVsExitCorrelationDrafted: round(correlation(drafted.map((r) => r.initialPotential), drafted.map((r) => r.potentialAtExit))),
      initialVsExitCorrelationAll: round(correlation(records.map((r) => r.initialPotential), records.map((r) => r.potentialAtExit))),
      draftedRoseBy8Plus: round(drafted.filter((r) => r.potentialAtExit - r.initialPotential >= 8).length / drafted.length),
      collegeFellBy8Plus: round(
        records.filter((r) => r.college && r.potentialAtExit - r.initialPotential <= -8).length /
          records.filter((r) => r.college).length,
      ),
      youngRegressionShare: round(regressions.length / youngSeasons.length),
    },
  };
}
