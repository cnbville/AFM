import { ATTRIBUTE_KEYS, ATTRIBUTE_PROFILES } from './attributes.js';
import { developSeason } from './development.js';
import { generateProspect, overall, potential, type Level, type Player, type PlayingTime, type SeasonRecord } from './player.js';
import { Rng, clamp } from './rng.js';
import { buildLevelScale, levelGroupKey, levelRating, positionAligner, setsLevelScale, type LevelScale } from './levelRating.js';
import { scoutPlayer } from './scouting.js';
import { POSITION_PROFILES, type Position } from './positions.js';
import { DEPTH_SLOTS, PRACTICE_SQUAD, advanceLeague, makeLeague, organisation, teamStrength, weakestStarter, type Member, type Team } from './league.js';

/**
 * Simulates a whole national high school class from freshman year to the end
 * of every pro career. Roster slots at each level are a fixed share of the
 * class, matching real participation: about 1.3% sign with FBS schools, 1% with
 * FCS, 4.5% with smaller colleges, and roughly 257 per 250,000 are drafted.
 * The pros are a full league of 32 teams (see league.ts) whose depth charts
 * the class competes for.
 */
export const PIPELINE = {
  /** Real US freshman class size the shares below are expressed against. */
  referenceClassSize: 250_000,
  starShares: { 5: 0.00013, 4: 0.0014, 3: 0.008, 2: 0.016 } as Record<RecruitStars, number>,
  collegeShares: { FBS: 0.013, FCS: 0.01, SmallCollege: 0.045 },
  draftPicks: 257,
  /** UDFAs signed per drafted player. */
  udfaRatio: 1.6,
  /** How far above the FBS median a lower-level player must be to get an FBS offer in the portal. */
  transferMargin: { FCS: 0, SmallCollege: 2 } as Record<'FCS' | 'SmallCollege', number>,
  /**
   * Chance a player who earns a transfer up takes it. Most standouts stay put:
   * about 85% of FCS all-conference players with eligibility left return.
   */
  transferChance: 0.15,
  /**
   * Scholarships open to lower-level transfers each year, by route, as a share
   * of the class. Real portals move roughly 400 FCS and 150 D2 players up to
   * FBS a year; D2 to FCS is a smaller, less tracked route.
   */
  transferSlots: { 'FCS>FBS': 0.0004, 'SmallCollege>FBS': 0.00015, 'SmallCollege>FCS': 0.0001 } as Record<UpRoute, number>,
  /**
   * Yearly chance a player enters the portal looking for playing time, by
   * role. A coaching change at his school makes it more likely.
   */
  portalEntry: { backup: 0.18, rotation: 0.07, coachingChange: 1.8 },
  /** Yearly chance his college's head coach leaves or is fired. */
  collegeCoachingChange: 0.12,
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
   * school. The chance is a year's risk for average work ethic and
   * coachability and runs about three times higher for the least disciplined.
   * A serious cause (violence, a felony charge) drops him a level or ends his
   * career; a lesser one (academics, a failed drug test, team rules) usually
   * means a fresh start at a smaller FBS program if he is good enough.
   */
  collegeTrouble: { chance: 0.03, dismissalShare: 0.3, seriousShare: 0.35, seriousQuitShare: 0.45, lesserQuitShare: 0.1 },
  /** Points every draft board takes off for each suspension and dismissal on a player's record. */
  redFlagDiscount: { suspended: 4, dismissed: 8 },
  /**
   * Stars follow what a senior is on film as well as what he might become.
   * Nobody gets four or five stars without being among the best seniors in
   * the country (top 100 for five stars, top 1,000 for four, 2,700 for three
   * and 6,500 for two), and nobody in the top 150 goes without four stars,
   * the top 800 without three, or the top 3,000 without two. Kickers and
   * punters get at most three stars. (Film line option C.)
   */
  starFilmRank: { maxFor5: 100, maxFor4: 1000, maxFor3: 2700, maxFor2: 6500, minFor4: 150, minFor3: 800, minFor2: 3000 },
  /** Worst senior film rank an FBS staff will sign below three stars. */
  fbsOfferMaxFilmRank: 4500,
  /**
   * General draft value by position, in grade points: teams pay for
   * quarterbacks, pass rushers, tackles and corners, and wait on backs.
   */
  positionValue: { QB: 3.5, EDGE: 1.5, OT: 1.5, CB: 1, WR: 0.5, DT: 0.5, IOL: -0.8, S: -0.8, LB: -1, TE: -1, RB: -3.5, K: 0, P: 0 } as Record<Position, number>,
  /**
   * Roster value a rookie contract buys, by round (UDFAs last): guaranteed
   * money makes teams patient with early picks. It fades out over the deal.
   */
  rookieProtection: [0, 9, 6, 4, 2.5, 1.5, 1, 0.5, 0] as readonly number[],
  /**
   * Typical age a player at each position walks away. Longevity, love of the
   * game, wear and still being good move it a lot. A quarterback who relies
   * on his legs leaves earlier than a pocket passer.
   */
  retireAge: { QB: 35, RB: 28, WR: 30, TE: 31, OT: 32, IOL: 32, EDGE: 30, DT: 31, LB: 30, CB: 30, S: 30, K: 37, P: 37 } as Record<Position, number>,
  runningQbRetireOffset: -4,
} as const;

export type RecruitStars = 0 | 2 | 3 | 4 | 5;
export type CollegeLevel = 'FBS' | 'FCS' | 'SmallCollege';
/** Routes up through the transfer portal for standouts at lower levels. */
export type UpRoute = 'FCS>FBS' | 'SmallCollege>FBS' | 'SmallCollege>FCS';
export type TransferReason = 'standout' | 'playingTime' | 'coachingChange' | 'dismissedLesser' | 'dismissedSerious';
export type ProExit = 'retired' | 'unsigned';

export interface CareerRecord {
  player: Player;
  initialOverall: number;
  initialPotential: number;
  /** Recruiting stars at signing (0 = unranked). */
  stars: RecruitStars;
  /** Where his senior film ranked nationally (1 = best). */
  seniorFilmRank: number;
  /** College he finished at (after any transfers). */
  college: CollegeLevel | null;
  /** College that first signed him out of high school. */
  signedWith: CollegeLevel | null;
  /** Profile of his current FBS program, 0-1 (1 = a national power); null below FBS. */
  prestige: number | null;
  transfers: { age: number; from: CollegeLevel; to: CollegeLevel; reason: TransferReason }[];
  /** Every portal entry, including those that found no school. */
  portal: { age: number; from: CollegeLevel; reason: TransferReason; outcome: 'up' | 'sideways' | 'down' | 'none' }[];
  /** Left football during college (a dismissal, or the portal found him no school). */
  leftFootball: boolean;
  /** True potential when leaving college (or high school). */
  potentialAtExit: number;
  /** Consensus draft grade when he left college: what teams saw, after red flags and the competition discount. */
  draftGrade: number | null;
  draftPick: number | null;
  udfa: boolean;
  /** Team that drafted or first signed him. */
  team: Team | null;
  /** Every pro team he was with, in order. */
  proTeams: string[];
  /** Seasons on an active roster. */
  nflSeasons: number;
  practiceSquadSeasons: number;
  /** How his pro career ended: retired, or retired after going unsigned. */
  proExit: ProExit | null;
  peakOverall: number;
  peakAge: number;
  /** A quarterback whose game leans on his legs. */
  runningQB: boolean;
  /** Plain-language reasons for the notable things that happened to him, for trial cards. */
  notes: string[];
}

export interface CohortResult {
  records: CareerRecord[];
  draftSlots: number;
  teams: Team[];
}

export interface CohortOptions {
  size: number;
  seed: number | string;
}

const LEVEL_NAME: Record<CollegeLevel, string> = { FBS: 'FBS', FCS: 'FCS', SmallCollege: 'D2' };
const LEVEL_RANK: Record<CollegeLevel, number> = { SmallCollege: 1, FCS: 2, FBS: 3 };

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
      seniorFilmRank: 0,
      college: null,
      signedWith: null,
      prestige: null,
      transfers: [],
      portal: [],
      leftFootball: false,
      potentialAtExit: 0,
      draftGrade: null,
      draftPick: null,
      udfa: false,
      team: null,
      proTeams: [],
      nflSeasons: 0,
      practiceSquadSeasons: 0,
      proExit: null,
      peakOverall: overall(player),
      peakAge: player.age,
      runningQB: false,
      notes: [],
    });
    hsCoaching.push(clamp(rng.normal(8, 3), 1, 20));
  }
  // The pro league exists the whole time: it ages and changes staffs while the class grows up.
  const teams = makeLeague(rng);

  // --- High school: four seasons, ages 14-17 -------------------------------
  // Varsity starting spots get easier to earn as a class gets older.
  const hsStarterShare = [0.12, 0.3, 0.5, 0.6];
  for (let year = 0; year < 4; year++) {
    const roles = rolesByRank(records, hsStarterShare[year]!, 0.2, (r) => overall(r.player));
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
  // projections, so pooling them doesn't sharpen it. A senior who missed much
  // of his last season to injury has less film, and what there is worries staffs.
  const filmScore = new Map<CareerRecord, number>();
  const projection = new Map<CareerRecord, number>();
  for (const r of records) {
    const senior = r.player.history[3]!;
    const seniorSeason = senior.overallStart;
    const now = overall(r.player);
    let film = 0;
    let upside = 0;
    for (let k = 0; k < 3; k++) {
      const rep = scoutPlayer(r.player, { accuracy: 12, level: 'HS' }, rng);
      film += (seniorSeason + rep.overall - now) / 3;
      if (k === 0) upside = rep.potential - rep.overall;
    }
    const missed = (senior.gamesMissed ?? 0) / 10;
    if (missed > 0.3) {
      film -= 6 * missed;
      if (senior.events.includes('majorInjury')) r.notes.push(`A major injury cost him ${senior.gamesMissed} of 10 games as a senior, so recruiters had little senior film and ranked him lower.`);
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
  [...records].sort((a, b) => filmScore.get(b)! - filmScore.get(a)!).forEach((r, i) => {
    filmRank.set(r, i + 1);
    r.seniorFilmRank = i + 1;
  });
  const byStarScore = [...records].sort((a, b) => starScore.get(b)! - starScore.get(a)!);
  const { maxFor5, maxFor4, maxFor3, maxFor2, minFor4, minFor3, minFor2 } = PIPELINE.starFilmRank;
  const maxRank: Record<2 | 3 | 4 | 5, number> = { 5: maxFor5, 4: maxFor4, 3: maxFor3, 2: maxFor2 };
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
  /** First-year dip at his current school, cleared after that year. */
  const pendingDip = new Map<CareerRecord, number>();
  for (const level of ['FBS', 'FCS', 'SmallCollege'] as const) {
    const n = Math.round(PIPELINE.collegeShares[level] * size);
    if (level === 'FBS') {
      const limit = PIPELINE.fbsOfferMaxFilmRank * share;
      const onFilm = (r: CareerRecord) => (tier(r) || filmRank.get(r)! <= limit ? 1 : 0);
      reorderRest((a, b) => onFilm(b) - onFilm(a) || tier(b) - tier(a) || recruitScore.get(b)! - recruitScore.get(a)!);
    }
    if (level === 'FCS') reorderRest((a, b) => filmScore.get(b)! - filmScore.get(a)!);
    for (let k = 0; k < n && cursor < byRecruitScore.length; k++) {
      const r = byRecruitScore[cursor++]!;
      r.college = level;
      r.signedWith = level;
      // Blue-chip recruits land at the programs with the best staffs.
      if (level === 'FBS') r.prestige = 1 - k / n;
      collegeCoaching.set(r, programCoaching(r, rng));
      const adj = firstYearAdjustment(r.player, `HS>${level}`, rng);
      pendingDip.set(r, adj.dip);
      noteDip(r, adj, `college (${LEVEL_NAME[level]})`);
    }
  }
  for (const r of records) if (!r.college) r.potentialAtExit = potential(r.player);

  // --- College: four seasons of eligibility in five years --------------------
  const collegians = records.filter((r) => r.college);
  const entrants: CareerRecord[] = [];
  const active = new Set(collegians);
  const seasonsPlayed = new Map<CareerRecord, number>();
  const redshirted = new Set<CareerRecord>();
  const lastRole = new Map<CareerRecord, PlayingTime>();
  const dismissed: { r: CareerRecord; serious: boolean; cause: string }[] = [];
  // Players already suspended at their current school: a second incident there ends it.
  const onNotice = new Set<CareerRecord>();
  const collegeStarterShare = [0.08, 0.3, 0.5, 0.65, 0.7];
  for (let year = 0; year < 5 && active.size > 0; year++) {
    advanceLeague(teams, rng);
    const coachingChanged = new Set<CareerRecord>();
    for (const level of ['FBS', 'FCS', 'SmallCollege'] as const) {
      const group = [...active].filter((r) => r.college === level);
      // Staffs pick starters on what they see in camp: a player still adjusting
      // looks worse than his rating. Young players also have to beat the
      // upperclassmen ahead of them, who are better at bigger programs.
      const blocked = year === 0 ? 10 : year === 1 ? 5 : 0;
      const roles = rolesByRank(group, collegeStarterShare[year]!, 0.25, (r) => overall(r.player) - 12 * (pendingDip.get(r) ?? 0) - blocked * (r.prestige ?? 0.3) + rng.normal(0, 1.5));
      for (const r of group) {
        let role = roles.get(r)!;
        let trouble = rng.chance(troubleChance(r.player))
          ? rng.chance(PIPELINE.collegeTrouble.dismissalShare)
            ? ('dismissed' as const)
            : ('suspended' as const)
          : undefined;
        let cause = '';
        if (trouble === 'suspended' && onNotice.has(r)) {
          trouble = 'dismissed';
          cause = 'a second suspension at the same school';
        }
        if (trouble === 'suspended') onNotice.add(r);
        if (trouble) role = 'backup';
        else if (role === 'backup' && !redshirted.has(r) && year <= 1) {
          role = 'redshirt';
          redshirted.add(r);
        }
        const dip = pendingDip.get(r);
        pendingDip.delete(r);
        developSeason(r.player, { level, coaching: collegeCoaching.get(r)!, playingTime: role, trouble, adjustment: dip }, rng);
        addWear(r, role, 0.5);
        trackPeak(r);
        // A year without games doesn't use one of his four seasons.
        if (role !== 'redshirt' && !trouble) seasonsPlayed.set(r, (seasonsPlayed.get(r) ?? 0) + 1);
        if (trouble === 'dismissed') {
          const serious = !cause && rng.chance(PIPELINE.collegeTrouble.seriousShare);
          cause ||= serious ? rng.pick(SERIOUS_CAUSES) : rng.pick(LESSER_CAUSES);
          dismissed.push({ r, serious, cause });
        }
        if (trouble === 'suspended') r.notes.push(`Suspended for the season at ${r.player.age - 1} (${rng.pick(SUSPENSION_CAUSES)}).`);
        lastRole.set(r, role);
        if (rng.chance(PIPELINE.collegeCoachingChange)) {
          coachingChanged.add(r);
          collegeCoaching.set(r, programCoaching(r, rng));
        }
      }
    }

    // --- Declaring for the draft: projection plus urgency ---------------------
    const eligible = [...active].filter((r) => year >= 2 || (seasonsPlayed.get(r) ?? 0) >= 4);
    const grades = new Map<CareerRecord, ReturnType<typeof draftScore>>();
    for (const r of eligible) grades.set(r, draftScore(r, rng));
    const board = [...eligible, ...entrants].map((r) => r.draftGrade ?? grades.get(r)!.score).sort((a, b) => b - a);
    const draftSlots = Math.max(1, Math.round(PIPELINE.draftPicks * share));
    const order = draftOrder(teams, rng);
    for (const r of eligible) {
      const g = grades.get(r)!;
      const exhausted = (seasonsPlayed.get(r) ?? 0) >= 4 || year === 4;
      const projectedPick = board.findIndex((s) => s <= g.score) + 1;
      const declare = exhausted || decideToDeclare(r, g, projectedPick, draftSlots, order, { coachingChanged: coachingChanged.has(r), lastRole: lastRole.get(r)! }, rng);
      if (declare) {
        active.delete(r);
        r.potentialAtExit = potential(r.player);
        r.draftGrade = g.score;
        entrants.push(r);
      }
    }

    // A dismissed player who isn't ready to declare starts over, a level down or at a smaller program, or leaves the game.
    const startingOver = new Set<CareerRecord>();
    const sorted = levelRatings(active);
    for (const { r, serious, cause } of dismissed.splice(0)) {
      if (!active.has(r)) continue;
      const from = r.college!;
      const ovr = overall(r.player);
      let to: CollegeLevel | null;
      if (serious) {
        to = rng.chance(PIPELINE.collegeTrouble.seriousQuitShare) ? null : levelBelow(from);
      } else if (rng.chance(PIPELINE.collegeTrouble.lesserQuitShare)) {
        to = null;
      } else {
        // Better players get more offers: a good enough one can stay at his level at a smaller program.
        const stayChance = clamp(0.5 + (ovr - percentile(sorted[from], 0.5)) / 8, 0.05, 0.95);
        to = rng.chance(stayChance) ? from : levelBelow(from);
      }
      const age = r.player.age - 1;
      if (to) {
        moveSchool(r, to, serious ? 'dismissedSerious' : 'dismissedLesser', age, rng, collegeCoaching, pendingDip);
        onNotice.delete(r);
        startingOver.add(r);
        const where = to === from ? `a lower-profile ${LEVEL_NAME[to]} program` : `${LEVEL_NAME[to]}, a level down`;
        r.notes.push(`Dismissed at ${age} for ${cause}${serious ? ', a serious cause' : ', a lesser cause'}. ${to === from ? 'Good enough to keep offers at his level, he' : 'He'} moved to ${where}.`);
      } else {
        active.delete(r);
        r.leftFootball = true;
        r.potentialAtExit = potential(r.player);
        r.notes.push(`Dismissed at ${age} for ${cause}${serious ? ', a serious cause,' : ''} and never played again.`);
      }
    }

    // --- Transfer portal ----------------------------------------------------
    if (year <= 3) {
      const levels = levelRatings(active);
      // A player who turns down a move up has decided to stay this year, and
      // one just dismissed has to prove himself where he landed first.
      const stayed = new Set<CareerRecord>(startingOver);
      // Standouts at lower levels move up for the exposure, coaching and
      // competition. Each level only has so many open scholarships for transfers.
      for (const route of Object.keys(PIPELINE.transferSlots) as UpRoute[]) {
        const [from, to] = route.split('>') as ['FCS' | 'SmallCollege', 'FBS' | 'FCS'];
        const candidates = [...active]
          .filter((r) => r.college === from && (seasonsPlayed.get(r) ?? 0) < 4 && !stayed.has(r))
          .map((r) => ({ r, ovr: overall(r.player) }))
          .filter(({ ovr }) => ovr >= percentile(levels[to], 0.5) + PIPELINE.transferMargin[from])
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
          const age = r.player.age - 1;
          r.portal.push({ age, from, reason: 'standout', outcome: 'up' });
          moveSchool(r, to, 'standout', age, rng, collegeCoaching, pendingDip);
          stayed.add(r);
          r.notes.push(`At ${age} he was rated above the median ${LEVEL_NAME[to]} player, entered the portal and moved up to ${LEVEL_NAME[to]}.`);
        }
      }
      // Players short of snaps look for a new school, more so after a coaching
      // change. Where they land depends on how the market rates them: across,
      // a level down, or nowhere.
      for (const r of [...active]) {
        if (stayed.has(r) || (seasonsPlayed.get(r) ?? 0) >= 4 || seasonsPlayed.get(r) === undefined) continue;
        const role = lastRole.get(r);
        const base = role === 'backup' ? PIPELINE.portalEntry.backup : role === 'rotation' ? PIPELINE.portalEntry.rotation : 0;
        const changed = coachingChanged.has(r);
        const p = base * (changed ? PIPELINE.portalEntry.coachingChange : 1) * (1 + (r.player.personality.competitiveness - 10) * 0.04);
        if (!rng.chance(p)) continue;
        const from = r.college!;
        const reason: TransferReason = changed ? 'coachingChange' : 'playingTime';
        const seen = overall(r.player) + rng.normal(0, 3);
        const age = r.player.age - 1;
        const why = changed ? 'after a coaching change' : `stuck as a ${role}`;
        let to: CollegeLevel | null = null;
        if (seen >= percentile(levels[from], 0.4)) to = from;
        else if (from !== 'SmallCollege' && seen >= percentile(levels[levelBelow(from)!], 0.45)) to = levelBelow(from);
        else if (from === 'FBS' && seen >= percentile(levels.SmallCollege, 0.5) && rng.chance(0.4)) to = 'SmallCollege';
        if (!to) {
          r.portal.push({ age, from, reason, outcome: 'none' });
          active.delete(r);
          r.leftFootball = true;
          r.potentialAtExit = potential(r.player);
          r.notes.push(`Entered the portal at ${age} ${why}, found no school and left football.`);
          continue;
        }
        r.portal.push({ age, from, reason, outcome: to === from ? 'sideways' : 'down' });
        moveSchool(r, to, reason, age, rng, collegeCoaching, pendingDip);
        r.notes.push(`Entered the portal at ${age} ${why} and moved ${to === from ? `across to another ${LEVEL_NAME[to]} school` : `down to ${LEVEL_NAME[to]}`} for playing time.`);
      }
    }
  }

  // --- Draft and undrafted free agency ---------------------------------------
  const draftSlots = Math.max(1, Math.round(PIPELINE.draftPicks * share));
  const pros = runDraft(entrants, teams, draftSlots, rng);

  // --- Pro careers ------------------------------------------------------------
  simulateLeagueYears(pros, teams, draftSlots, rng);

  attachLevelRatings(records, share);
  return { records, draftSlots, teams };
}

const SERIOUS_CAUSES = ['an arrest for assault', 'a felony charge', 'a violent incident off the field'] as const;
const LESSER_CAUSES = ['academic ineligibility', 'a failed drug test', 'repeated violations of team rules'] as const;
const SUSPENSION_CAUSES = ['academics', 'a failed drug test', 'a violation of team rules'] as const;

function levelBelow(level: CollegeLevel): CollegeLevel | null {
  return level === 'FBS' ? 'FCS' : level === 'FCS' ? 'SmallCollege' : null;
}

/** Coaching at his current program: FBS powers have the best staffs. */
function programCoaching(r: CareerRecord, rng: Rng): number {
  const base = r.college === 'FBS' ? 11 + 4 * (r.prestige ?? 0.5) : r.college === 'FCS' ? 11 : 8.5;
  return clamp(rng.normal(base, 2), 1, 20);
}

function moveSchool(
  r: CareerRecord,
  to: CollegeLevel,
  reason: TransferReason,
  age: number,
  rng: Rng,
  coaching: Map<CareerRecord, number>,
  pendingDip: Map<CareerRecord, number>,
): void {
  const from = r.college!;
  r.transfers.push({ age, from, to, reason });
  // A move across goes to a smaller program; a move up lands about mid-pack.
  r.prestige = to !== 'FBS' ? null : from === 'FBS' ? clamp((r.prestige ?? 0.5) - rng.range(0.15, 0.45), 0, 0.6) : rng.range(0.2, 0.7);
  r.college = to;
  coaching.set(r, programCoaching(r, rng));
  const step = LEVEL_RANK[to] > LEVEL_RANK[from] ? 'up' : LEVEL_RANK[to] === LEVEL_RANK[from] ? 'sideways' : 'down';
  const adj = firstYearAdjustment(r.player, step, rng);
  pendingDip.set(r, adj.dip);
}

/** Sorted ratings of the active players at each level. */
function levelRatings(active: Set<CareerRecord>): Record<CollegeLevel, number[]> {
  const out: Record<CollegeLevel, number[]> = { FBS: [], FCS: [], SmallCollege: [] };
  for (const r of active) out[r.college!].push(overall(r.player));
  for (const k of Object.keys(out) as CollegeLevel[]) out[k].sort((a, b) => a - b);
  return out;
}

function percentile(sorted: number[], q: number): number {
  if (!sorted.length) return Infinity;
  return sorted[Math.min(sorted.length - 1, Math.floor(q * sorted.length))]!;
}

// --- First-year adjustment ---------------------------------------------------

export type Step = 'HS>FBS' | 'HS>FCS' | 'HS>SmallCollege' | 'up' | 'sideways' | 'down' | 'FBS>NFL' | 'FCS>NFL' | 'SmallCollege>NFL' | 'newTeam';
const STEP_SIZE: Record<Step, number> = {
  'HS>FBS': 1,
  'HS>FCS': 0.8,
  'HS>SmallCollege': 0.55,
  up: 0.55,
  sideways: 0.25,
  down: 0.1,
  'FBS>NFL': 1,
  'FCS>NFL': 1.2,
  'SmallCollege>NFL': 1.45,
  newTeam: 0.2,
};
const STEP_NAME: Record<Step, string> = {
  'HS>FBS': 'the jump from high school to FBS',
  'HS>FCS': 'the jump from high school to FCS',
  'HS>SmallCollege': 'the step from high school to D2',
  up: 'a move up a level',
  sideways: 'a new school',
  down: 'a move down a level',
  'FBS>NFL': 'the jump to the NFL',
  'FCS>NFL': 'the big jump from FCS to the NFL',
  'SmallCollege>NFL': 'the huge jump from D2 to the NFL',
  newTeam: 'a new team and scheme',
};
/** How long a position takes to learn at a new level. */
const POSITION_DIFFICULTY: Record<Position, number> = { QB: 1.3, OT: 1.2, IOL: 1.1, CB: 1.1, S: 1, LB: 1, DT: 1, TE: 1, EDGE: 0.9, WR: 0.9, RB: 0.65, K: 0.5, P: 0.5 };

/** Share of his ceiling a player has reached in one attribute group, weighted by what his position needs. */
function readiness(p: Player, group: 'physical' | 'technical' | 'mental'): number {
  const weights = POSITION_PROFILES[p.position].weights;
  let sum = 0;
  let w = 0;
  for (const key of ATTRIBUTE_KEYS) {
    if (ATTRIBUTE_PROFILES[key].group !== group) continue;
    const weight = (weights[key] ?? 0) + 0.2;
    sum += weight * Math.min(1, p.attributes[key] / Math.max(1, p.ceilings[key]));
    w += weight;
  }
  return w ? sum / w : 0.8;
}

export interface Adjustment {
  dip: number;
  step: Step;
  /** The biggest reasons, in plain words. */
  reasons: string[];
}

/**
 * How hard a player's first year at a new level is, 0-1. It depends on the
 * size of the step, the position, how polished his technique is, his
 * football IQ and make-up, whether his body is ready, and his fit with the
 * new scheme plus luck. Some arrive ready at once; others need a year.
 */
export function firstYearAdjustment(p: Player, step: Step, rng: Rng): Adjustment {
  const polish = readiness(p, 'technical');
  const iq = readiness(p, 'mental');
  const body = readiness(p, 'physical');
  const { workEthic, competitiveness, coachability } = p.personality;
  const makeup = (workEthic + competitiveness + coachability) / 3;
  const fit = rng.normal(0, 0.12);
  const luck = rng.normal(0, 0.08);
  const terms: [string, number][] = [
    [STEP_NAME[step], 0.42 * STEP_SIZE[step] * POSITION_DIFFICULTY[p.position]],
    [polish < 0.8 ? `raw technique (${Math.round(polish * 100)}% of his ceiling)` : `polished technique (${Math.round(polish * 100)}% of his ceiling)`, 0.9 * (0.8 - polish)],
    [iq < 0.75 ? 'still learning to read the game' : 'a quick football mind', 0.6 * (0.75 - iq)],
    [body < 0.85 ? 'a body not yet ready' : 'a body ready for the level', 0.6 * (0.85 - body)],
    [makeup >= 10 ? 'strong make-up' : 'weak make-up', -0.025 * (makeup - 10)],
    [fit < 0 ? 'a good scheme fit' : 'a poor scheme fit', fit],
  ];
  const dip = clamp(terms.reduce((s, [, v]) => s + v, 0) + luck, 0, 1);
  if (POSITION_DIFFICULTY[p.position] >= 1.2 && STEP_SIZE[step] >= 0.8) terms[0]![0] += ` at a position that takes time (${p.position})`;
  const reasons = terms
    .filter(([, v]) => Math.abs(v) >= 0.04)
    .sort((a, b) => Math.abs(b[1]) - Math.abs(a[1]))
    .slice(0, 3)
    .map(([t, v]) => `${t} (${v > 0 ? 'harder' : 'easier'})`);
  return { dip, step, reasons };
}

function noteDip(r: CareerRecord, adj: Adjustment, where: string): void {
  if (adj.dip >= 0.55) r.notes.push(`A hard first year in ${where} (dip ${adj.dip.toFixed(2)}): ${adj.reasons.join(', ')}.`);
  else if (adj.dip <= 0.12) r.notes.push(`Ready at once in ${where} (dip ${adj.dip.toFixed(2)}): ${adj.reasons.join(', ')}.`);
}

// --- Declaring early -----------------------------------------------------------

/**
 * Whether an underclassman leaves for the draft. He weighs where he projects
 * against how urgently he needs to go (little NIL money at a small school, a
 * running back's short shelf life, his age, injuries, a coaching change, a
 * lost starting job, fear of his stock falling) and what keeps him in school
 * (big NIL money, room to raise his stock, a bad year to make up for, a bad
 * franchise likely to pick him).
 */
function decideToDeclare(
  r: CareerRecord,
  g: ReturnType<typeof draftScore>,
  projectedPick: number,
  draftSlots: number,
  order: Team[],
  ctx: { coachingChanged: boolean; lastRole: PlayingTime },
  rng: Rng,
): boolean {
  if (isSpecialist(r.player)) return false;
  const p = r.player;
  const round = projectedPick > draftSlots ? 8 : draftRound(projectedPick, draftSlots);
  const base = [0, 3, 1.6, 0.6, -0.2, -0.7, -1.2, -1.6, -3][round]!;
  const go: [string, number][] = [];
  const stay: [string, number][] = [];
  if (r.college === 'FCS') go.push(['little NIL money at FCS', 0.5]);
  if (r.college === 'SmallCollege') go.push(['almost no NIL money at D2', 0.9]);
  if (p.position === 'RB') go.push(["a running back's short shelf life", 0.7]);
  if (p.age >= 23) go.push([`his age (${p.age})`, 0.7]);
  const college = p.history.filter((s) => s.level !== 'HS' && s.level !== 'NFL');
  if (college.some((s) => s.events.includes('majorInjury'))) go.push(['a major injury in college: get paid before the next one', 0.6]);
  if (ctx.coachingChanged) go.push(['a coaching change at his school', 0.5]);
  const before = college[college.length - 2];
  if (before?.playingTime === 'starter' && ctx.lastRole !== 'starter') go.push(['losing his starting job', 0.6]);
  const headroom = g.potentialSeen - g.overallSeen;
  if (round >= 2 && round <= 5 && headroom < 3) go.push(['fear his stock had peaked', 0.5]);
  if (r.college === 'FBS' && ctx.lastRole === 'starter') stay.push([`NIL money at ${r.prestige! >= 0.66 ? 'a big program' : 'his school'}`, -1.1 * (r.prestige ?? 0) * (round === 1 ? 0.3 : 1)]);
  if (headroom > 3) stay.push(['room to raise his stock', -Math.min(1.2, 0.12 * (headroom - 3))]);
  const last = college[college.length - 1]!;
  if (last.performance !== null && last.performance < last.overallStart - 3) stay.push(['a bad final year to make up for', -0.5]);
  const likelyTeam = projectedPick <= draftSlots ? order[(projectedPick - 1) % order.length] : undefined;
  if (likelyTeam && round <= 2 && organisation(likelyTeam) <= 8) stay.push([`the badly run ${likelyTeam.name} likely to pick him`, -0.6]);
  const total = base + [...go, ...stay].reduce((s, [, v]) => s + v, 0) + rng.normal(0, 0.5);
  const declare = total > 0;
  const projected = round === 8 ? 'to go undrafted' : `round ${round}`;
  const reasons = (xs: [string, number][]) => xs.filter(([, v]) => Math.abs(v) >= 0.2).map(([t]) => t);
  const pushes = reasons(go);
  const pulls = reasons(stay);
  if (declare) {
    r.notes.push(
      `Declared early at ${p.age - 1}, projected ${projected}${pushes.length ? `, pushed by ${pushes.join(', ')}` : ''}${pulls.length ? `, despite ${pulls.join(', ')}` : ''}.`,
    );
  } else if (round <= 3) {
    r.notes.push(`Stayed in school at ${p.age - 1} though projected ${projected}${pulls.length ? `, for ${pulls.join(', ')}` : ''}.`);
  }
  return declare;
}

// --- The draft -----------------------------------------------------------------

/** Draft order: the weakest teams pick first. */
function draftOrder(teams: Team[], rng: Rng): Team[] {
  return teams
    .map((t) => ({ t, s: teamStrength(t) + rng.normal(0, 0.6) }))
    .sort((a, b) => a.s - b.s)
    .map(({ t }) => t);
}

/** How much a team needs a position, in grade points. */
function need(team: Team, pos: Position): number {
  const w = weakestStarter(team, pos);
  if (!w) return 0;
  if (pos === 'QB') {
    const best = [...team.depth.QB].sort((a, b) => b.rating - a.rating)[0]!;
    // A team with a young franchise quarterback doesn't spend a high pick on another.
    if (best.rating >= 82 && best.age <= 30) return -8;
  }
  return clamp((78 - w.rating) / 2.5, -1.5, 3) + (w.age >= 31 ? 1 : 0);
}

interface DraftState {
  record: CareerRecord;
  team: Team;
  member: Member;
  round: number;
  contractYears: number;
}

function runDraft(entrants: CareerRecord[], teams: Team[], draftSlots: number, rng: Rng): DraftState[] {
  // Each team picks from its own board. A team's board is the consensus grade
  // seen through its front office (good GMs see clearly, bad ones reach),
  // plus what the position is worth and what the team needs.
  const order = draftOrder(teams, rng);
  const strengthRank = new Map([...teams].sort((a, b) => teamStrength(b) - teamStrength(a)).map((t, i) => [t, i]));
  // Boards rank each prospect against others at his position first, since
  // positions mature at different ages (a 21-year-old quarterback rates lower
  // than a back of the same promise); position value and need come on top.
  const field = entrants.filter((r) => !isSpecialist(r.player));
  const align = positionAligner(field.map((r) => [r.player.position, r.draftGrade!] as [Position, number]));
  for (const r of field) r.draftGrade = align(r.player.position, r.draftGrade!);
  const pool = field.sort((a, b) => b.draftGrade! - a.draftGrade!);
  const kickers = entrants.filter((r) => isSpecialist(r.player)).sort((a, b) => b.draftGrade! - a.draftGrade!);
  const signed: DraftState[] = [];
  const boards = new Map<Team, Map<CareerRecord, number>>();
  const grade = (team: Team, r: CareerRecord) => {
    let board = boards.get(team);
    if (!board) boards.set(team, (board = new Map()));
    let offset = board.get(r);
    if (offset === undefined) board.set(r, (offset = rng.normal(0, draftBoardNoise(team.draftSkill))));
    return offset;
  };
  const join = (r: CareerRecord, team: Team, round: number, years: number) => {
    const member: Member = { rating: overall(r.player), age: r.player.age, id: r.player.id };
    team.depth[r.player.position].push(member);
    r.team = team;
    r.proTeams.push(team.name);
    signed.push({ record: r, team, member, round, contractYears: years });
  };
  for (let pick = 1; pick <= draftSlots && (pool.length > 0 || kickers.length > 0); pick++) {
    const team = order[(pick - 1) % order.length]!;
    const round = draftRound(pick, draftSlots);
    // Kickers and punters have their own board: a team with a weak or ageing
    // one takes the best specialist left on day 3, now and then.
    if ((round >= 5 || (round === 4 && rng.chance(0.3))) && kickers.length) {
      const k = kickers[0]!;
      const incumbent = weakestStarter(team, k.player.position)!;
      const wants = incumbent.rating < 78 || incumbent.age >= 35;
      if (wants && k.draftGrade! + grade(team, k) >= incumbent.rating + 1 && rng.chance(0.15)) {
        kickers.shift();
        k.draftPick = pick;
        k.notes.push(`The ${team.name} needed a ${k.player.position === 'K' ? 'kicker' : 'punter'} (theirs was rated ${Math.round(incumbent.rating)}${incumbent.age >= 35 ? ` and ${incumbent.age}` : ''}) and took him with pick ${pick} off the specialist board.`);
        join(k, team, round, 4);
        continue;
      }
    }
    if (!pool.length) continue;
    let bestIndex = -1;
    let bestValue = -Infinity;
    let bestParts = { need: 0, pos: 0, reach: 0 };
    const rf = round <= 2 ? 1 : round <= 4 ? 0.6 : 0.3;
    for (let i = 0; i < Math.min(60, pool.length); i++) {
      const r = pool[i]!;
      if (r.college === 'SmallCollege' && round < PIPELINE.smallCollegeEarliestRound) continue;
      const pos = r.player.position;
      const n = need(team, pos) * (round <= 2 ? 0.5 : 1);
      const pv = PIPELINE.positionValue[pos] * rf;
      // A strong team with one hole reaches for a rare talent who fills it.
      const reach = strengthRank.get(team)! < 10 && r.draftGrade! >= 82 && n >= 1 ? 1.5 : 0;
      const value = r.draftGrade! + grade(team, r) + pv + (pos === 'QB' && round > 3 ? Math.max(n, -1) : n) + reach;
      if (value > bestValue) {
        bestValue = value;
        bestIndex = i;
        bestParts = { need: n, pos: pv, reach };
      }
    }
    if (bestIndex < 0) continue;
    const [chosen] = pool.splice(bestIndex, 1);
    const r = chosen!;
    r.draftPick = pick;
    const consensus = entrants.filter((e) => !isSpecialist(e.player) && e.draftGrade! > r.draftGrade!).length + 1;
    const parts: string[] = [];
    if (bestParts.need >= 1) parts.push(`they needed a ${r.player.position}`);
    if (bestParts.need <= -3) parts.push(`even with a starting ${r.player.position} in place`);
    if (bestParts.pos >= 1) parts.push(`${r.player.position}s are valued highly`);
    if (bestParts.pos <= -0.5) parts.push(`${r.player.position}s are valued less`);
    if (bestParts.reach) parts.push('a strong team reaching for a rare talent at its one weak spot');
    r.notes.push(
      `Drafted ${pick}${ordinal(pick)} (round ${round}) by the ${team.name} (GM ${team.draftSkill}) as the ${consensus}${ordinal(consensus)} prospect on the consensus board${parts.length ? `: ${parts.join(', ')}` : ''}.`,
    );
    join(r, team, round, 4);
  }
  // Undrafted free agents sign where they see the clearest path to a roster spot.
  const udfaSlots = Math.round(draftSlots * PIPELINE.udfaRatio);
  let signedUdfa = 0;
  for (const r of [...pool, ...kickers].sort((a, b) => b.draftGrade! - a.draftGrade!)) {
    if (signedUdfa >= udfaSlots) break;
    if (overall(r.player) < 48 || r.draftPick) continue;
    const best = bestOpening(r, teams, rng);
    if (isSpecialist(r.player) && best.slot > 0) continue;
    r.udfa = true;
    signedUdfa++;
    r.notes.push(`Undrafted, he signed with the ${best.team.name}, where he saw the clearest path to a roster spot.`);
    join(r, best.team, 8, 3);
  }
  return signed;
}

/** The team where a player would rank highest on the depth chart, with some noise for interest and money. */
function bestOpening(r: CareerRecord, teams: Team[], rng: Rng, exclude?: Team): { team: Team; slot: number } {
  const ovr = overall(r.player);
  let best: { team: Team; slot: number; v: number } | null = null;
  for (const team of teams) {
    if (team === exclude) continue;
    const seen = ovr + rng.normal(0, boardNoise(team.draftSkill) * 0.5);
    const slot = team.depth[r.player.position].filter((m) => m.id !== r.player.id && m.rating > seen).length;
    const v = -slot + rng.normal(0, 0.8) + (organisation(team) - 10.5) * 0.05;
    if (!best || v > best.v) best = { team, slot, v };
  }
  return best!;
}

// --- Pro careers --------------------------------------------------------------

interface ProState extends DraftState {
  /** Null while a free agent. */
  team: Team;
  freeAgent: boolean;
  unsignedYears: number;
  firstYear: boolean;
  newTeam: boolean;
  done: boolean;
  lastPerf: number | null;
  /** A bad situation at the current team has been noted. */
  stuckNoted?: boolean;
  /** Released in this year's camp: other teams can still claim him before the season. */
  justReleased?: boolean;
}

/**
 * Every pro season of the class, year by year, inside the league: free
 * agency, camp battles for depth chart spots, development, injuries and
 * retirement. The league's background players age and change around them.
 */
function simulateLeagueYears(signed: DraftState[], teams: Team[], draftSlots: number, rng: Rng): void {
  const states: ProState[] = signed.map((s) => ({ ...s, freeAgent: false, unsignedYears: 0, firstYear: true, newTeam: true, done: false, lastPerf: null }));
  for (const s of states) s.record.runningQB = isRunningQB(s.record.player);
  for (let year = 0; year < 30 && states.some((s) => !s.done); year++) {
    if (year > 0) advanceLeague(teams, rng);
    const leagueStrength = teams.reduce((t, x) => t + teamStrength(x), 0) / teams.length;
    const live = states.filter((s) => !s.done);
    // Free agency: expired contracts and released players find teams. Own team first if it still wants him.
    for (const s of live) {
      if (!s.freeAgent && s.contractYears > 0) continue;
      const r = s.record;
      const ovr = overall(r.player);
      const own = s.freeAgent ? null : s.team;
      const ownSlot = own ? own.depth[r.player.position].filter((m) => m !== s.member && m.rating > ovr).length : Infinity;
      const market = bestOpening(r, teams, rng, own ?? undefined);
      const { starters, rotation, roster } = DEPTH_SLOTS[r.player.position];
      const pChance = (slot: number) => (slot < starters ? 0.95 : slot < starters + rotation ? 0.85 : slot < roster ? 0.55 : 0);
      // Teams keep most of their own players; a player leaves mainly for a starting job elsewhere.
      const startsElsewhere = market.slot < starters && ownSlot >= starters && rng.chance(0.7);
      if (own && ownSlot < roster && !startsElsewhere && rng.chance(Math.min(0.95, pChance(ownSlot) + 0.25))) {
        s.contractYears = contractLength(ownSlot, r.player.position, rng);
        continue;
      }
      if (rng.chance(pChance(market.slot))) {
        if (own) removeMember(s);
        const from = s.freeAgent ? 'As a free agent' : `When his deal with the ${s.team.name} ended`;
        s.team = market.team;
        s.member = { rating: ovr, age: r.player.age, id: r.player.id };
        market.team.depth[r.player.position].push(s.member);
        s.contractYears = contractLength(market.slot, r.player.position, rng);
        s.freeAgent = false;
        s.newTeam = true;
        s.stuckNoted = false;
        r.proTeams.push(market.team.name);
        const fresh = r.notes.some((n) => n.includes('stuck in a bad situation')) && organisation(market.team) >= 12 ? ' A better-run team gave him a fresh start.' : '';
        r.notes.push(`${from} at ${r.player.age}, he signed with the ${market.team.name} (organisation ${Math.round(organisation(market.team))}/20).${fresh}`);
      } else if (!s.freeAgent) {
        removeMember(s);
        s.freeAgent = true;
      }
    }
    // Camp: each team ranks everyone at a position on what it sees.
    for (const team of teams) {
      for (const pos of Object.keys(team.depth) as Position[]) {
        const members = team.depth[pos];
        const ours = live.filter((s) => !s.freeAgent && s.team === team && s.record.player.position === pos);
        if (!ours.length) continue;
        campBattle(team, pos, members, ours, draftSlots, rng);
      }
    }
    // A starter-calibre player stuck behind better players is traded to a team that will start him.
    for (const s of live) {
      if (s.freeAgent || s.member.role === 'starter' || s.member.role === 'practiceSquad') continue;
      const r = s.record;
      const pos = r.player.position;
      if (overall(r.player) < NFL_LEVELS.starter + 3) continue;
      const offer = bestOpening(r, teams, rng, s.team);
      if (offer.slot >= DEPTH_SLOTS[pos].starters || !rng.chance(0.6)) continue;
      const from = s.team.name;
      removeMember(s);
      s.team = offer.team;
      s.member = { rating: overall(r.player), age: r.player.age, id: r.player.id, role: 'starter', depth: `${pos}${offer.slot + 1}` };
      offer.team.depth[pos].push(s.member);
      s.newTeam = true;
      s.stuckNoted = false;
      r.proTeams.push(offer.team.name);
      r.notes.push(`Stuck behind better players with the ${from} at ${r.player.age} (rated ${Math.round(overall(r.player))}), he was traded to the ${offer.team.name}, who started him.`);
    }
    // Players released in camp can be claimed by a team with a need before the season.
    for (const s of live) {
      if (!s.justReleased) continue;
      s.justReleased = false;
      const r = s.record;
      const pos = r.player.position;
      const claim = bestOpening(r, teams, rng, s.team);
      const { starters, rotation, roster } = DEPTH_SLOTS[pos];
      // Waiver claims are for younger players; a released veteran has to wait for free agency.
      if (claim.slot >= roster || r.player.age >= 30 || !rng.chance(0.6)) continue;
      s.team = claim.team;
      s.member = { rating: overall(r.player), age: r.player.age, id: r.player.id };
      s.member.role = claim.slot < starters ? 'starter' : claim.slot < starters + rotation ? 'rotation' : 'backup';
      s.member.depth = `${pos}${claim.slot + 1}`;
      claim.team.depth[pos].push(s.member);
      s.freeAgent = false;
      s.newTeam = true;
      s.stuckNoted = false;
      s.contractYears = 1;
      r.proTeams.push(claim.team.name);
      r.notes.push(`The ${claim.team.name} claimed him before the season.`);
    }
    for (const s of live) {
      const r = s.record;
      const p = r.player;
      let role: PlayingTime;
      let depth: string;
      if (s.freeAgent && p.age <= 32 && rng.chance(0.15)) {
        // Injuries open spots during the season; a free agent near the bar gets a call.
        const call = bestOpening(r, teams, rng);
        if (call.slot < DEPTH_SLOTS[p.position].roster) {
          s.team = call.team;
          s.member = { rating: overall(p), age: p.age, id: p.id, role: 'backup', depth: 'signed mid-season' };
          call.team.depth[p.position].push(s.member);
          s.freeAgent = false;
          s.newTeam = true;
          s.stuckNoted = false;
          s.contractYears = 1;
          r.proTeams.push(call.team.name);
          r.notes.push(`Unsigned in camp at ${p.age}, he was signed mid-season by the ${call.team.name} after injuries there.`);
        }
      }
      if (s.freeAgent) {
        role = 'unsigned';
        depth = 'free agent';
      } else {
        role = s.member.role ?? 'backup';
        depth = s.member.depth ?? 'backup';
      }
      const team = s.freeAgent ? null : s.team;
      const coaching = team ? clamp(team.coaching + rng.normal(0, 1.5), 1, 20) : 6;
      let adjustment = 0;
      if (team && (s.firstYear || s.newTeam) && role !== 'unsigned') {
        const from = r.college ? (`${r.college}>NFL` as Step) : 'FBS>NFL';
        const adj = firstYearAdjustment(p, s.firstYear ? from : 'newTeam', rng);
        adjustment = adj.dip;
        if (s.firstYear) noteDip(r, adj, 'the NFL');
      }
      const situation = team ? situationAt(team, p, role, leagueStrength) : 0;
      if (team && situation <= -0.45 && p.age <= 25 && s.round <= 3 && (role === 'starter' || role === 'rotation') && !s.stuckNoted) {
        s.stuckNoted = true;
        r.notes.push(`At ${p.age} he was stuck in a bad situation with the ${team.name} (organisation ${Math.round(organisation(team))}/20, a weak roster around him), which dragged his play and his development.`);
      }
      const report = developSeason(p, { level: 'NFL', coaching, playingTime: role, adjustment, situation }, rng);
      const row = p.history[p.history.length - 1]!;
      row.depth = depth;
      if (team) row.situation = Math.round(situation * 100) / 100;
      if (team) row.team = team.name;
      trackPeak(r);
      addWear(r, role, 1);
      if (role === 'starter' || role === 'rotation' || role === 'backup') r.nflSeasons++;
      if (role === 'practiceSquad') r.practiceSquadSeasons++;
      if (!s.freeAgent) {
        s.member.rating = report.overallAfter;
        s.member.age = p.age;
        s.contractYears--;
        s.unsignedYears = 0;
      } else s.unsignedYears++;
      s.lastPerf = row.performance;
      s.firstYear = false;
      s.newTeam = false;
      if (retires(s, report, rng)) {
        s.done = true;
        r.proExit = s.freeAgent ? 'unsigned' : 'retired';
        if (!s.freeAgent) removeMember(s);
      }
    }
  }
  for (const s of states) if (!s.done) s.record.proExit = 'retired';
}

function removeMember(s: ProState): void {
  const list = s.team.depth[s.record.player.position];
  const i = list.indexOf(s.member);
  if (i >= 0) list.splice(i, 1);
}

function contractLength(slot: number, pos: Position, rng: Rng): number {
  const { starters } = DEPTH_SLOTS[pos];
  return slot < starters ? rng.int(2, 4) : rng.int(1, 2);
}

/**
 * Camp and preseason: everyone at a position is ranked on what the staff sees
 * (rating, last season's production, a camp that can go well or badly, read
 * through the GM's eyes). Rookie contracts buy early picks patience, and
 * teams lean young on close calls. The best make the 53, the rest of the
 * class's players go to the practice squad or free agency; background
 * players who lose out are released.
 */
function campBattle(team: Team, pos: Position, members: Member[], ours: ProState[], draftSlots: number, rng: Rng): void {
  const noise = boardNoise(team.draftSkill) * 0.5;
  // Camp brings rookies and street free agents from other classes (a 90-man
  // camp roster for 53 places): the class's players have to beat them too.
  const { starters, rotation, roster } = DEPTH_SLOTS[pos];
  // Kickers and punters last into their late thirties, so teams don't favour youth there.
  const youthBias = pos === 'K' || pos === 'P' ? 0 : 0.3;
  const talent = (teamStrength(team) - 78) / 3;
  for (let i = 0; i < Math.ceil(roster * 0.7); i++) {
    members.push({ rating: clamp(rng.normal(68.5 + 1.5 * talent, 4.5), 45, 85), age: rng.int(22, 25) });
  }
  const play = new Map<Member, number>();
  const keep = new Map<Member, number>();
  for (const m of members) {
    const s = ours.find((o) => o.member === m);
    // Last year's starter keeps the job unless someone clearly beats him; teams rarely flip quarterbacks.
    // Staffs also expect an ageing veteran to fade and lean towards the younger man.
    const incumbent = m.role === 'starter' && m.age < 32 ? (pos === 'QB' ? 3 : 1.5) : 0;
    const fading = pos === 'K' || pos === 'P' ? 0 : Math.max(0, m.age - 30) * 0.6;
    if (!s) {
      const v = m.rating + rng.normal(0, noise);
      play.set(m, v + incumbent - fading);
      keep.set(m, v + (27 - m.age) * youthBias - (m.age >= 29 && youthBias ? 1 : 0));
      continue;
    }
    const p = s.record.player;
    const ovr = overall(p);
    // A player still adjusting to the level (or a new scheme) looks it in camp.
    const camp = ovr + rng.normal(0, 2.5) + (p.personality.competitiveness - 10) * 0.15 - (s.firstYear ? 4 : s.newTeam ? 1 : 0);
    // Years of NFL film make a player easier to judge: a proven starter is never badly misread.
    const seen = 0.55 * ovr + 0.25 * (s.lastPerf ?? ovr) + 0.2 * camp + rng.normal(0, noise / (1 + 0.4 * s.record.nflSeasons));
    const protection = s.record.nflSeasons + s.record.practiceSquadSeasons < 3 && s.round <= 8 ? PIPELINE.rookieProtection[s.round]! : 0;
    // Teams give their top picks a real shot at starting.
    play.set(m, seen + incumbent - fading + (s.firstYear && s.round === 1 ? 2 : 0));
    // A veteran backup costs more than a rookie who plays as well.
    const cost = s.record.nflSeasons >= 4 && overall(p) < NFL_LEVELS.starter ? 0.5 * (s.record.nflSeasons - 3) : 0;
    keep.set(m, seen + protection + (m.role === 'starter' ? 3 : 0) + (27 - p.age) * youthBias - Math.min(3, cost));
  }
  const byKeep = [...members].sort((a, b) => keep.get(b)! - keep.get(a)!);
  const made = new Set(byKeep.slice(0, roster));
  const byPlay = byKeep.slice(0, roster).sort((a, b) => play.get(b)! - play.get(a)!);
  byPlay.forEach((m, i) => {
    m.role = i < starters ? 'starter' : i < starters + rotation ? 'rotation' : 'backup';
    m.depth = `${pos}${i + 1}`;
  });
  // Released background players leave; the class's players go to the practice squad or free agency.
  for (const m of byKeep.slice(roster)) {
    const s = ours.find((o) => o.member === m);
    const i = members.indexOf(m);
    if (!s) {
      members.splice(i, 1);
      continue;
    }
    const p = s.record.player;
    // Practice squads are for players with at most two seasons on a roster.
    const young = s.record.nflSeasons <= 2 && s.record.practiceSquadSeasons < 3;
    const close = keep.get(m)! >= keep.get(byKeep[roster - 1]!)! - 7;
    if (young && close && p.age <= 26 && rng.chance(0.75)) {
      m.role = 'practiceSquad';
      m.depth = 'practice squad';
      if (s.record.practiceSquadSeasons === 0) s.record.notes.push(`Cut from the 53 by the ${team.name} at ${p.age}, he stayed on as a practice squad player.`);
    } else {
      members.splice(i, 1);
      s.freeAgent = true;
      s.justReleased = true;
      s.contractYears = 0;
      s.record.notes.push(`Released by the ${team.name} at ${p.age} (rated ${Math.round(overall(p))}), he went into free agency.`);
    }
  }
}

/** How his surroundings help or hurt him, -1 to 1: roster around him and how well the team is run. */
function situationAt(team: Team, p: Player, role: PlayingTime, leagueStrength: number): number {
  if (role === 'practiceSquad' || role === 'unsigned') return 0;
  const cast = (teamStrength(team) - leagueStrength) / 1.8;
  const org = (organisation(team) - 11.5) / 2.5;
  // Quarterbacks and receivers depend most on the people around them.
  const reliance = p.position === 'QB' ? 1 : p.position === 'WR' || p.position === 'TE' || p.position === 'RB' ? 0.7 : 0.4;
  return clamp(reliance * (0.5 * cast + 0.5 * org), -1, 1);
}

/** Yearly wear: backs take the most hits, quarterbacks who run take more than pocket passers. */
function addWear(r: CareerRecord, role: PlayingTime, scale: number): void {
  const pos = r.player.position;
  const share = role === 'starter' ? 1 : role === 'rotation' ? 0.5 : 0;
  if (!share) return;
  const perSeason = pos === 'RB' ? 1 : pos === 'QB' ? (r.runningQB ? 0.6 : 0.15) : pos === 'K' || pos === 'P' ? 0 : 0.35;
  r.player.wear += perSeason * share * scale;
}

function isRunningQB(p: Player): boolean {
  if (p.position !== 'QB') return false;
  return (p.attributes.speed + p.attributes.acceleration + p.attributes.agility) / 3 >= 78;
}

/**
 * Retirement: each position has a typical age to walk away, moved by his
 * longevity (rare bodies like Derrick Henry), love of the game, the wear on
 * his body, and whether he is still good. A player who goes unsigned
 * usually retires within a year or two.
 */
function retires(s: ProState, report: { overallAfter: number; events: string[] }, rng: Rng): boolean {
  const r = s.record;
  const p = r.player;
  const age = p.age;
  if (s.freeAgent) {
    if (s.unsignedYears >= 2) {
      r.notes.push(`Unsigned for two seasons, he retired at ${age}.`);
      return true;
    }
    const specialist = p.position === 'K' || p.position === 'P';
    const base = specialist ? (age >= 36 ? 0.6 : 0.25) : age >= 30 ? 0.7 : age >= 27 ? 0.5 : 0.3;
    if (rng.chance(base * clamp(1.3 - p.personality.loveOfGame * 0.03, 0.6, 1.3))) {
      r.notes.push(`Unsigned for a season, he retired at ${age}.`);
      return true;
    }
    return false;
  }
  const typical =
    PIPELINE.retireAge[p.position] +
    (r.runningQB ? PIPELINE.runningQbRetireOffset : 0) +
    p.longevity * 0.8 +
    (p.personality.loveOfGame - 10) * 0.2 -
    Math.max(0, p.wear - 6) * 0.3 * clamp(1 - 0.15 * p.longevity, 0.5, 1.5);
  // Still being good keeps a player going, but only for a few years past the usual age.
  let standing = report.overallAfter >= 85 ? 0.35 : report.overallAfter >= 78 ? 0.6 : 1;
  if (age > typical + 2) standing += (1 - standing) * Math.min(1, (age - typical - 2) / 3);
  const major = report.events.includes('majorInjury') && age >= 27 ? 0.12 : 0;
  const chance = (0.008 + 0.5 / (1 + Math.exp(-(age - typical) / 1.3))) * standing + major + Math.max(0, age - typical - 4) * 0.08;
  if (rng.chance(chance)) {
    // Only name the reasons that point the way his career went: why he lasted, or why he left early.
    const why: string[] = [];
    const late = age > typical + 1;
    const early = age < typical - 1;
    if (early && r.runningQB) why.push('a quarterback who relied on his legs');
    if (early && p.wear >= 7) why.push(`a heavy workload had worn his body down (wear ${p.wear.toFixed(1)})`);
    if (early && p.longevity <= -1.5) why.push(`a body that aged early (longevity ${p.longevity.toFixed(1)})`);
    if (early && p.personality.loveOfGame <= 6) why.push(`little love of the grind (${p.personality.loveOfGame}/20)`);
    if (late && p.longevity >= 1.5) why.push(`a body that aged slowly (longevity +${p.longevity.toFixed(1)})`);
    if (late && p.personality.loveOfGame >= 15) why.push(`a deep love of the game (${p.personality.loveOfGame}/20)`);
    if (late && report.overallAfter >= 85) why.push(`still one of the best at his position (rated ${Math.round(report.overallAfter)})`);
    if (major) why.push('a major injury late in his career');
    const odds = chance >= 0.2 ? '' : `, a ${Math.max(1, Math.round(chance * 100))}% chance that year`;
    r.notes.push(`Retired at ${age}, rated ${Math.round(report.overallAfter)} (typical for his case: about ${Math.round(typical)}${odds})${why.length ? `: ${why.join(', ')}` : ''}.`);
    return true;
  }
  return false;
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

export type { Team } from './league.js';
export { PRACTICE_SQUAD };

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

/** Yearly chance of off-field or academic trouble in college. */
function troubleChance(p: Player): number {
  const { workEthic, coachability } = p.personality;
  return PIPELINE.collegeTrouble.chance * clamp(1 + (10 - workEthic) * 0.12 + (10 - coachability) * 0.06, 0.25, 3);
}

function isSpecialist(p: Player): boolean {
  return p.position === 'K' || p.position === 'P';
}

/** What a draft room thinks of a prospect: current ability plus some projection. Specialists are graded on their own board. */
function draftScore(r: CareerRecord, rng: Rng): { score: number; overallSeen: number; potentialSeen: number } {
  const rep = scoutPlayer(r.player, { accuracy: 13, level: r.college ?? 'HS' }, rng);
  const youth = clamp((23 - r.player.age) / 3, 0, 1);
  const competition = PIPELINE.competitionDiscount[r.college ?? 'HS'];
  // Off-field and academic red flags cost a player on every board.
  const { suspended, dismissed } = PIPELINE.redFlagDiscount;
  const flags = r.player.history.reduce(
    (sum, s) => sum + (s.events.includes('dismissed') ? dismissed : s.events.includes('suspended') ? suspended : 0),
    0,
  );
  const score = rep.overall + (0.25 + 0.2 * youth) * (rep.potential - rep.overall) - competition - flags;
  return { score, overallSeen: rep.overall, potentialSeen: rep.potential };
}

/** Playing time by rank within each position: a quarterback competes with quarterbacks. */
function rolesByRank(
  group: CareerRecord[],
  starterShare: number,
  rotationShare: number,
  seen: (r: CareerRecord) => number,
): Map<CareerRecord, PlayingTime> {
  const byPosition = new Map<Position, { r: CareerRecord; v: number }[]>();
  for (const r of group) {
    let g = byPosition.get(r.player.position);
    if (!g) byPosition.set(r.player.position, (g = []));
    g.push({ r, v: seen(r) });
  }
  const roles = new Map<CareerRecord, PlayingTime>();
  for (const g of byPosition.values()) {
    g.sort((a, b) => b.v - a.v);
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

function ordinal(n: number): string {
  const t = n % 100;
  if (t >= 11 && t <= 13) return 'th';
  return ['th', 'st', 'nd', 'rd'][n % 10] ?? 'th';
}

export type { Level };
