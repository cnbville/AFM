import { simulateCohort, PIPELINE } from '/home/claude/AFM/src/career.ts';
const opts: Record<string, [number, number, number]> = {
  now: [Infinity, Infinity, Infinity], A: [5000, 12000, 10000], B: [3500, 8000, 6500], C: [2700, 6500, 4500],
};
const [opt, seed] = process.argv.slice(2);
const [m3, m2, fbs] = opts[opt]!;
const P = PIPELINE as any; P.starFilmRank.maxFor3 = m3; P.starFilmRank.maxFor2 = m2; P.fbsOfferMaxFilmRank = fbs;
const { records } = simulateCohort({ size: 250_000, seed });
const q = (xs: number[], p: number) => { const s = [...xs].sort((a, b) => a - b); return s.length ? s[Math.min(s.length - 1, Math.floor(p * s.length))] : NaN; };
const ranks = (f: (r: any) => boolean) => records.filter(f).map((r) => r.seniorFilmRank);
const three = ranks((r) => r.stars === 3), two = ranks((r) => r.stars === 2);
const fbsS = records.filter((r) => r.signedWith === 'FBS'), fbsR = fbsS.map((r) => r.seniorFilmRank);
const drafted = records.filter((r) => r.draftPick != null);
const lowStar = drafted.filter((r) => r.stars <= 2);
const deep = drafted.filter((r) => r.seniorFilmRank > 10000);
const by = (xs: any[], k: string) => xs.reduce((m, r) => ((m[r[k] ?? 'none'] = (m[r[k] ?? 'none'] ?? 0) + 1), m), {} as any);
const fbsDrafted = drafted.filter((r) => r.signedWith === 'FBS');
console.log(JSON.stringify({ opt, seed,
  three: { p50: q(three, .5), p90: q(three, .9), max: q(three, 1), outside5700: three.filter((x) => x > 5700).length },
  two: { p50: q(two, .5), p90: q(two, .9), max: q(two, 1) },
  fbs: { n: fbsS.length, p50: q(fbsR, .5), p90: q(fbsR, .9), max: q(fbsR, 1), outside10k: fbsR.filter((x) => x > 10000).length, outside13800: fbsR.filter((x) => x > 13800).length, under3star: fbsS.filter((r) => r.stars < 3).length },
  draft: { n: drafted.length, lowStarShare: +(lowStar.length / drafted.length).toFixed(3), deepFilm: deep.length, deepSignedWith: by(deep, 'signedWith'), deepFinished: by(deep, 'college'),
    fbsSignedShare: +(fbsDrafted.length / drafted.length).toFixed(3), signedWith: by(drafted, 'signedWith'), r1LowStar: drafted.filter((r) => r.draftPick! <= 32 && r.stars <= 2).length },
}));
