// Prints pixelfx's SMOKE_RAMP as packed u32 words (R | G << 8 | B << 16 | A << 24), the goldens in tests/sim.rs.
// Usage: node smoke_ramp.mjs <path to a pixelfx checkout>   (goldens taken at pixelfx 044e12a)
import { pathToFileURL } from "node:url";
import { join, resolve } from "node:path";

const root = process.argv[2];
if (!root) throw new Error("usage: node smoke_ramp.mjs <pixelfx dir>");
const { SMOKE_RAMP } = await import(pathToFileURL(join(resolve(root), "src/core/sim.js")).href);
console.log(SMOKE_RAMP.map((c) => "0x" + (c >>> 0).toString(16).padStart(8, "0")).join(", "));
