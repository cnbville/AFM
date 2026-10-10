import { developSeason } from './development.js';
import { generateProspect, overall, potential, type Level, type Player, type PlayingTime } from './player.js';
import { Rng, clamp } from './rng.js';
import { scoutPlayer } from './scouting.js';

/**
 * Simulates a whole national high school class from freshman year to the end
 * of every pro career. Roster slots at each level are a fixed share of the
 * class, matching real participation: about 1.3% sign with FBS schools, 1% with
 * FCS, 4.5% with smaller colleges, and roughly 257 per 250,000 are drafted.
 */
export const PIPELINE = {
  /** Real US freshman class size the shares below are expressed against. */
  referenceClassSize: 250_000,
  starShares: { 5: 0.00013, 4: 0.0014, 3: 0.008, 2: 0.016 } as Record<RecruitStars, number>,
  collegeShares: { FBS: 0.013, FCS: 0.01, SmallCollege: 0.045 },
  draftPicks: 257,
  /** UDFAs signed per drafted player. */
  udfaRatio: 1.6,
  /** Draft-score cutoff for an underclassman to leave school early. */
  earlyEntryScore: 70,
} as const;

export type RecruitStars = 0 | 2 | 3 | 4 | 5;
type CollegeLevel = 'FBS' | 'FCS' | 'SmallCollege';

export interface CareerRecord {
  player: Player;
  initialOverall: number;
  initialPotential: number;
  /** Recruiting stars at signing (0 = unranked). */
  stars: RecruitStars;
  college: CollegeLevel | null;
  /** True potential when leaving college (or high school). */
  potentialAtExit: number;
  draftPick: number | null;
  udfa: boolean;
  nflSeasons: number;
  peakOverall: number;
  peakAge: number;
}

export interface CohortResult {
  records: CareerRecord[];
  draftSlots: number;
}

export interface CohortOptions {
  size: number;
  seed: number | string;
}

export function simulateCohort({ size, seed }: CohortOptions): CohortResult {
  const rng = new Rng(seed);
  const share = size / PIPELINE.referenceClassSize;
  const records: CareerRecord[] = [];
  const hsCoaching: number[] = [];
  for (let i = 0; i < size; i++) {
    const player = generateProspect(rng, { id: `c${i}` });
    records.push({
      player,
      initialOverall: overall(player),
      initialPotential: potential(player),
      stars: 0,
      college: null,
      potentialAtExit: 0,
      draftPick: null,
      udfa: false,
      nflSeasons: 0,
      peakOverall: overall(player),
      peakAge: player.age,
    });
    hsCoaching.push(clamp(rng.normal(8, 3), 1, 20));
  }

  // --- High school: four seasons, ages 14-17 -------------------------------
  // Varsity starting spots get easier to earn as a class gets older.
  const hsStarterShare = [0.12, 0.3, 0.5, 0.6];
  for (let year = 0; year < 4; year++) {
    const roles = rolesByRank(records, hsStarterShare[year]!, 0.2);
    records.forEach((r, i) => {
      developSeason(r.player, { level: 'HS', coaching: hsCoaching[i]!, playingTime: roles.get(r)! }, rng);
      trackPeak(r);
    });
  }

  // --- Recruiting: rank the class on what scouts see -------------------------
  const recruitScore = new Map<CareerRecord, number>();
  for (const r of records) {
    const rep = scoutPlayer(r.player, { accuracy: 12, level: 'HS' }, rng);
    recruitScore.set(r, rep.overall + 0.75 * (rep.potential - rep.overall));
  }
  const byRecruitScore = [...records].sort((a, b) => recruitScore.get(b)! - recruitScore.get(a)!);
  let cursor = 0;
  for (const stars of [5, 4, 3, 2] as const) {
    const n = Math.round(PIPELINE.starShares[stars] * size);
    for (let k = 0; k < n && cursor < byRecruitScore.length; k++) byRecruitScore[cursor++]!.stars = stars;
  }
  cursor = 0;
  const collegeCoaching = new Map<CareerRecord, number>();
  for (const level of ['FBS', 'FCS', 'SmallCollege'] as const) {
    const n = Math.round(PIPELINE.collegeShares[level] * size);
    for (let k = 0; k < n && cursor < byRecruitScore.length; k++) {
      const r = byRecruitScore[cursor++]!;
      r.college = level;
      // Blue-chip recruits land at the programs with the best staffs.
      const prestige = level === 'FBS' ? 1 - k / n : 0.5;
      const base = level === 'FBS' ? 11 + 4 * prestige : level === 'FCS' ? 10 : 8.5;
      collegeCoaching.set(r, clamp(rng.normal(base, 2), 1, 20));
    }
  }
  for (const r of records) if (!r.college) r.potentialAtExit = potential(r.player);

  // --- College: four seasons of eligibility in five years --------------------
  const collegians = records.filter((r) => r.college);
  const entrants: { record: CareerRecord; score: number }[] = [];
  const active = new Set(collegians);
  const seasonsPlayed = new Map<CareerRecord, number>();
  const redshirted = new Set<CareerRecord>();
  const collegeStarterShare = [0.08, 0.3, 0.5, 0.65, 0.7];
  for (let year = 0; year < 5 && active.size > 0; year++) {
    for (const level of ['FBS', 'FCS', 'SmallCollege'] as const) {
      const group = [...active].filter((r) => r.college === level);
      const roles = rolesByRank(group, collegeStarterShare[year]!, 0.25);
      for (const r of group) {
        let role = roles.get(r)!;
        if (role === 'backup' && !redshirted.has(r) && year <= 1) {
          role = 'redshirt';
          redshirted.add(r);
        }
        developSeason(r.player, { level, coaching: collegeCoaching.get(r)!, playingTime: role }, rng);
        trackPeak(r);
        if (role !== 'redshirt') seasonsPlayed.set(r, (seasonsPlayed.get(r) ?? 0) + 1);
      }
    }
    for (const r of [...active]) {
      const exhausted = (seasonsPlayed.get(r) ?? 0) >= 4 || year === 4;
      const score = draftScore(r, rng);
      if (exhausted || (year >= 2 && score >= PIPELINE.earlyEntryScore)) {
        active.delete(r);
        r.potentialAtExit = potential(r.player);
        entrants.push({ record: r, score });
      }
    }
  }

  // --- Draft and undrafted free agency ---------------------------------------
  entrants.sort((a, b) => b.score - a.score);
  const draftSlots = Math.max(1, Math.round(PIPELINE.draftPicks * share));
  const udfaSlots = Math.round(draftSlots * PIPELINE.udfaRatio);
  const pros: CareerRecord[] = [];
  entrants.forEach((e, i) => {
    if (i < draftSlots) {
      e.record.draftPick = i + 1;
      pros.push(e.record);
    } else if (i < draftSlots + udfaSlots && overall(e.record.player) >= 48) {
      e.record.udfa = true;
      pros.push(e.record);
    }
  });

  // --- Pro careers ------------------------------------------------------------
  for (const r of pros) simulateProCareer(r, draftSlots, rng);

  return { records, draftSlots };
}

function simulateProCareer(r: CareerRecord, draftSlots: number, rng: Rng): void {
  const p = r.player;
  let coaching = clamp(rng.normal(13, 2.5), 1, 20);
  // Draft capital buys patience: early picks get years to develop before being cut.
  const round = r.draftPick === null ? 8 : Math.ceil((r.draftPick / draftSlots) * 7);
  const protection = [0, 7, 5, 4, 3, 2, 1, 0, 0][round]!;
  for (let season = 0; p.age <= 42; season++) {
    const ovr = overall(p);
    const role: PlayingTime = ovr >= 74 ? 'starter' : ovr >= 66 ? 'rotation' : 'backup';
    const report = developSeason(p, { level: 'NFL', coaching, playingTime: role }, rng);
    r.nflSeasons++;
    trackPeak(r);
    if (season % 3 === 2) coaching = clamp(rng.normal(13, 2.5), 1, 20);

    const after = overall(p);
    const kicker = p.position === 'K' || p.position === 'P';
    const age = p.age;
    const cutLine = 69 + Math.max(0, age - (kicker ? 36 : 30)) * 1.2 - (season < 3 ? protection : 0);
    const keepChance = 1 / (1 + Math.exp(-(after - cutLine) / 2.2));
    if (!rng.chance(keepChance)) return;
    const retireChance =
      0.03 + Math.max(0, age - (kicker ? 36 : 30)) * 0.06 + (report.events.includes('majorInjury') ? 0.08 : 0);
    if (rng.chance(retireChance)) return;
  }
}

/** What a draft room thinks of a prospect: current ability plus some projection. */
function draftScore(r: CareerRecord, rng: Rng): number {
  const rep = scoutPlayer(r.player, { accuracy: 13, level: r.college ?? 'HS' }, rng);
  const youth = clamp((23 - r.player.age) / 3, 0, 1);
  const specialist = r.player.position === 'K' || r.player.position === 'P' ? 8 : 0;
  return rep.overall + (0.25 + 0.2 * youth) * (rep.potential - rep.overall) - specialist;
}

function rolesByRank(group: CareerRecord[], starterShare: number, rotationShare: number): Map<CareerRecord, PlayingTime> {
  const sorted = group.map((r) => ({ r, ovr: overall(r.player) })).sort((a, b) => b.ovr - a.ovr);
  const roles = new Map<CareerRecord, PlayingTime>();
  sorted.forEach(({ r }, i) => {
    const q = i / sorted.length;
    roles.set(r, q < starterShare ? 'starter' : q < starterShare + rotationShare ? 'rotation' : 'backup');
  });
  return roles;
}

function trackPeak(r: CareerRecord): void {
  const last = r.player.history[r.player.history.length - 1];
  if (last && last.overall > r.peakOverall) {
    r.peakOverall = last.overall;
    r.peakAge = last.age;
  }
}

export type { Level };
