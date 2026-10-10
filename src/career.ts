import { developSeason } from './development.js';
import { generateProspect, overall, potential, type Level, type Player, type PlayingTime } from './player.js';
import { Rng, clamp } from './rng.js';
import { buildLevelScale, levelGroupKey, levelRating, setsLevelScale, type LevelScale } from './levelRating.js';
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
  /** How far above the FBS median a lower-level player must be to get an FBS offer in the portal. */
  transferMargin: { FCS: 0, SmallCollege: 2 } as Record<'FCS' | 'SmallCollege', number>,
  /**
   * Chance a player who earns a transfer up takes it. Most standouts stay put:
   * about 85% of FCS all-conference players with eligibility left return.
   */
  transferChance: 0.2,
  /**
   * Scholarships open to lower-level transfers each year, by route, as a share
   * of the class. Real portals move roughly 400 FCS and 150 D2 players up to
   * FBS a year; D2 to FCS is a smaller, less tracked route.
   */
  transferSlots: { 'FCS>FBS': 0.0004, 'SmallCollege>FBS': 0.00015, 'SmallCollege>FCS': 0.0001 } as Record<TransferRoute, number>,
  /**
   * Points scouts take off a grade for production against weaker opponents.
   * Teams can't be sure a D2 star will hold up against pro athletes.
   */
  competitionDiscount: { FBS: 0, FCS: 1, SmallCollege: 5, HS: 0 } as Record<CollegeLevel | 'HS', number>,
  /** First round in which teams will spend a pick on a D2/D3/NAIA player (day 3 = round 4). */
  smallCollegeEarliestRound: 4,
} as const;

export type RecruitStars = 0 | 2 | 3 | 4 | 5;
export type CollegeLevel = 'FBS' | 'FCS' | 'SmallCollege';
export type TransferRoute = 'FCS>FBS' | 'SmallCollege>FBS' | 'SmallCollege>FCS';

export interface CareerRecord {
  player: Player;
  initialOverall: number;
  initialPotential: number;
  /** Recruiting stars at signing (0 = unranked). */
  stars: RecruitStars;
  /** College he finished at (after any transfers). */
  college: CollegeLevel | null;
  /** College that first signed him out of high school. */
  signedWith: CollegeLevel | null;
  transfers: { age: number; from: CollegeLevel; to: CollegeLevel }[];
  /** True potential when leaving college (or high school). */
  potentialAtExit: number;
  draftPick: number | null;
  udfa: boolean;
  /** Team that drafted or signed him. */
  team: Team | null;
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
      signedWith: null,
      transfers: [],
      potentialAtExit: 0,
      draftPick: null,
      udfa: false,
      team: null,
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
  // Recruiting services rank on upside; college staffs offer on film, so what
  // a senior already is counts for more in who signs where.
  const starScore = new Map<CareerRecord, number>();
  const recruitScore = new Map<CareerRecord, number>();
  for (const r of records) {
    const rep = scoutPlayer(r.player, { accuracy: 12, level: 'HS' }, rng);
    // Kickers and punters are ranked on their own lists and almost never earn four or five stars.
    const specialist = r.player.position === 'K' || r.player.position === 'P' ? 8 : 0;
    starScore.set(r, rep.overall + 0.75 * (rep.potential - rep.overall) - specialist);
    recruitScore.set(r, rep.overall + 0.6 * (rep.potential - rep.overall));
  }
  const byStarScore = [...records].sort((a, b) => starScore.get(b)! - starScore.get(a)!);
  let cursor = 0;
  for (const stars of [5, 4, 3, 2] as const) {
    const n = Math.round(PIPELINE.starShares[stars] * size);
    for (let k = 0; k < n && cursor < byStarScore.length; k++) byStarScore[cursor++]!.stars = stars;
  }
  const byRecruitScore = [...records].sort((a, b) => recruitScore.get(b)! - recruitScore.get(a)!);
  cursor = 0;
  const collegeCoaching = new Map<CareerRecord, number>();
  for (const level of ['FBS', 'FCS', 'SmallCollege'] as const) {
    const n = Math.round(PIPELINE.collegeShares[level] * size);
    for (let k = 0; k < n && cursor < byRecruitScore.length; k++) {
      const r = byRecruitScore[cursor++]!;
      r.college = level;
      r.signedWith = level;
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
    // Transfer portal: standouts at lower levels move up for the exposure,
    // coaching and competition, while they still have eligibility left.
    // Each level only has so many open scholarships for transfers.
    if (year <= 3) {
      const medians = {
        FBS: medianOverall([...active].filter((r) => r.college === 'FBS')),
        FCS: medianOverall([...active].filter((r) => r.college === 'FCS')),
      };
      for (const route of Object.keys(PIPELINE.transferSlots) as TransferRoute[]) {
        const [from, to] = route.split('>') as ['FCS' | 'SmallCollege', 'FBS' | 'FCS'];
        const candidates = [...active]
          .filter((r) => r.college === from && (seasonsPlayed.get(r) ?? 0) < 4)
          .map((r) => ({ r, ovr: overall(r.player) }))
          .filter(({ ovr }) => ovr >= medians[to] + PIPELINE.transferMargin[from])
          // Coaches recruit the portal on film, so their order is only roughly by ability.
          .map((c) => ({ ...c, seen: c.ovr + rng.normal(0, 4) }))
          .sort((a, b) => b.seen - a.seen);
        let open = Math.round(PIPELINE.transferSlots[route] * size);
        for (const { r } of candidates) {
          if (open <= 0) break;
          if (!rng.chance(PIPELINE.transferChance)) continue;
          open--;
          r.transfers.push({ age: r.player.age, from, to });
          r.college = to;
          collegeCoaching.set(r, clamp(rng.normal(to === 'FBS' ? 12 : 10, 2), 1, 20));
        }
      }
    }
  }

  // --- Draft and undrafted free agency ---------------------------------------
  // Each team picks from its own board. A team's board is the consensus grade
  // seen through its front office: good GMs see clearly, bad ones reach.
  const teams = makeTeams(rng);
  const order = [...teams].sort(() => rng.next() - 0.5);
  entrants.sort((a, b) => b.score - a.score);
  const draftSlots = Math.max(1, Math.round(PIPELINE.draftPicks * share));
  const udfaSlots = Math.round(draftSlots * PIPELINE.udfaRatio);
  const pros: CareerRecord[] = [];
  const pool = [...entrants];
  for (let pick = 1; pick <= draftSlots && pool.length > 0; pick++) {
    const team = order[(pick - 1) % order.length]!;
    let bestIndex = 0;
    let bestValue = -Infinity;
    const round = draftRound(pick, draftSlots);
    for (let i = 0; i < Math.min(60, pool.length); i++) {
      if (pool[i]!.record.college === 'SmallCollege' && round < PIPELINE.smallCollegeEarliestRound) continue;
      const value = pool[i]!.score + rng.normal(0, boardNoise(team.draftSkill));
      if (value > bestValue) {
        bestValue = value;
        bestIndex = i;
      }
    }
    const [chosen] = pool.splice(bestIndex, 1);
    chosen!.record.draftPick = pick;
    chosen!.record.team = team;
    pros.push(chosen!.record);
  }
  for (const e of pool.slice(0, udfaSlots)) {
    if (overall(e.record.player) < 48) continue;
    e.record.udfa = true;
    e.record.team = rng.pick(teams);
    pros.push(e.record);
  }

  // --- Pro careers ------------------------------------------------------------
  for (const r of pros) simulateProCareer(r, draftSlots, rng);

  attachLevelRatings(records, share);
  return { records, draftSlots };
}

/** Fill in each season's level rating against the national pool for his level and age. */
function attachLevelRatings(records: CareerRecord[], classShare: number): void {
  const groups = new Map<string, number[]>();
  for (const r of records) {
    for (const s of r.player.history) {
      const key = levelGroupKey(s.level, s.age);
      if (!key || !setsLevelScale(s.level)) continue;
      let g = groups.get(key);
      if (!g) groups.set(key, (g = []));
      g.push(s.overall);
    }
  }
  const scales = new Map<string, LevelScale>();
  for (const [key, overalls] of groups) scales.set(key, buildLevelScale(overalls, key === 'College' ? 'College' : 'HS', classShare));
  for (const r of records) {
    for (const s of r.player.history) {
      const key = levelGroupKey(s.level, s.age);
      const discount = s.level === 'FCS' || s.level === 'SmallCollege' ? PIPELINE.competitionDiscount[s.level] : 0;
      s.levelRating = levelRating(s.overall - discount, s.level, key ? scales.get(key) : undefined);
    }
  }
}

/**
 * Last pick of each round in a real 257-pick draft (2025). Rounds 1 and 2 have
 * 32 picks; compensatory picks make rounds 3-7 longer.
 */
const ROUND_ENDS = [32, 64, 102, 138, 176, 216, 257] as const;

/** Round of a pick, with the real round sizes scaled to the class's number of picks. */
export function draftRound(pick: number, draftSlots: number): number {
  const scale = draftSlots / ROUND_ENDS[ROUND_ENDS.length - 1]!;
  const i = ROUND_ENDS.findIndex((end) => pick <= Math.round(end * scale));
  return i === -1 ? ROUND_ENDS.length : i + 1;
}

/**
 * Rating guidance for the pro game. These are what the user sees as rough
 * bands; actual roster decisions are made by teams through their own eyes.
 */
export const NFL_LEVELS = { roster: 65, starter: 75, proBowl: 85, elite: 92 } as const;

export interface Team {
  id: number;
  /** Front office judgment, 1-20. Will come from GM and owner attributes. */
  draftSkill: number;
}

function makeTeams(rng: Rng): Team[] {
  return Array.from({ length: 32 }, (_, id) => ({ id, draftSkill: clamp(Math.round(rng.normal(10.5, 3.5)), 1, 20) }));
}

/** How far off a front office's read of a player is, in rating points. */
function boardNoise(skill: number): number {
  return 1.5 + (20 - skill) * 0.35;
}

function simulateProCareer(r: CareerRecord, draftSlots: number, rng: Rng): void {
  const p = r.player;
  const team = r.team!;
  let coaching = clamp(rng.normal(13, 2.5), 1, 20);
  // Draft capital buys patience: early picks get years to develop before being cut.
  const round = r.draftPick === null ? 8 : draftRound(r.draftPick, draftSlots);
  const protection = [0, 6, 4, 3, 2, 1, 0, 0, 0][round]!;
  for (let season = 0; p.age <= 42; season++) {
    const ovr = overall(p);
    const role: PlayingTime = ovr >= NFL_LEVELS.starter ? 'starter' : ovr >= NFL_LEVELS.roster + 2 ? 'rotation' : 'backup';
    const report = developSeason(p, { level: 'NFL', coaching, playingTime: role }, rng);
    r.nflSeasons++;
    trackPeak(r);
    if (season % 3 === 2) coaching = clamp(rng.normal(13, 2.5), 1, 20);

    const kicker = p.position === 'K' || p.position === 'P';
    const age = p.age;
    // Teams judge on what they see, including this season's production.
    const seen = report.overallAfter * 0.7 + p.history[p.history.length - 1]!.performance * 0.3 + rng.normal(0, boardNoise(team.draftSkill) * 0.5);
    // Roster spots go to whoever beats the next man up, so the real line sits
    // a little above the roster guidance and rises with age.
    const cutLine = NFL_LEVELS.roster + 5 + Math.max(0, age - (kicker ? 35 : 29)) * 1.2 - (season < 3 ? protection : 0);
    const keepChance = 1 / (1 + Math.exp(-(seen - cutLine) / 2));
    if (!rng.chance(keepChance)) return;
    const major = report.events.includes('majorInjury');
    const retireAge = kicker ? 35 : 30;
    // Stars keep playing while they are still stars; fringe veterans walk away.
    const standing = report.overallAfter >= 85 ? 0.25 : report.overallAfter >= 78 ? 0.5 : 1;
    const retireChance =
      ((age < retireAge ? 0.004 : 0.02 + (age - retireAge) * 0.045) + (major ? (age >= 27 ? 0.15 : 0.04) : 0)) * standing;
    if (rng.chance(retireChance)) return;
  }
}

/** What a draft room thinks of a prospect: current ability plus some projection. */
function draftScore(r: CareerRecord, rng: Rng): number {
  const rep = scoutPlayer(r.player, { accuracy: 13, level: r.college ?? 'HS' }, rng);
  const youth = clamp((23 - r.player.age) / 3, 0, 1);
  const specialist = r.player.position === 'K' || r.player.position === 'P' ? 8 : 0;
  const competition = PIPELINE.competitionDiscount[r.college ?? 'HS'];
  return rep.overall + (0.25 + 0.2 * youth) * (rep.potential - rep.overall) - specialist - competition;
}

function medianOverall(group: CareerRecord[]): number {
  if (!group.length) return Infinity;
  const xs = group.map((r) => overall(r.player)).sort((a, b) => a - b);
  return xs[Math.floor(xs.length / 2)]!;
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
