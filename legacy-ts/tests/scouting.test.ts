import { describe, expect, it } from 'vitest';
import { generateProspect, overall, potential } from '../src/player.js';
import { Rng } from '../src/rng.js';
import { scoutPlayer, toStars } from '../src/scouting.js';
import { mean } from './stats.js';

describe('scouting', () => {
  const rng = new Rng('scouting');
  const players = Array.from({ length: 2000 }, () => generateProspect(rng, { age: 17, talentZ: 2.5 }));

  const error = (accuracy: number) =>
    mean(players.map((p) => Math.abs(scoutPlayer(p, { accuracy, level: 'HS' }, rng).potential - potential(p))));

  it('gives better estimates from better scouts', () => {
    expect(error(18)).toBeLessThan(error(4) * 0.75);
  });

  it('is less sure about younger players', () => {
    const young = generateProspect(new Rng(1), { age: 15 });
    const veteran = { ...generateProspect(new Rng(1), { age: 15 }), age: 28 };
    const width = (p: typeof young) => {
      const r = scoutPlayer(p, { accuracy: 12, level: 'FBS' }, new Rng(2));
      return r.potentialHigh - r.potentialLow;
    };
    expect(width(young)).toBeGreaterThan(width(veteran));
  });

  it('reports a range that usually contains the truth', () => {
    const hits = players.filter((p) => {
      const r = scoutPlayer(p, { accuracy: 12, level: 'HS' }, rng);
      return r.potentialLow <= potential(p) && potential(p) <= r.potentialHigh;
    }).length;
    expect(hits / players.length).toBeGreaterThan(0.6);
    expect(overall(players[0]!)).toBeGreaterThan(0);
  });

  it('maps ratings to half stars', () => {
    expect(toStars(90)).toBe(5);
    expect(toStars(70)).toBe(3);
    expect(toStars(50)).toBe(1);
    expect(toStars(20)).toBe(0.5);
  });
});
