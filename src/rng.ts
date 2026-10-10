/**
 * Seeded pseudo-random generator (sfc32). Every simulation takes an Rng so
 * that careers, saves and tests are fully reproducible from a seed.
 */
export class Rng {
  private a: number;
  private b: number;
  private c: number;
  private d: number;
  private spareNormal: number | null = null;

  constructor(seed: number | string) {
    let h = typeof seed === 'number' ? seed >>> 0 : hashString(seed);
    // splitmix32 to spread the seed over the four state words
    const next = () => {
      h = (h + 0x9e3779b9) | 0;
      let z = h;
      z = Math.imul(z ^ (z >>> 16), 0x85ebca6b);
      z = Math.imul(z ^ (z >>> 13), 0xc2b2ae35);
      return (z ^ (z >>> 16)) >>> 0;
    };
    this.a = next();
    this.b = next();
    this.c = next();
    this.d = next();
    for (let i = 0; i < 12; i++) this.next();
  }

  /** Uniform float in [0, 1). */
  next(): number {
    const t = (((this.a + this.b) | 0) + this.d) | 0;
    this.d = (this.d + 1) | 0;
    this.a = this.b ^ (this.b >>> 9);
    this.b = (this.c + (this.c << 3)) | 0;
    this.c = (this.c << 21) | (this.c >>> 11);
    this.c = (this.c + t) | 0;
    return (t >>> 0) / 4294967296;
  }

  range(min: number, max: number): number {
    return min + (max - min) * this.next();
  }

  /** Integer in [min, max] inclusive. */
  int(min: number, max: number): number {
    return min + Math.floor(this.next() * (max - min + 1));
  }

  chance(p: number): boolean {
    return this.next() < p;
  }

  normal(mean = 0, sd = 1): number {
    if (this.spareNormal !== null) {
      const z = this.spareNormal;
      this.spareNormal = null;
      return mean + sd * z;
    }
    let u = 0;
    while (u === 0) u = this.next();
    const v = this.next();
    const r = Math.sqrt(-2 * Math.log(u));
    this.spareNormal = r * Math.sin(2 * Math.PI * v);
    return mean + sd * r * Math.cos(2 * Math.PI * v);
  }

  /** Multiplicative noise centred on 1 (median 1, always positive). */
  logNormal(sd: number): number {
    return Math.exp(this.normal(0, sd));
  }

  pick<T>(items: readonly T[]): T {
    return items[Math.floor(this.next() * items.length)] as T;
  }

  weighted<T>(entries: readonly (readonly [T, number])[]): T {
    let total = 0;
    for (const [, w] of entries) total += w;
    let roll = this.next() * total;
    for (const [item, w] of entries) {
      roll -= w;
      if (roll < 0) return item;
    }
    return entries[entries.length - 1]![0];
  }

  /** Derive an independent child generator (e.g. one per player). */
  fork(): Rng {
    return new Rng(Math.floor(this.next() * 4294967296));
  }
}

function hashString(s: string): number {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

export function clamp(x: number, min: number, max: number): number {
  return x < min ? min : x > max ? max : x;
}
