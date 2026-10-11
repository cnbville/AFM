import { draftRound, type CohortResult, type CareerRecord } from '../src/career.js';
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

const LEVEL_RANK = { SmallCollege: 1, FCS: 2, FBS: 3 } as const;
const movedUp = (t: CareerRecord['transfers'][number]) => LEVEL_RANK[t.to] > LEVEL_RANK[t.from];

const round = (x: number, d = 3) => Math.round(x * 10 ** d) / 10 ** d;

export const isDrafted = (r: CareerRecord) => r.draftPick !== null;
/** Drafted or signed as an undrafted free agent. Every signing plays a season until camp cuts exist. */
export const signedPro = (r: CareerRecord) => r.nflSeasons > 0;
const isSpecialist = (r: CareerRecord) => r.player.position === 'K' || r.player.position === 'P';

/** NFL peak: the season he played at his best rating. */
export function nflPeakAge(r: CareerRecord): number | null {
  let best = -1;
  let age: number | null = null;
  for (const s of r.player.history) {
    if (s.level === 'NFL' && s.overallStart > best) {
      best = s.overallStart;
      age = s.age;
    }
  }
  return age;
}

/** Players suspended twice at the same school, a second incident that should have ended his time there. */
function repeatSuspensionsAtOneSchool(r: CareerRecord): boolean {
  let lastLevel: string | null = null;
  let suspensions = 0;
  for (const s of r.player.history) {
    if (s.level === 'HS' || s.level === 'NFL') continue;
    if (s.level !== lastLevel) suspensions = 0;
    lastLevel = s.level;
    if (s.events.includes('suspended') && ++suspensions >= 2) return true;
  }
  return false;
}

export function summarize({ records, draftSlots }: CohortResult) {
  const n = records.length;
  const drafted = records.filter(isDrafted);
  const pros = records.filter(signedPro);
  const byStars = (s: number) => records.filter((r) => r.stars === s);
  const draftRate = (rs: CareerRecord[]) => (rs.length ? rs.filter(isDrafted).length / rs.length : NaN);
  const nflSeasons = records.flatMap((r) => r.player.history.filter((s) => s.level === 'NFL'));
  const freshman = records.map((r) => r.initialOverall);
  const senior = records.map((r) => r.player.history[3]!.overall);
  const peakAgesBy = (pos: Position[]) =>
    pros.filter((r) => pos.includes(r.player.position) && r.nflSeasons >= 4).map((r) => nflPeakAge(r)!);
  const firstRound = drafted.filter((r) => draftRound(r.draftPick!, draftSlots) === 1);
  const lateRound = drafted.filter((r) => draftRound(r.draftPick!, draftSlots) === 7);
  const youngSeasons = records.flatMap((r) =>
    r.player.history.filter((s, i) => s.age >= 18 && s.age <= 23 && i > 0),
  );
  const regressions = records.flatMap((r) =>
    r.player.history.filter((s, i) => i > 0 && s.age >= 18 && s.age <= 23 && s.overall < r.player.history[i - 1]!.overall),
  );
  // How close pros with 4+ seasons came to their potential: at their best
  // season against their potential that season, and against their potential
  // on draft day (ceilings keep rising in the pros as veterans learn).
  const longPros = pros.filter((r) => r.nflSeasons >= 4);
  const reach = longPros.map((r) => {
    const peak = r.player.history.reduce((b, s) => (s.overallStart > b.overallStart ? s : b));
    const potentialThen = r.player.history[r.player.history.indexOf(peak) - 1]?.potential ?? r.player.originalPotential;
    return { gap: potentialThen - peak.overallStart, ratio: peak.overallStart / potentialThen };
  });
  const reachDraftDay = longPros.map((r) => ({ gap: r.potentialAtExit - r.peakOverall, ratio: r.peakOverall / r.potentialAtExit }));
  const topSeniors = records.filter((r) => r.player.history[3]!.levelRating! >= 85);
  const exitOverall = records
    .filter((r) => r.college)
    .map((r) => r.player.history.filter((s) => s.level !== 'NFL').at(-1)!.overall);
  /** Rating on draft day: the end of his last year before the pros. */
  const draftOverall = drafted.map((r) => r.player.history.filter((s) => s.level !== 'NFL').at(-1)!.overall);
  // Consensus board: every prospect ranked by the grade teams saw.
  const consensusRank = new Map<CareerRecord, number>();
  records
    .filter((r) => r.draftGrade !== null)
    .sort((a, b) => b.draftGrade! - a.draftGrade!)
    .forEach((r, i) => consensusRank.set(r, i + 1));
  const veteranRows = records.flatMap((r) => r.player.history.filter((s) => s.level === 'NFL' && s.age >= 33 && s.age <= 35));
  const specialistNflRows = records.filter(isSpecialist).flatMap((r) => r.player.history.filter((s) => s.level === 'NFL'));
  const seniorL = (r: CareerRecord) => r.player.history[3]!.levelRating!;
  return {
    players: n,
    draftSlots,
    rates: {
      college: round(records.filter((r) => r.college).length / n, 4),
      signedFBS: round(records.filter((r) => r.signedWith === 'FBS').length / n, 4),
      finishedAtFBS: round(records.filter((r) => r.college === 'FBS').length / n, 4),
      drafted: round(drafted.length / n, 5),
      signedPro: round(pros.length / n, 5),
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
      /** Draftees' rating on draft day. */
      rookieOverall: {
        p10: round(quantile(draftOverall, 0.1), 1),
        median: round(median(draftOverall), 1),
        p90: round(quantile(draftOverall, 0.9), 1),
      },
      age: round(mean(drafted.map((r) => r.player.history.find((s) => s.level === 'NFL')!.age)), 1),
      /** First-round picks from outside the consensus top 50: reaches. */
      firstRoundersOutsideTop50: drafted.filter((r) => draftRound(r.draftPick!, draftSlots) === 1 && consensusRank.get(r)! > 50).length,
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
        /** Drafted players and undrafted signings. */
        allSigned: round(mean(pros.map((r) => r.nflSeasons)), 2),
        firstRound: round(mean(firstRound.map((r) => r.nflSeasons)), 2),
        round7: round(mean(lateRound.map((r) => r.nflSeasons)), 2),
        udfa: round(mean(pros.filter((r) => r.udfa).map((r) => r.nflSeasons)), 2),
      },
      medianPeakAge: {
        allButKP: median(peakAgesBy(['QB', 'RB', 'WR', 'TE', 'OT', 'IOL', 'EDGE', 'DT', 'LB', 'CB', 'S'])),
        RB: median(peakAgesBy(['RB'])),
        QB: median(peakAgesBy(['QB'])),
        KP: median(peakAgesBy(['K', 'P'])),
      },
      /** Upside veterans of 33-35 still show above their rating. */
      veteranPotentialGap: round(median(veteranRows.map((s) => s.potential - s.overall)), 2),
      specialists: {
        signed: records.filter((r) => isSpecialist(r) && r.nflSeasons > 0).length,
        seasonsAt38Plus: round(specialistNflRows.filter((s) => s.age >= 38).length / specialistNflRows.length),
        oldestSeason: Math.max(...specialistNflRows.map((s) => s.age)),
      },
    },
    smallSchools: {
      d2Drafted: drafted.filter((r) => r.college === 'SmallCollege').length,
      /** Earliest round a D2 player was drafted in (null if none was). */
      d2EarliestRound: drafted.some((r) => r.college === 'SmallCollege')
        ? Math.min(...drafted.filter((r) => r.college === 'SmallCollege').map((r) => draftRound(r.draftPick!, draftSlots)))
        : null,
      d2Udfa: records.filter((r) => r.udfa && r.college === 'SmallCollege').length,
      fcsDrafted: drafted.filter((r) => r.college === 'FCS').length,
      playersMovedUp: records.filter((r) => r.transfers.some(movedUp)).length,
      draftedAfterMovingUp: drafted.filter((r) => r.transfers.some(movedUp)).length,
      /** Every move, by route. Moves down follow a dismissal. */
      transferRoutes: records.flatMap((r) => r.transfers.map((t) => `${t.from}>${t.to}`)).reduce<Record<string, number>>((m, k) => ((m[k] = (m[k] ?? 0) + 1), m), {}),
      draftedSignedFCS: drafted.filter((r) => r.signedWith === 'FCS').length,
      draftedSignedD2: drafted.filter((r) => r.signedWith === 'SmallCollege').length,
      draftedAfterMovingUpFromD2: drafted.filter((r) => r.signedWith === 'SmallCollege' && r.transfers.some(movedUp)).length,
      draftedAfterMovingUpRounds1to3: drafted.filter((r) => r.transfers.some(movedUp) && draftRound(r.draftPick!, draftSlots) <= 3).length,
    },
    recruiting: {
      fiveStarSeniorMin: Math.min(...byStars(5).map((r) => r.player.history[3]!.levelRating!)),
      topSeniors: topSeniors.length,
      topSeniorsUnranked: topSeniors.filter((r) => r.stars === 0).length,
      topSeniorsAtD2OrNone: topSeniors.filter((r) => r.signedWith === 'SmallCollege' || !r.signedWith).length,
      /** QBs' share of the top 100-odd seniors, against their share of all players (1 = fair). */
      qbTopSeniorRatio: round(
        records.filter((r) => r.player.history[3]!.levelRating! >= 90 && r.player.position === 'QB').length /
          records.filter((r) => r.player.history[3]!.levelRating! >= 90).length /
          (records.filter((r) => r.player.position === 'QB').length / n),
        2,
      ),
      blueChipQBs: records.filter((r) => r.stars >= 4 && r.player.position === 'QB').length,
      fiveStarPositions: new Set(byStars(5).map((r) => r.player.position)).size,
      /** National top-100-odd seniors (level 90+, kickers and punters aside) with fewer than four stars. */
      top100SeniorsBelowFourStars: records.filter((r) => !isSpecialist(r) && seniorL(r) >= 90 && r.stars < 4).length,
      blueChipSpecialists: records.filter((r) => isSpecialist(r) && r.stars >= 4).length,
      unrankedSignedFbsOrFcs: records.filter((r) => r.stars === 0 && (r.signedWith === 'FBS' || r.signedWith === 'FCS')).length,
      /** Seniors around the national top 2,000 who got no college offer at all. */
      level80SeniorsWithNoOffer: records.filter((r) => seniorL(r) >= 80 && !r.signedWith).length,
    },
    trouble: {
      perCollegeSeason: round(
        records.flatMap((r) => r.player.history.filter((s) => s.events.includes('suspended') || s.events.includes('dismissed'))).length /
          records.flatMap((r) => r.player.history.filter((s) => s.level === 'FBS' || s.level === 'FCS' || s.level === 'SmallCollege')).length,
      ),
      suspensions: records.flatMap((r) => r.player.history.filter((s) => s.events.includes('suspended'))).length,
      dismissals: records.flatMap((r) => r.player.history.filter((s) => s.events.includes('dismissed'))).length,
      leftFootball: records.filter((r) => r.leftFootball).length,
      fiveStarsInTrouble: records.filter((r) => r.stars === 5 && r.player.history.some((s) => s.events.includes('suspended') || s.events.includes('dismissed'))).length,
      suspendedTwiceAtOneSchool: records.filter(repeatSuspensionsAtOneSchool).length,
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
    /** Pros with 4+ seasons: his best season against his potential going into it. */
    reachingPotential: {
      within2: round(reach.filter((x) => x.gap <= 2).length / reach.length),
      /** 90% of it or more, but more than 2 points short. */
      reached90pctPlus: round(reach.filter((x) => x.gap > 2 && x.ratio >= 0.9).length / reach.length),
      below90pct: round(reach.filter((x) => x.ratio < 0.9).length / reach.length),
    },
    /** The same against his potential on draft day. */
    reachingDraftDayPotential: {
      within2: round(reachDraftDay.filter((x) => x.gap <= 2).length / reachDraftDay.length),
      reached90pctPlus: round(reachDraftDay.filter((x) => x.gap > 2 && x.ratio >= 0.9).length / reachDraftDay.length),
      below90pct: round(reachDraftDay.filter((x) => x.ratio < 0.9).length / reachDraftDay.length),
    },
    potential: {
      initialVsExitCorrelationDrafted: round(correlation(drafted.map((r) => r.initialPotential), drafted.map((r) => r.potentialAtExit))),
      initialVsExitCorrelationAll: round(correlation(records.map((r) => r.initialPotential), records.map((r) => r.potentialAtExit))),
      draftedRoseBy8PlusSince14: round(drafted.filter((r) => r.potentialAtExit - r.initialPotential >= 8).length / drafted.length),
      collegiansFellBy8PlusSince14: round(
        records.filter((r) => r.college && r.potentialAtExit - r.initialPotential <= -8).length /
          records.filter((r) => r.college).length,
      ),
      youngRegressionShare: round(regressions.length / youngSeasons.length),
      /** Seasons tagged as a breakout although the rating fell. */
      fallingBreakouts: records.flatMap((r) => r.player.history.filter((s) => s.events.includes('breakout') && s.overall < s.overallStart)).length,
    },
  };
}
