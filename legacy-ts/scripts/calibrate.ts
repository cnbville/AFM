/**
 * Prints the headline statistics of a simulated national class so the model
 * can be tuned against real-world rates. Usage: npm run calibrate [-- size seed]
 */
import { simulateCohort } from '../src/career.js';
import { summarize } from '../tests/stats.js';

const size = Number(process.argv[2] ?? 250_000);
const seed = process.argv[3] ?? 'calibrate';
const t0 = Date.now();
const result = simulateCohort({ size, seed });
console.log(`simulated ${size} players in ${((Date.now() - t0) / 1000).toFixed(1)}s`);
console.log(JSON.stringify(summarize(result), null, 2));
