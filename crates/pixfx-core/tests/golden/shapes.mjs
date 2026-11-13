// Prints the goldens of tests/shapes.rs: fly samples, the calls cycle makes (with the first draw of each r), and
// the points of a bolt and of forks drawn from the same Rng.
// Usage: node shapes.mjs <path to a pixelfx checkout>   (goldens taken at pixelfx 044e12a)
import { pathToFileURL } from "node:url";
import { join, resolve } from "node:path";

const root = process.argv[2];
if (!root) throw new Error("usage: node shapes.mjs <pixelfx dir>");
const load = (f) => import(pathToFileURL(join(resolve(root), "src/core", f)).href);
const { fly, cycle } = await load("particles.js");
const { bolt, forks } = await load("shapes.js");
const { Rng } = await load("math.js");

const f = (v) => (Number.isInteger(v) ? v.toFixed(1) : String(v));
for (const d of [0, 1.5]) {
  for (const t of [0, 0.25, 1]) {
    const [x, y] = fly(0, 0, 40, -90, t, 300, d);
    console.log(`    (${f(t)}, ${f(d)}, ${f(x)}, ${f(y)}),`);
  }
}
for (const [name, life] of [["fn", (g) => 0.5 + g.next()], ["num", 0.8]]) {
  console.log(`cycle ${name}`);
  cycle(1.3, 2, 6, 9, life, (age, L, r, i) => console.log(`    (${f(age)}, ${f(L)}, ${f(r.next())}, ${i}),`));
}
const r = new Rng(4);
const b = bolt(r, 0, 0, 40, 60);
console.log(`bolt ${b.length}`);
for (const [x, y] of b) console.log(`    (${f(x)}, ${f(y)}),`);
const fk = forks(r, b);
console.log(`forks ${fk.length}`);
for (const p of fk) {
  console.log(`  path ${p.length}`);
  for (const [x, y] of p) console.log(`    (${f(x)}, ${f(y)}),`);
}
