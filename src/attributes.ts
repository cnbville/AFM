/**
 * Player attributes on a 1-99 scale. Each attribute has a development profile
 * describing how mature it is in a 14-year-old, how quickly it grows, what
 * drives that growth, and when age starts to take it away.
 */

export const ATTRIBUTE_KEYS = [
  // physical
  'speed',
  'acceleration',
  'agility',
  'strength',
  'jumping',
  'stamina',
  // technical
  'throwPower',
  'throwAccuracy',
  'carrying',
  'catching',
  'routeRunning',
  'elusiveness',
  'runBlocking',
  'passBlocking',
  'tackling',
  'blockShedding',
  'passRush',
  'manCoverage',
  'zoneCoverage',
  'kickPower',
  'kickAccuracy',
  // mental
  'awareness',
  'playRecognition',
  'vision',
  'poise',
] as const;

export type AttributeKey = (typeof ATTRIBUTE_KEYS)[number];
export type AttributeGroup = 'physical' | 'technical' | 'mental';
export type Attributes = Record<AttributeKey, number>;

export const MIN_RATING = 1;
export const MAX_RATING = 99;

export interface AttributeProfile {
  group: AttributeGroup;
  /** Share of the eventual ceiling a typical 14-year-old already has. */
  maturityAt14: number;
  /** Fraction of the remaining gap to the ceiling closed in a normal year at prime development age. */
  growthRate: number;
  /** Ages (inclusive) over which growth runs at full rate before tapering. */
  growthWindow: readonly [number, number];
  /** How much growth depends on coaching and training (0 = innate, 1 = fully coached). */
  coachability: number;
  /** How much growth depends on game snaps (0 = not at all, 1 = only learned by playing). */
  experience: number;
  /** Typical age decline begins, before player and position adjustments. */
  declineAge: number;
  /** Points lost per year one year past declineAge; grows linearly each year after. */
  declineRate: number;
  /** Whether a major injury can permanently cost this attribute. */
  injurySensitive: boolean;
}

const physical = (p: Omit<AttributeProfile, 'group' | 'experience'>): AttributeProfile => ({
  group: 'physical',
  experience: 0.1,
  ...p,
});
const technical = (p: Omit<AttributeProfile, 'group' | 'injurySensitive'>): AttributeProfile => ({
  group: 'technical',
  injurySensitive: false,
  ...p,
});
const mental = (p: Omit<AttributeProfile, 'group' | 'injurySensitive' | 'coachability'>): AttributeProfile => ({
  group: 'mental',
  injurySensitive: false,
  coachability: 0.4,
  ...p,
});

export const ATTRIBUTE_PROFILES: Record<AttributeKey, AttributeProfile> = {
  // Speed is largely genetic: high at 14, modest gains, first thing to go.
  speed: physical({ maturityAt14: 0.8, growthRate: 0.24, growthWindow: [14, 20], coachability: 0.25, declineAge: 27, declineRate: 0.9, injurySensitive: true }),
  acceleration: physical({ maturityAt14: 0.76, growthRate: 0.24, growthWindow: [14, 21], coachability: 0.35, declineAge: 27, declineRate: 0.8, injurySensitive: true }),
  agility: physical({ maturityAt14: 0.74, growthRate: 0.24, growthWindow: [14, 21], coachability: 0.35, declineAge: 27, declineRate: 0.8, injurySensitive: true }),
  // Strength is built in the weight room through the college years.
  strength: physical({ maturityAt14: 0.48, growthRate: 0.2, growthWindow: [15, 24], coachability: 0.8, declineAge: 31, declineRate: 0.6, injurySensitive: false }),
  jumping: physical({ maturityAt14: 0.74, growthRate: 0.22, growthWindow: [14, 21], coachability: 0.3, declineAge: 28, declineRate: 0.8, injurySensitive: true }),
  stamina: physical({ maturityAt14: 0.62, growthRate: 0.22, growthWindow: [14, 24], coachability: 0.7, declineAge: 30, declineRate: 0.6, injurySensitive: false }),

  throwPower: technical({ maturityAt14: 0.58, growthRate: 0.2, growthWindow: [14, 22], coachability: 0.5, experience: 0.2, declineAge: 34, declineRate: 0.6 }),
  throwAccuracy: technical({ maturityAt14: 0.48, growthRate: 0.15, growthWindow: [15, 27], coachability: 0.8, experience: 0.6, declineAge: 35, declineRate: 0.6 }),
  carrying: technical({ maturityAt14: 0.58, growthRate: 0.18, growthWindow: [14, 24], coachability: 0.7, experience: 0.4, declineAge: 32, declineRate: 0.5 }),
  catching: technical({ maturityAt14: 0.56, growthRate: 0.17, growthWindow: [14, 24], coachability: 0.6, experience: 0.4, declineAge: 32, declineRate: 0.5 }),
  routeRunning: technical({ maturityAt14: 0.46, growthRate: 0.15, growthWindow: [15, 26], coachability: 0.8, experience: 0.6, declineAge: 32, declineRate: 0.5 }),
  elusiveness: technical({ maturityAt14: 0.64, growthRate: 0.18, growthWindow: [14, 22], coachability: 0.3, experience: 0.4, declineAge: 27, declineRate: 0.8 }),
  runBlocking: technical({ maturityAt14: 0.46, growthRate: 0.16, growthWindow: [15, 26], coachability: 0.8, experience: 0.6, declineAge: 32, declineRate: 0.5 }),
  passBlocking: technical({ maturityAt14: 0.42, growthRate: 0.15, growthWindow: [15, 27], coachability: 0.8, experience: 0.7, declineAge: 32, declineRate: 0.5 }),
  tackling: technical({ maturityAt14: 0.52, growthRate: 0.17, growthWindow: [14, 24], coachability: 0.7, experience: 0.5, declineAge: 31, declineRate: 0.6 }),
  blockShedding: technical({ maturityAt14: 0.46, growthRate: 0.16, growthWindow: [15, 26], coachability: 0.7, experience: 0.6, declineAge: 31, declineRate: 0.6 }),
  passRush: technical({ maturityAt14: 0.46, growthRate: 0.16, growthWindow: [15, 26], coachability: 0.7, experience: 0.6, declineAge: 30, declineRate: 0.7 }),
  manCoverage: technical({ maturityAt14: 0.48, growthRate: 0.16, growthWindow: [15, 25], coachability: 0.7, experience: 0.6, declineAge: 29, declineRate: 0.8 }),
  zoneCoverage: technical({ maturityAt14: 0.46, growthRate: 0.15, growthWindow: [15, 27], coachability: 0.7, experience: 0.7, declineAge: 32, declineRate: 0.5 }),
  kickPower: technical({ maturityAt14: 0.62, growthRate: 0.2, growthWindow: [14, 23], coachability: 0.5, experience: 0.1, declineAge: 36, declineRate: 0.5 }),
  kickAccuracy: technical({ maturityAt14: 0.52, growthRate: 0.16, growthWindow: [14, 28], coachability: 0.8, experience: 0.4, declineAge: 37, declineRate: 0.4 }),

  // Mental attributes are learned by playing and keep improving into the 30s.
  awareness: mental({ maturityAt14: 0.4, growthRate: 0.13, growthWindow: [16, 30], experience: 0.8, declineAge: 35, declineRate: 0.5 }),
  playRecognition: mental({ maturityAt14: 0.4, growthRate: 0.13, growthWindow: [16, 30], experience: 0.8, declineAge: 34, declineRate: 0.5 }),
  vision: mental({ maturityAt14: 0.55, growthRate: 0.14, growthWindow: [14, 26], experience: 0.6, declineAge: 31, declineRate: 0.6 }),
  poise: mental({ maturityAt14: 0.42, growthRate: 0.12, growthWindow: [16, 31], experience: 0.8, declineAge: 36, declineRate: 0.4 }),
};

export function emptyAttributes(value = MIN_RATING): Attributes {
  const out = {} as Attributes;
  for (const k of ATTRIBUTE_KEYS) out[k] = value;
  return out;
}
