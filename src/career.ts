import { developSeason } from './development.js';
import { generateProspect, overall, potential, type Level, type Player, type PlayingTime, type SeasonRecord } from './player.js';
import { Rng, clamp } from './rng.js';
import { buildLevelScale, levelGroupKey, levelRating, positionAligner, setsLevelScale, type LevelScale } from './levelRating.js';
import { scoutPlayer } from './scouting.js';
import type { Position } from './positions.js';

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
  /**
   * Off-field and academic trouble in college. A suspension or academic
   * ineligibility costs a season of games; a dismissal ends his time at the
   * school, and many dismissed players drop a level or leave football. The
   * chance is a year's risk for average work ethic and coachability and runs
   * about three times higher for the least disciplined.
   */
  collegeTrouble: { chance: 0.03, dismissalShare: 0.3, dismissedQuitShare: 0.45 },
  /** Points every draft board takes off for each suspension and dismissal on a player's record. */
  redFlagDiscount: { suspended: 4, dismissed: 8 },
  /**
   * Stars follow what a senior is on film as well as what he might become.
   * Nobody gets four or five stars without being among the best seniors in
   * the country (top 100 for five stars, top 1,000 for four), and nobody in
   * the top 150 goes without four stars, the top 800 without three, or the
   * top 3,000 without two. Kickers and punters get at most three stars.
   */
  starFilmRank: { maxFor5: 100, maxFor4: 1000, minFor4: 150, minFor3: 800, minFor2: 3000 },
} as const;

export type RecruitStars = 0 | 2 | 3 | 4 | 5;
export type CollegeLevel = 'FBS' | 'FCS' | 'SmallCollege';
/** Routes up through the transfer portal. (A dismissed player can also drop a level.) */
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
  /** Left football during college, after a dismissal. */
  leftFootball: boolean;
  /** True potential when leaving college (or high school). */
  potentialAtExit: number;
  /** Consensus draft grade when he left college: what teams saw, after red flags and the competition discount. */
  draftGrade: number | null;
  draftPick: number | null;
  udfa: boolean;
  /** Team that drafted or signed him. */
  team: Team | null;
  nflSeasons: number;
  /** How his pro career ended. */
  proExit: 'cut' | 'retired' | null;
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
      leftFootball: false,
      potentialAtExit: 0,
      draftGrade: null,
      draftPick: null,
      udfa: false,
      team: null,
      nflSeasons: 0,
      proExit: null,
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
  // a senior already is counts for more in who signs where. Film is his senior
  // season: the summer before college comes after signing day. Several
  // services and staffs grade the same film, so between them they read it
  // well. Upside is one shared guess: services follow each other's
  // projections, so pooling them doesn't sharpen it.
  const filmScore = new Map<CareerRecord, number>();
  const projection = new Map<CareerRecord, number>();
  for (const r of records) {
    const seniorSeason = r.player.history[3]!.overallStart;
    const now = overall(r.player);
    let film = 0;
    let upside = 0;
    for (let k = 0; k < 3; k++) {
      const rep = scoutPlayer(r.player, { accuracy: 12, level: 'HS' }, rng);
      film += (seniorSeason + rep.overall - now) / 3;
      if (k === 0) upside = rep.potential - rep.overall;
    }
    filmScore.set(r, film);
    projection.set(r, film + (now - seniorSeason) + upside);
  }
  // Dominance is judged against his own position: quarterbacks and linemen
  // mature later, so the best of them rate lower at 17 than the best backs.
  const alignFilm = positionAligner(records.map((r) => [r.player.position, filmScore.get(r)!] as [Position, number]));
  const starScore = new Map<CareerRecord, number>();
  const recruitScore = new Map<CareerRecord, number>();
  for (const r of records) {
    const film = alignFilm(r.player.position, filmScore.get(r)!);
    filmScore.set(r, film);
    // Few programs spend a high ranking or an early scholarship on a kicker or punter.
    const specialist = isSpecialist(r.player) ? 8 : 0;
    starScore.set(r, 0.25 * film + 0.75 * projection.get(r)! - specialist);
    recruitScore.set(r, 0.4 * film + 0.6 * projection.get(r)!);
  }
  const filmRank = new Map<CareerRecord, number>();
  [...records].sort((a, b) => filmScore.get(b)! - filmScore.get(a)!).forEach((r, i) => filmRank.set(r, i + 1));
  const byStarScore = [...records].sort((a, b) => starScore.get(b)! - starScore.get(a)!);
  const { maxFor5, maxFor4, minFor4, minFor3, minFor2 } = PIPELINE.starFilmRank;
  const maxRank: Record<2 | 3 | 4 | 5, number> = { 5: maxFor5, 4: maxFor4, 3: Infinity, 2: Infinity };
  const mustRank: Record<2 | 3 | 4 | 5, number> = { 5: 0, 4: minFor4, 3: minFor3, 2: minFor2 };
  for (const stars of [5, 4, 3, 2] as const) {
    let n = Math.round(PIPELINE.starShares[stars] * size);
    const ceiling = Math.max(n, Math.round(maxRank[stars] * share));
    const floor = Math.round(mustRank[stars] * share);
    // First the dominant seniors this tier must include, then the best of the rest on upside.
    for (const pass of ['floor', 'rest'] as const) {
      for (const r of byStarScore) {
        if (n <= 0) break;
        const rank = filmRank.get(r)!;
        if (r.stars || rank > ceiling || (pass === 'floor' && rank > floor)) continue;
        if (stars >= 4 && isSpecialist(r.player)) continue;
        r.stars = stars;
        n--;
      }
    }
  }
  let cursor = 0;
  // Offers follow the rankings: every three-star or better signs before anyone
  // below him. Below that, staffs trust their own film over a service's last
  // stars. FBS staffs pick two-stars and unranked seniors on film and upside;
  // FCS and then D2 staffs sign the best of what's left on film.
  const tier = (r: CareerRecord) => (r.stars >= 3 ? r.stars : 0);
  const byRecruitScore = [...records].sort((a, b) => tier(b) - tier(a) || recruitScore.get(b)! - recruitScore.get(a)!);
  const reorderRest = (compare: (a: CareerRecord, b: CareerRecord) => number) =>
    byRecruitScore
      .slice(cursor)
      .sort(compare)
      .forEach((r, k) => (byRecruitScore[cursor + k] = r));
  const collegeCoaching = new Map<CareerRecord, number>();
  for (const level of ['FBS', 'FCS', 'SmallCollege'] as const) {
    const n = Math.round(PIPELINE.collegeShares[level] * size);
    if (level === 'FCS') reorderRest((a, b) => filmScore.get(b)! - filmScore.get(a)!);
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
  const dismissed: CareerRecord[] = [];
  // Players already suspended at their current school: a second incident there ends it.
  const onNotice = new Set<CareerRecord>();
  const collegeStarterShare = [0.08, 0.3, 0.5, 0.65, 0.7];
  for (let year = 0; year < 5 && active.size > 0; year++) {
    for (const level of ['FBS', 'FCS', 'SmallCollege'] as const) {
      const group = [...active].filter((r) => r.college === level);
      const roles = rolesByRank(group, collegeStarterShare[year]!, 0.25);
      for (const r of group) {
        let role = roles.get(r)!;
        let trouble = rng.chance(troubleChance(r.player))
          ? rng.chance(PIPELINE.collegeTrouble.dismissalShare)
            ? ('dismissed' as const)
            : ('suspended' as const)
          : undefined;
        if (trouble === 'suspended' && onNotice.has(r)) trouble = 'dismissed';
        if (trouble === 'suspended') onNotice.add(r);
        if (trouble) role = 'backup';
        else if (role === 'backup' && !redshirted.has(r) && year <= 1) {
          role = 'redshirt';
          redshirted.add(r);
        }
        developSeason(r.player, { level, coaching: collegeCoaching.get(r)!, playingTime: role, trouble }, rng);
        trackPeak(r);
        // A year without games doesn't use one of his four seasons.
        if (role !== 'redshirt' && !trouble) seasonsPlayed.set(r, (seasonsPlayed.get(r) ?? 0) + 1);
        if (trouble === 'dismissed') dismissed.push(r);
      }
    }
    for (const r of [...active]) {
      const exhausted = (seasonsPlayed.get(r) ?? 0) >= 4 || year === 4;
      const score = draftScore(r, rng);
      if (exhausted || (year >= 2 && score >= PIPELINE.earlyEntryScore)) {
        active.delete(r);
        r.potentialAtExit = potential(r.player);
        r.draftGrade = score;
        entrants.push({ record: r, score });
      }
    }
    // A dismissed player who isn't ready to declare starts over a level down, or leaves the game.
    const startingOver = new Set<CareerRecord>();
    for (const r of dismissed.splice(0)) {
      if (!active.has(r)) continue;
      const from = r.college!;
      const to = from === 'FBS' ? 'FCS' : from === 'FCS' ? 'SmallCollege' : null;
      if (to && !rng.chance(PIPELINE.collegeTrouble.dismissedQuitShare)) {
        r.transfers.push({ age: r.player.age, from, to });
        r.college = to;
        onNotice.delete(r);
        startingOver.add(r);
        collegeCoaching.set(r, clamp(rng.normal(to === 'FCS' ? 10 : 8.5, 2), 1, 20));
      } else {
        active.delete(r);
        r.leftFootball = true;
        r.potentialAtExit = potential(r.player);
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
      // A player who turns down a move up has decided to stay this year, and
      // one just dismissed has to prove himself where he landed first.
      const stayed = new Set<CareerRecord>(startingOver);
      for (const route of Object.keys(PIPELINE.transferSlots) as TransferRoute[]) {
        const [from, to] = route.split('>') as ['FCS' | 'SmallCollege', 'FBS' | 'FCS'];
        const candidates = [...active]
          .filter((r) => r.college === from && (seasonsPlayed.get(r) ?? 0) < 4 && !stayed.has(r))
          .map((r) => ({ r, ovr: overall(r.player) }))
          .filter(({ ovr }) => ovr >= medians[to] + PIPELINE.transferMargin[from])
          // Coaches recruit the portal on film, so their order is only roughly by ability.
          .map((c) => ({ ...c, seen: c.ovr + rng.normal(0, 4) }))
          .sort((a, b) => b.seen - a.seen);
        let open = Math.round(PIPELINE.transferSlots[route] * size);
        for (const { r } of candidates) {
          if (open <= 0) break;
          if (!rng.chance(PIPELINE.transferChance)) {
            stayed.add(r);
            continue;
          }
          open--;
          r.transfers.push({ age: r.player.age, from, to });
          r.college = to;
          onNotice.delete(r);
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
  // A team grades each prospect once; its board doesn't change between its picks.
  const boards = new Map<Team, Map<CareerRecord, number>>();
  const grade = (team: Team, r: CareerRecord) => {
    let board = boards.get(team);
    if (!board) boards.set(team, (board = new Map()));
    let offset = board.get(r);
    if (offset === undefined) board.set(r, (offset = rng.normal(0, draftBoardNoise(team.draftSkill))));
    return offset;
  };
  for (let pick = 1; pick <= draftSlots && pool.length > 0; pick++) {
    const team = order[(pick - 1) % order.length]!;
    let bestIndex = 0;
    let bestValue = -Infinity;
    const round = draftRound(pick, draftSlots);
    for (let i = 0; i < Math.min(60, pool.length); i++) {
      if (pool[i]!.record.college === 'SmallCollege' && round < PIPELINE.smallCollegeEarliestRound) continue;
      const value = pool[i]!.score + grade(team, pool[i]!.record);
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

/**
 * Fill in each season's level rating against the national pool for his level
 * and age, using the rating he played that season at. The end-of-year level
 * ranks his rating at the end of the season against everyone else's at the
 * end of theirs.
 */
function attachLevelRatings(records: CareerRecord[], classShare: number): void {
  type Scales = Map<string, { scale: LevelScale; align: ReturnType<typeof positionAligner> }>;
  const buildScales = (pick: (s: SeasonRecord) => number): Scales => {
    const groups = new Map<string, [Position, number][]>();
    for (const r of records) {
      for (const s of r.player.history) {
        const key = levelGroupKey(s.level, s.age);
        if (!key || !setsLevelScale(s.level)) continue;
        let g = groups.get(key);
        if (!g) groups.set(key, (g = []));
        g.push([r.player.position, pick(s)]);
      }
    }
    const scales: Scales = new Map();
    for (const [key, samples] of groups) {
      const align = positionAligner(samples);
      const aligned = samples.map(([position, x]) => align(position, x));
      scales.set(key, { scale: buildLevelScale(aligned, key === 'College' ? 'College' : 'HS', classShare), align });
    }
    return scales;
  };
  const atStart = buildScales((s) => s.overallStart);
  const atEnd = buildScales((s) => s.overall);
  for (const r of records) {
    for (const s of r.player.history) {
      const key = levelGroupKey(s.level, s.age);
      const discount = s.level === 'FCS' || s.level === 'SmallCollege' ? PIPELINE.competitionDiscount[s.level] : 0;
      const rate = (scales: Scales, ovr: number) => {
        const group = key ? scales.get(key) : undefined;
        return levelRating(group ? group.align(r.player.position, ovr - discount) : ovr, s.level, group?.scale);
      };
      s.levelRating = rate(atStart, s.overallStart);
      s.levelRatingEnd = rate(atEnd, s.overall);
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

/** How far off a front office's read of a player on its own roster is, in rating points. */
function boardNoise(skill: number): number {
  return 1.5 + (20 - skill) * 0.35;
}

/**
 * How far off a front office's draft grade is, in rating points: 1 for the
 * best, 3.5 for an average team and about 6 for the worst. Prospects are
 * scouted for years and graded by many people, so an average team's board
 * strays less than its read of a camp body. Real drafts take 1-3 first-
 * rounders a year from outside the consensus top 50.
 */
function draftBoardNoise(skill: number): number {
  return 1 + (20 - skill) * 0.25;
}

function simulateProCareer(r: CareerRecord, draftSlots: number, rng: Rng): void {
  const p = r.player;
  const team = r.team!;
  let coaching = clamp(rng.normal(13, 2.5), 1, 20);
  // Draft capital buys patience: early picks get years to develop before being cut.
  const round = r.draftPick === null ? 8 : draftRound(r.draftPick, draftSlots);
  const protection = [0, 6, 4, 3, 2, 1, 0, 0, 0][round]!;
  // (The age limit is only a safeguard: cuts and retirement end careers.)
  for (let season = 0; p.age <= 50; season++) {
    // Roles go by the rating a player shows, the one on his card.
    const ovr = Math.round(overall(p));
    const role: PlayingTime = ovr >= NFL_LEVELS.starter ? 'starter' : ovr >= NFL_LEVELS.roster + 2 ? 'rotation' : 'backup';
    const report = developSeason(p, { level: 'NFL', coaching, playingTime: role }, rng);
    r.nflSeasons++;
    trackPeak(r);
    if (season % 3 === 2) coaching = clamp(rng.normal(13, 2.5), 1, 20);

    const kicker = isSpecialist(p);
    const age = p.age;
    // Teams judge on what they see, including this season's production.
    const performance = p.history[p.history.length - 1]!.performance ?? report.overallAfter;
    const seen = report.overallAfter * 0.7 + performance * 0.3 + rng.normal(0, boardNoise(team.draftSkill) * 0.5);
    // Roster spots go to whoever beats the next man up, so the real line sits
    // a little above the roster guidance and rises with age.
    const cutLine = NFL_LEVELS.roster + 5 + Math.max(0, age - (kicker ? 35 : 29)) * 1.2 - (season < 3 ? protection : 0);
    const keepChance = 1 / (1 + Math.exp(-(seen - cutLine) / 2));
    if (!rng.chance(keepChance)) {
      r.proExit = 'cut';
      return;
    }
    const major = report.events.includes('majorInjury');
    const retireAge = kicker ? 35 : 30;
    // Stars keep playing while they are still stars; fringe veterans walk away.
    const standing = report.overallAfter >= 85 ? 0.25 : report.overallAfter >= 78 ? 0.5 : 1;
    const retireChance =
      ((age < retireAge ? 0.004 : 0.02 + (age - retireAge) * 0.045) + (major ? (age >= 27 ? 0.15 : 0.04) : 0)) * standing;
    if (rng.chance(retireChance)) {
      r.proExit = 'retired';
      return;
    }
  }
  r.proExit = 'retired';
}

/** Yearly chance of off-field or academic trouble in college. */
function troubleChance(p: Player): number {
  const { workEthic, coachability } = p.personality;
  return PIPELINE.collegeTrouble.chance * clamp(1 + (10 - workEthic) * 0.12 + (10 - coachability) * 0.06, 0.25, 3);
}

function isSpecialist(p: Player): boolean {
  return p.position === 'K' || p.position === 'P';
}

/** What a draft room thinks of a prospect: current ability plus some projection. */
function draftScore(r: CareerRecord, rng: Rng): number {
  const rep = scoutPlayer(r.player, { accuracy: 13, level: r.college ?? 'HS' }, rng);
  const youth = clamp((23 - r.player.age) / 3, 0, 1);
  const specialist = isSpecialist(r.player) ? 8 : 0;
  const competition = PIPELINE.competitionDiscount[r.college ?? 'HS'];
  // Off-field and academic red flags cost a player on every board.
  const { suspended, dismissed } = PIPELINE.redFlagDiscount;
  const flags = r.player.history.reduce(
    (sum, s) => sum + (s.events.includes('dismissed') ? dismissed : s.events.includes('suspended') ? suspended : 0),
    0,
  );
  return rep.overall + (0.25 + 0.2 * youth) * (rep.potential - rep.overall) - specialist - competition - flags;
}

function medianOverall(group: CareerRecord[]): number {
  if (!group.length) return Infinity;
  const xs = group.map((r) => overall(r.player)).sort((a, b) => a - b);
  return xs[Math.floor(xs.length / 2)]!;
}

/** Playing time by rank within each position: a quarterback competes with quarterbacks. */
function rolesByRank(group: CareerRecord[], starterShare: number, rotationShare: number): Map<CareerRecord, PlayingTime> {
  const byPosition = new Map<Position, { r: CareerRecord; ovr: number }[]>();
  for (const r of group) {
    let g = byPosition.get(r.player.position);
    if (!g) byPosition.set(r.player.position, (g = []));
    g.push({ r, ovr: overall(r.player) });
  }
  const roles = new Map<CareerRecord, PlayingTime>();
  for (const g of byPosition.values()) {
    g.sort((a, b) => b.ovr - a.ovr);
    g.forEach(({ r }, i) => {
      const q = i / g.length;
      roles.set(r, q < starterShare ? 'starter' : q < starterShare + rotationShare ? 'rotation' : 'backup');
    });
  }
  return roles;
}

/** Best rating he played a season at. */
function trackPeak(r: CareerRecord): void {
  const last = r.player.history[r.player.history.length - 1];
  if (last && last.overallStart > r.peakOverall) {
    r.peakOverall = last.overallStart;
    r.peakAge = last.age;
  }
}

export type { Level };
