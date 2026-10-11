/**
 * Trial 6: simulates one national class and draws 20 careers at random, one
 * from each kind of outcome, so together they show the whole range. Writes
 * docs/trials/trial6-cards.json. Usage: npx tsx scripts/trial6-cards.ts [seed]
 */
import { writeFileSync } from 'node:fs';
import { draftRound, simulateCohort, type CareerRecord } from '../src/career.js';
import { overall, type Player } from '../src/player.js';
import { Rng } from '../src/rng.js';

const seed = process.argv[2] ?? 'trial-6';
const t0 = Date.now();
const { records, draftSlots } = simulateCohort({ size: 250_000, seed });
console.error(`simulated in ${((Date.now() - t0) / 1000).toFixed(0)}s`);

const nfl = (r: CareerRecord) => r.player.history.filter((h) => h.level === 'NFL');
const has = (r: CareerRecord, text: string) => r.notes.some((n) => n.includes(text));
const round = (r: CareerRecord) => (r.draftPick ? draftRound(r.draftPick, draftSlots) : null);
const BUCKETS: [string, (r: CareerRecord) => boolean][] = [
  ['Five-star, first-round pick', (r) => r.stars === 5 && round(r) === 1],
  ['Five-star who busted', (r) => r.stars === 5 && (!r.draftPick || round(r)! >= 5)],
  ['Three-star who was drafted', (r) => r.stars === 3 && !!r.draftPick],
  ['Two-star or unranked who was drafted', (r) => r.stars <= 2 && !!r.draftPick],
  ['FCS player', (r) => r.college === 'FCS' && (!!r.draftPick || r.udfa)],
  ['D2 player', (r) => r.signedWith === 'SmallCollege' && r.transfers.length === 0 && r.player.history.some((h) => h.level === 'SmallCollege' && h.playingTime === 'starter')],
  ['Moved up through the portal', (r) => r.portal.some((p) => p.outcome === 'up')],
  ['Moved across or down for playing time', (r) => r.portal.some((p) => p.outcome === 'sideways' || p.outcome === 'down')],
  ['Portal found him no school', (r) => r.portal.some((p) => p.outcome === 'none') && r.signedWith === 'FBS'],
  ['Dismissed for a lesser cause', (r) => r.transfers.some((t) => t.reason === 'dismissedLesser' && t.from === t.to)],
  ['Dismissed for a serious cause', (r) => has(r, 'a serious cause') && !has(r, 'a lesser cause')],
  ['Declared early', (r) => has(r, 'Declared early')],
  ['Stayed in school with a high projection', (r) => has(r, 'Stayed in school')],
  ['Stalled by a bad team', (r) => has(r, 'stuck in a bad situation')],
  ['Undrafted who made it', (r) => r.udfa && r.nflSeasons >= 4],
  ['Practice squad and journeyman', (r) => r.practiceSquadSeasons > 0 && r.proTeams.length >= 3],
  ['Kicker or punter', (r) => (r.player.position === 'K' || r.player.position === 'P') && r.proTeams.length > 0],
  ['Long career', (r) => r.proExit === 'retired' && nfl(r).length >= 12],
  ['Major injury changed his path', (r) => r.player.history.some((h) => h.level !== 'HS' && h.events.includes('majorInjury')) && r.proTeams.length > 0],
  ['Anyone from the class', () => true],
];
const pick = new Rng(`${seed}-picks`);
const used = new Set<CareerRecord>();
const cards = BUCKETS.map(([label, test], i) => {
  const pool = records.filter((r) => test(r) && !used.has(r));
  console.error(`${label}: ${pool.length}`);
  if (!pool.length) throw new Error(`no career for "${label}"`);
  const r = pick.pick(pool);
  used.add(r);
  return card(r, label, i + 1, pool.length);
});
writeFileSync('docs/trials/trial6-cards.json', JSON.stringify(cards, null, 1));
console.error('wrote docs/trials/trial6-cards.json');

function card(r: CareerRecord, label: string, n: number, poolSize: number) {
  const p = r.player;
  const seasons = p.history.map((h) => ({
    age: h.age,
    level: h.level,
    role: h.playingTime,
    start: Math.round(h.overallStart),
    ovr: Math.round(h.overall),
    pot: Math.round(h.potential),
    perf: h.performance === null ? null : Math.round(h.performance),
    levelRating: h.levelRating,
    levelRatingEnd: h.levelRatingEnd,
    coaching: Math.round(h.coaching * 10) / 10,
    events: h.events,
    gamesMissed: h.gamesMissed ?? 0,
    dip: h.adjustment ?? null,
    team: h.team ?? null,
    depth: h.depth ?? null,
    situation: h.situation ?? null,
  }));
  return {
    n,
    label,
    title: label,
    how: 'random',
    poolSize,
    id: p.id,
    position: p.position,
    runningQB: r.runningQB,
    stars: r.stars,
    seniorFilmRank: r.seniorFilmRank,
    signedWith: r.signedWith,
    college: r.college,
    transfers: r.transfers,
    portal: r.portal,
    leftFootball: r.leftFootball,
    draftGrade: r.draftGrade === null ? null : Math.round(r.draftGrade * 10) / 10,
    draftPick: r.draftPick,
    round: round(r),
    udfa: r.udfa,
    teamDraftSkill: r.team?.draftSkill ?? null,
    proTeams: r.proTeams,
    proExit: r.proExit,
    nflSeasons: r.nflSeasons,
    practiceSquadSeasons: r.practiceSquadSeasons,
    potentialAtExit: Math.round(r.potentialAtExit),
    personality: p.personality,
    bloom: Math.round(p.bloom * 10) / 10,
    longevity: Math.round(p.longevity * 10) / 10,
    injuryProneness: p.injuryProneness,
    wear: Math.round(p.wear * 10) / 10,
    start: { overall: Math.round(r.initialOverall), potential: Math.round(r.initialPotential) },
    end: { overall: Math.round(overall(p)) },
    seasons,
    notes: r.notes,
    story: '',
    why: [] as string[],
    flags: [],
  };
}
void ({} as Player);
