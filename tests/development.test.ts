import { describe, expect, it } from 'vitest';
import { ATTRIBUTE_KEYS, MAX_RATING, MIN_RATING } from '../src/attributes.js';
import { MAX_POTENTIAL_SHIFT, developSeason, type SeasonContext } from '../src/development.js';
import { generateProspect, overall, potential, type Level, type Player } from '../src/player.js';
import { Rng } from '../src/rng.js';
import { mean } from './stats.js';

/** A typical path for a good prospect: HS to 17, FBS to 21, NFL after. */
function contextFor(age: number, overrides: Partial<SeasonContext> = {}): SeasonContext {
  const level: Level = age <= 17 ? 'HS' : age <= 21 ? 'FBS' : 'NFL';
  const coaching = level === 'HS' ? 9 : level === 'FBS' ? 13 : 14;
  return { level, coaching, playingTime: age <= 15 ? 'rotation' : 'starter', ...overrides };
}

function career(player: Player, rng: Rng, toAge: number, overrides: Partial<SeasonContext> = {}) {
  while (player.age < toAge) developSeason(player, contextFor(player.age, overrides), rng);
  return player;
}

function prospects(n: number, seed: string, talentZ = 3) {
  const rng = new Rng(seed);
  return Array.from({ length: n }, () => generateProspect(rng, { talentZ: talentZ + rng.normal(0, 0.3) }));
}

describe('development curves', () => {
  it('peaks speed in the mid 20s and awareness around 30', () => {
    const rng = new Rng('curves');
    const speedPeak: number[] = [];
    const awarenessPeak: number[] = [];
    const speedLossBy33: number[] = [];
    for (const p of prospects(400, 'curves', 3)) {
      let best = { speed: [0, 0], awareness: [0, 0] };
      while (p.age < 36) {
        developSeason(p, contextFor(p.age), rng);
        if (p.attributes.speed > best.speed[0]!) best.speed = [p.attributes.speed, p.age];
        if (p.attributes.awareness > best.awareness[0]!) best.awareness = [p.attributes.awareness, p.age];
        if (p.age === 33) speedLossBy33.push(best.speed[0]! - p.attributes.speed);
      }
      speedPeak.push(best.speed[1]!);
      awarenessPeak.push(best.awareness[1]!);
    }
    expect(mean(speedPeak)).toBeGreaterThan(21);
    expect(mean(speedPeak)).toBeLessThan(27);
    expect(mean(awarenessPeak)).toBeGreaterThan(28);
    expect(mean(awarenessPeak)).toBeGreaterThan(mean(speedPeak) + 4);
    expect(mean(speedLossBy33)).toBeGreaterThan(5);
  });

  it('keeps kickers and quarterbacks going longer than running backs', () => {
    const peakAge = (position: Player['position']) => {
      const rng = new Rng(`peak-${position}`);
      return mean(
        Array.from({ length: 300 }, () => {
          const p = generateProspect(rng, { position, talentZ: 3 });
          let best = [0, 0];
          while (p.age < 40) {
            developSeason(p, contextFor(p.age), rng);
            if (overall(p) > best[0]!) best = [overall(p), p.age];
          }
          return best[1]!;
        }),
      );
    };
    const rb = peakAge('RB');
    expect(peakAge('QB')).toBeGreaterThan(rb + 3);
    expect(peakAge('K')).toBeGreaterThan(rb + 5);
  });

  it('rewards work ethic, coaching and playing time', () => {
    const run = (mutate: (p: Player) => void, overrides: Partial<SeasonContext> = {}) => {
      const rng = new Rng('levers');
      return mean(
        prospects(400, 'levers', 2.5).map((p) => {
          mutate(p);
          return overall(career(p, rng, 22, overrides));
        }),
      );
    };
    const baseline = run(() => {});
    expect(run((p) => (p.personality.workEthic = 18))).toBeGreaterThan(baseline + 1.5);
    expect(run((p) => (p.personality.workEthic = 3))).toBeLessThan(baseline - 1.5);
    expect(run(() => {}, { coaching: 18 })).toBeGreaterThan(run(() => {}, { coaching: 5 }) + 2);
    expect(run(() => {}, { playingTime: 'starter' })).toBeGreaterThan(run(() => {}, { playingTime: 'backup' }) + 1.5);
  });

  it('lets late bloomers catch up with early bloomers', () => {
    const rng = new Rng('bloom');
    const early: { at16: number; at24: number }[] = [];
    const late: { at16: number; at24: number }[] = [];
    for (const p of prospects(1500, 'bloom', 2.5)) {
      const bucket = p.bloom < -0.8 ? early : p.bloom > 0.8 ? late : null;
      career(p, rng, 16);
      const at16 = overall(p);
      career(p, rng, 24);
      bucket?.push({ at16, at24: overall(p) });
    }
    const gap16 = mean(early.map((x) => x.at16)) - mean(late.map((x) => x.at16));
    const gap24 = mean(early.map((x) => x.at24)) - mean(late.map((x) => x.at24));
    expect(gap16).toBeGreaterThan(2);
    expect(Math.abs(gap24)).toBeLessThan(gap16 / 2);
  });

  it('takes explosiveness away with a major injury', () => {
    const rng = new Rng('injury');
    const hurt: number[] = [];
    const healthy: number[] = [];
    for (const p of prospects(3000, 'injury', 3)) {
      career(p, rng, 23);
      const before = p.attributes.speed;
      const report = developSeason(p, contextFor(23), rng);
      (report.events.includes('majorInjury') ? hurt : healthy).push(p.attributes.speed - before);
    }
    expect(hurt.length).toBeGreaterThan(20);
    expect(mean(hurt)).toBeLessThan(mean(healthy) - 2);
  });
});

describe('dynamic potential', () => {
  it('never moves more than one band per checkpoint while young', () => {
    const rng = new Rng('bands');
    let checked = 0;
    for (const p of prospects(1500, 'bands', 1.5)) {
      let prev = potential(p);
      while (p.age < 22) {
        const r = developSeason(p, contextFor(p.age), rng);
        if (!r.events.includes('majorInjury')) {
          // Small allowance: a ceiling is nudged up when a player outgrows it.
          expect(Math.abs(r.potentialAfter - prev)).toBeLessThanOrEqual(MAX_POTENTIAL_SHIFT + 0.6);
          checked++;
        }
        prev = r.potentialAfter;
      }
    }
    expect(checked).toBeGreaterThan(8000);
  });

  it('moves potential both ways and mostly before 24', () => {
    const rng = new Rng('drift');
    const teen: number[] = [];
    const late: number[] = [];
    for (const p of prospects(1500, 'drift', 2)) {
      const p14 = potential(p);
      career(p, rng, 18);
      teen.push(potential(p) - p14);
      const p24 = (career(p, rng, 24), potential(p));
      career(p, rng, 27);
      late.push(potential(p) - p24);
    }
    expect(teen.filter((d) => d >= 5).length / teen.length).toBeGreaterThan(0.1);
    expect(teen.filter((d) => d <= -5).length / teen.length).toBeGreaterThan(0.05);
    // From 24 to 27 the ceiling barely rises.
    expect(late.filter((d) => d >= 5).length / late.length).toBeLessThan(0.02);
  });
});

describe('invariants', () => {
  it('keeps every rating within 1-99 and current at or below its ceiling', () => {
    const rng = new Rng('bounds');
    for (const p of prospects(300, 'bounds', 4.5).concat(prospects(300, 'bounds-low', -2))) {
      while (p.age < 40) {
        developSeason(p, contextFor(p.age), rng);
        for (const k of ATTRIBUTE_KEYS) {
          const ok = p.attributes[k] >= MIN_RATING && p.ceilings[k] <= MAX_RATING && p.attributes[k] <= p.ceilings[k];
          if (!ok) expect.fail(`${p.id} ${k} at age ${p.age}: ${p.attributes[k]} / ${p.ceilings[k]}`);
        }
      }
    }
  });

  it('is deterministic for a seed', () => {
    const a = career(generateProspect(new Rng(7), { id: 'x' }), new Rng(8), 30);
    const b = career(generateProspect(new Rng(7), { id: 'x' }), new Rng(8), 30);
    expect(a).toEqual(b);
  });
});
