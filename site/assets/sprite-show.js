// A show on sprites: Thunder and Frost, two pixfx-fighter looks rendered to sprite sheets, drawn by the page the way
// any game draws its sprites, with the engine's effects played on them. An effect that reads the sprite it plays on
// is handed the fighter's frame as it shows now (`imageActor`); while one takes the body over (`replacesActor`, an
// appear or a disappear) the sprite is hidden and the effect draws the body.
//
// The sheets (site/assets/sprites/) come from the engine's pose script, one cell per pose in `POSES` order:
//   cargo run -p pixfx-fighter --example poses -- <thunder|frost> --out site/assets/sprites
//
// Units are game pixels and seconds; y grows down, `ground` is the row the fighters' soles stand on.
import { createPixfx, effects, imageActor, meta } from "../pkg/pixfx-canvas2d.js";
import { DEMOS } from "./effect-demos.js";
import { SHOW_BOX, paintStage, stageFloor } from "./show-stage.js";

/** The sheets' cells, in pixfx-fighter's `POSES` order (crates/pixfx-fighter/src/poses.rs). */
const POSES = [
  "idle", "walk", "jump", "fly", "land", "crouch", "duck", "draw",
  "slash", "channel", "cast", "guard", "meditate", "hurt", "frozen", "shocked",
  "launched", "tumble", "fall", "down", "roll", "sit", "kneel", "win",
];
/** A cell's size, where the feet are in it (pixfx-fighter `SPRITE`, `CX`, `FEET`), and cells a row. */
const CELL = 104;
const CX = 52;
const FEET = 96;
const COLUMNS = 8;
/** Where a fighter's chest is above its soles: where an effect that reads no sprite plays. */
const CHEST = 20;
/** A dash's seconds each way, and its pause at the far end. */
const DASH = 0.22;
const DASH_HOLD = 0.45;
/** How long a looping effect plays before it is stopped. */
const HOLD = 2.4;
/** Seconds after a take ends before the next starts. */
const REST = 0.6;
/** A take that has not ended by then is cut short. */
const TAKE_MAX = 9;
/** The respawn: a fighter an effect took away comes back in it. */
const RESPAWN = "appear/teleport-beam";
/** An exit whose own entrance brings the fighter back instead: a fade out is answered by a fade in. */
const COMES_BACK = { "disappear/fade": "appear/fade" };
/** How the fighter an effect lands on stands while it plays. */
const STRUCK = { "feedback/electrified": "shocked", "feedback/encase": "frozen", "feedback/frozen": "frozen", "feedback/soaked": "hurt" };
/** Effects the fighter that casts them wears; the rest of the sprite-reading ones land on the other. */
const WORN = new Set(["feedback/power-up", "feedback/outline", "feedback/cloak-shimmer", "disappear/blink-out", "disappear/fade", "appear/fade", RESPAWN]);

/** Load the two sheets. */
export async function loadFighters(pixelDensity = 2) {
  const folder = pixelDensity === 2 ? "./sprites/dense" : "./sprites";
  const load = async (look) => {
    const image = new Image();
    image.src = new URL(`${folder}/${look}-poses.png`, import.meta.url).href;
    await image.decode();
    return image;
  };
  const [thunder, frost] = await Promise.all([load("thunder"), load("frost")]);
  return { thunder, frost };
}

/** The ids the show can stage, in registry order: everything but a page transition, which takes the whole screen. */
export function showable() {
  return effects().filter((id) => !id.startsWith("transitions/"));
}

/** A fighter of `sheet` standing at `x`, facing right unless `flip`. */
function fighter(name, sheet, x, ground, flip, pixelDensity) {
  const f = { name, sheet, x, home: x, ground, flip, pose: "idle", lift: 0, hidden: false };
  const cell = CELL * pixelDensity;
  const cut = () => {
    const i = Math.max(0, POSES.indexOf(f.pose));
    return { x: (i % COLUMNS) * cell, y: Math.floor(i / COLUMNS) * cell, w: cell, h: cell };
  };
  f.frame = () => ({ image: sheet, cut: cut(), x: Math.round(f.x) - CX, y: f.ground - FEET - f.lift, w: CELL, h: CELL, flipX: flip });
  // an effect reads the fighter even while the page hides it: one that takes the body over draws it from this frame
  f.actor = imageActor(f.frame);
  f.draw = (ctx, scale) => {
    if (f.hidden) return;
    const d = f.frame();
    ctx.save();
    ctx.translate(d.x * scale + (flip ? CELL * scale : 0), d.y * scale);
    if (flip) ctx.scale(-1, 1);
    ctx.drawImage(sheet, d.cut.x, d.cut.y, d.cut.w, d.cut.h, 0, 0, CELL * scale, CELL * scale);
    ctx.restore();
  };
  return f;
}

/**
 * The show on `canvas` at `scale` canvas pixels per game pixel: the two fighters and the takes of `ids`
 * in turn, each effect staged the way a fight would use it, again and again. `told` gets each take's sentence.
 * `step(dt)` plays and draws; `playing()` is how many effects are on screen; `restart(opts)` starts the take over
 * with new options for the effect.
 */
export function createShow(canvas, { scale = 2, sheets, ids, told = () => {}, still = false, scenario = null, replay = true, pixelDensity = 2 }) {
  const { w, h } = SHOW_BOX;
  canvas.width = w * scale;
  canvas.height = h * scale;
  const ctx = canvas.getContext("2d");
  ctx.imageSmoothingEnabled = false;
  const ground = stageFloor(h);
  const gap = 84;
  const left = fighter("Thunder", sheets.thunder, Math.round(w / 2 - gap / 2), ground, false, pixelDensity);
  const right = fighter("Frost", sheets.frost, Math.round(w / 2 + gap / 2), ground, true, pixelDensity);
  const fighters = [left, right];
  const first = ids[0];
  if (!replay && first?.startsWith("appear/")) (WORN.has(first) ? left : right).hidden = true;
  if (!replay && first === "spawn/poof") left.hidden = true;
  const backdrop = new OffscreenCanvas(canvas.width, canvas.height);
  const backdropCtx = backdrop.getContext("2d");
  let backdropTone;
  const stage = createPixfx(ctx, { scale, pixelDensity });
  if (still) stage.feel.setShakeStrength(0).setFlashStrength(0);

  let take = null;
  let turn = 0;
  let resting = 0;
  let opts = {};

  function next() {
    const id = ids[turn % ids.length];
    const caster = fighters[turn % 2];
    turn++;
    begin(id, caster);
  }

  function begin(id, caster) {
    take?.end();
    for (const f of fighters) Object.assign(f, { pose: "idle", lift: 0, hidden: false, x: f.home });
    take = stageTake(stage, id, caster, fighters.find((f) => f !== caster), opts, ground, {
      scenario: scenario === null ? DEMOS[id]?.[0]?.[2] : scenario, replay,
    });
    told(take.caption);
    resting = 0;
  }

  const view = {
    canvas,
    restart(o) {
      opts = o ?? {};
      turn = Math.max(0, turn - 1);
      next();
    },
    playing: () => take?.live() ?? 0,
    draw,
    finished: () => take?.done ?? false,
    step(dt) {
      if (!take) next();
      take.step(dt);
      if (replay && take.done) {
        resting += dt;
        if (resting >= REST) next();
      }
      stage.update(dt);
      if (!replay && take.done) take.end();
      draw();
    },
    free() {
      take?.end();
      stage.destroy();
    },
  };

  function draw(withEffects = true) {
    const [sx, sy] = stage.feel.shake;
    const tone = (take?.id ?? first) === "feedback/outline" ? "outline" : "night";
    if (tone !== backdropTone) {
      paintStage(backdropCtx, w, h, scale, tone);
      backdropTone = tone;
    }
    ctx.drawImage(backdrop, 0, 0);
    ctx.save();
    ctx.translate(Math.round(sx) * scale, Math.round(sy) * scale);
    if (sx || sy) ctx.drawImage(backdrop, 0, 0);
    if (withEffects) stage.drawLayer("back");
    for (const f of fighters) f.draw(ctx, scale);
    take?.drawProp?.(ctx, scale);
    if (withEffects) stage.drawLayer("front");
    ctx.restore();
    if (!withEffects) return;
    stage.warp();
    const flash = stage.feel.flash;
    if (flash > 0) {
      ctx.fillStyle = `rgba(255, 255, 255, ${flash})`;
      ctx.fillRect(0, 0, w * scale, h * scale);
    }
  }

  return view;
}

/**
 * One take: `id` staged between `caster` and `foe`. Returns its caption, `step(dt)`, whether it is `done`, how many
 * effects are `live()`, and `end()` to cut it short.
 */
function stageTake(stage, id, caster, foe, opts, ground, { scenario, replay }) {
  const m = meta(id);
  const live = new Set();
  const timers = [];
  let t = 0;
  let movers = [];
  const at = (when, fn) => timers.push({ when, fn });

  /** Play `fx` with `o` at (`x`, `y`); worn by `wearer` when it reads the sprite it plays on. */
  const play = (fx, x, y, o, wearer, hold = HOLD) => {
    const fm = meta(fx);
    const handle = stage.play(fx, Math.round(x), Math.round(y), { seed: fm.seed, ...o });
    live.add(handle);
    if (wearer && fm.wrapsActor) handle.wrap(wearer.actor);
    const takes = wearer && takesBody(fx, fm);
    if (takes) wearer.hidden = true;
    handle.on("complete", () => {
      live.delete(handle);
      if (!wearer) return;
      if (STRUCK[fx]) wearer.pose = "idle";
      if (wearer.pose === "channel") wearer.pose = "idle";
      if (!takes) return;
      // the one an effect took away comes back: in the respawn effect when there is one
      const back = COMES_BACK[fx] ?? RESPAWN;
      if (fx.startsWith("appear/") || !has(back)) wearer.hidden = false;
      else if (replay) at(t + 0.5, () => play(back, wearer.x, wearer.ground, {}, wearer));
    });
    if (fm.loop && hold) at(t + Math.max(hold, fm.duration), () => handle.stop());
    return handle;
  };

  let caption;
  let drawProp;
  const violetWake = id === "projectiles/violet-wake";
  if (scenario === "gust") {
    caption = "The ribbons curl and break apart around a fixed point.";
    const handle = play(id, (caster.x + foe.x) * 0.5, ground - CHEST, opts);
    if (violetWake) at(m.duration, () => handle.stop());
  } else if (id === "feedback/afterimage") {
    caption = `${caster.name} leaves a fading trace at the start of the dash.`;
    caster.pose = "walk";
    const ghost = caster.actor();
    const start = caster.x;
    const finish = (caster.x + foe.x) * 0.5;
    play(id, start, ground, opts).wrap(() => ghost);
    movers.push((now) => {
      caster.x = start + (finish - start) * ease(Math.min(1, now / DASH));
      if (now < DASH) return true;
      caster.pose = "idle";
      return false;
    });
  } else if (id === "feedback/occlusion") {
    caption = `${foe.name}'s silhouette shows through the pillar.`;
    const start = foe.x;
    const left = start - 6;
    const top = ground - 75;
    const width = 18;
    const direction = foe.flip ? -1 : 1;
    foe.pose = "walk";
    play(id, start, ground, { left: left - start, top: top - ground, width, height: 75, ...opts }, foe);
    movers.push((time) => {
      const progress = Math.min(1, time / m.duration);
      foe.x = start + (progress - 0.5) * direction * 42;
      if (progress < 1) return true;
      foe.pose = "idle";
      return false;
    });
    drawProp = (ctx, scale) => {
      ctx.fillStyle = "#362c46";
      ctx.fillRect(left * scale, top * scale, width * scale, 75 * scale);
    };
  } else if (id === "feedback/arc-link") {
    caption = `An arc reaches from ${caster.name} to ${foe.name}.`;
    caster.pose = "cast";
    const direction = caster.flip ? -1 : 1;
    const y = ground - 38;
    const source = outlineX(caster.actor(), y, -direction);
    const contact = outlineX(foe.actor(), y, direction);
    play(id, foe.x, ground, {
      fromX: source - foe.x, fromY: y - ground, contactX: contact - foe.x, contactY: y - ground, ...opts,
    }, foe);
    at(0.35, () => (caster.pose = "idle"));
  } else if (id === "melee/sword-slash") {
    caption = `${caster.name} cuts from overhead.`;
    caster.pose = "slash";
    play(id, caster.x, ground, { ...opts, flip: caster.flip }, caster);
    at(0.6, () => (caster.pose = "idle"));
  } else if (id === "feedback/ricochet") {
    caption = `${caster.name}'s bolt hits ${foe.name}'s outline and throws sparks back.`;
    caster.pose = "cast";
    const direction = caster.flip ? -1 : 1;
    const bolt = makeBolt(direction);
    const contactY = ground - 38;
    const contactX = outlineX(foe.actor(), contactY, direction);
    const tip = direction > 0 ? bolt.width - 1 : 0;
    const startX = caster.x + 14 * direction - tip;
    const hitX = contactX - tip;
    const hitY = contactY - Math.floor(bolt.height / 2);
    const rise = 9;
    const flight = 0.25;
    const angle = Math.atan2(-rise, hitX - startX) * 180 / Math.PI;
    const cut = { x: 0, y: 0, w: bolt.width, h: bolt.height };
    // Sample the bolt where it touches the target, even after the bolt rebounds.
    const actor = imageActor(() => ({ image: bolt, cut, x: hitX, y: hitY }));
    at(flight, () => {
      play(id, contactX, contactY, {
        normalAngle: direction > 0 ? 180 : 0,
        incomingAngle: (angle + 360) % 360,
        ...opts,
      }).wrap(actor);
    });
    at(flight + m.duration, () => { caster.pose = "idle"; });
    drawProp = (ctx, scale) => {
      if (t >= flight + m.duration) return;
      const u = t / flight;
      const rebound = Math.max(0, u - 1 - 0.18) * 0.55;
      const travel = u <= 1 ? u : 1 - rebound;
      const x = Math.round(startX + (hitX - startX) * travel);
      const y = Math.round(hitY + rise * (u <= 1 ? 1 - u : travel - 1));
      ctx.drawImage(bolt, x * scale, y * scale, bolt.width * scale, bolt.height * scale);
    };
  } else if (scenario === "hit" || scenario === "shield" || scenario === "shield-break") {
    const direction = caster.flip ? -1 : 1;
    const shielding = scenario.startsWith("shield");
    const shatter = opts.shatter ?? Number(scenario === "shield-break");
    const redrawShield = shielding && (id === "feedback/shield-absorb" || shatter >= 0.5);
    caption = shielding
      ? `${foe.name} takes the bolt on a shield.`
      : `${caster.name}'s bolt strikes ${foe.name} at the edge of the sprite.`;
    caster.pose = "cast";
    if (shielding) foe.pose = "guard";

    const y = ground - 38;
    const center = foe.x - 21 * direction;
    const shield = shielding ? makeShield() : undefined;
    const shieldX = center - (shield ? shield.width / 2 : 0);
    const shieldY = y - (shield ? shield.height / 2 : 0);
    const actor = shield && imageActor(() => ({
      image: shield, cut: { x: 0, y: 0, w: shield.width, h: shield.height }, x: shieldX, y: shieldY,
    }));
    const contact = actor ? outlineX(actor(), y, direction) : outlineX(foe.actor(), y, direction);
    const bolt = makeBolt(direction);
    const tip = direction > 0 ? bolt.width - 1 : 0;
    const from = outlineX(caster.actor(), y, -direction) - tip;
    const to = contact - tip;
    const flight = 0.28;
    const interval = 0.38;
    const strikeCount = id === "impacts/charged-pulse" ? 3 : 1;
    const finish = (strikeCount - 1) * interval + flight + m.duration;
    for (let i = 0; i < strikeCount; i++) {
      at(i * interval + flight, () => {
        if (actor) {
          const contactOpts = { contactX: contact - center, contactY: 0, ...opts };
          if (id === "feedback/shield-crack") contactOpts.shatter = shatter;
          play(id, center, y, contactOpts).wrap(actor);
        } else if (m.wrapsActor) {
          const contactOpts = { contactX: contact - foe.x, contactY: y - ground, ...opts };
          if (id === "impacts/directional-wave") contactOpts.heading = opts.heading ?? (direction > 0 ? 0 : 180);
          if (strikeCount > 1) contactOpts.charge = opts.charge ?? (i + 1) / strikeCount;
          play(id, foe.x, ground, contactOpts, foe);
        } else {
          play(id, contact, y, opts);
        }
      });
    }
    at(finish, () => { caster.pose = foe.pose = "idle"; });
    drawProp = (ctx, scale) => {
      if (shield && (!redrawShield || t < flight || (id === "feedback/shield-absorb" && t >= finish))) {
        ctx.drawImage(shield, shieldX * scale, shieldY * scale, shield.width * scale, shield.height * scale);
      }
      const strike = Math.floor(t / interval);
      const age = t - strike * interval;
      if (strike < strikeCount && age < flight + 0.04) {
        const x = Math.round(from + (to - from) * Math.min(1, age / flight));
        ctx.drawImage(bolt, x * scale, (y - Math.floor(bolt.height / 2)) * scale, bolt.width * scale, bolt.height * scale);
      }
    };
  } else if (scenario === "projectile") {
    caption = `${caster.name} sends ${m.name} towards ${foe.name}.`;
    caster.pose = "cast";
    const y = ground - 38;
    const direction = Math.sign(foe.x - caster.x);
    const start = outlineX(caster.actor(), y, -direction);
    const duration = 0.68;
    let x = start;
    const bolt = makeBolt(direction);
    const tip = direction > 0 ? bolt.width - 1 - Math.floor(bolt.width / 2) : -Math.floor(bolt.width / 2);
    const end = outlineX(foe.actor(), y, direction) - tip;
    const comet = id === "feedback/comet-weave";
    const cut = { x: 0, y: 0, w: bolt.width, h: bolt.height };
    const left = () => x - Math.floor(bolt.width / 2);
    const top = y - Math.floor(bolt.height / 2);
    const actor = imageActor(() => ({ image: bolt, cut, x: left(), y: top }));
    // The weave's local axis points up. Ask the engine to turn it into the spell's heading.
    const headings = m.options.some((option) => option.name === "heading");
    const spell = comet ? { length: 4, ...opts, angle: direction * Math.PI / 2 }
      : headings ? { heading: direction > 0 ? 0 : 180, ...opts } : opts;
    const handle = play(id, x, y, spell, undefined, 0);
    if (m.wrapsActor) handle.wrap(actor);
    movers.push((now) => {
      x = Math.round(start + (end - start) * Math.min(1, now / duration));
      handle.setPosition(x, y);
      if (now < duration) return true;
      foe.pose = "hurt";
      caster.pose = "idle";
      at(now + 0.35, () => (foe.pose = "idle"));
      if (m.loop || violetWake) handle.stop();
      return false;
    });
    drawProp = (ctx, scale) => {
      if (t > duration) return;
      ctx.drawImage(bolt, left() * scale, top * scale, bolt.width * scale, bolt.height * scale);
    };
  } else if (scenario === "escort") {
    caption = `${m.name} stays anchored as ${caster.name} changes direction.`;
    const handle = play(id, caster.x, ground, opts, caster, 0);
    const duration = m.loop ? 1.6 : m.duration;
    const distance = (foe.x - caster.x) * 0.45;
    caster.pose = "walk";
    movers.push((now) => {
      const u = Math.min(1, now / duration);
      caster.x = caster.home + distance * Math.sin(Math.PI * u);
      handle.setPosition(Math.round(caster.x), ground);
      if (u < 1) return true;
      caster.pose = "idle";
      if (m.loop) handle.stop();
      return false;
    });
  } else if (violetWake || id === "projectiles/trail") {
    // the caster dashes at the other and back wearing the trail: it smears the fighter's own back edge, in its own
    // colours, along the way it moved
    caption = id === "projectiles/trail" ? `${caster.name}'s dash leaves a body afterimage.`
      : `${caster.name} dashes and leaves a wake that cools as they stop.`;
    const direction = Math.sign(foe.home - caster.home);
    const reach = foe.home - direction * 30;
    const wakeOffset = violetWake ? -direction * m.origin[0] : 0;
    const y = violetWake ? ground - CHEST : ground;
    const wakeOptions = violetWake ? { ...opts, flip: direction > 0 } : opts;
    const handle = play(id, caster.x + wakeOffset, y, wakeOptions, caster, 0);
    caster.pose = "walk";
    movers.push((now) => {
      if (violetWake) {
        const progress = Math.min(1, now / (DASH * 1.6));
        caster.x = caster.home + (reach - caster.home) * ease(progress);
        handle.setPosition(Math.round(caster.x + wakeOffset), y);
        if (progress < 1) return true;
        caster.pose = "idle";
        handle.stop();
        return false;
      }

      // a dash: out fast, a breath at the other, back fast
      const out = now < DASH ? ease(now / DASH) : now < DASH + DASH_HOLD ? 1 : ease(Math.max(0, 1 - (now - DASH - DASH_HOLD) / DASH));
      caster.x = caster.home + (reach - caster.home) * out;
      handle.setPosition(Math.round(caster.x), y);
      if (now >= 2 * DASH + DASH_HOLD) {
        caster.pose = "idle";
        handle.stop();
        return false;
      }
      return true;
    });
  } else if (id === "impacts/parry") {
    caption = `${caster.name} and ${foe.name} cross blades`;
    caster.pose = foe.pose = "slash";
    const direction = caster.flip ? -1 : 1;
    const y = ground - 36;
    const tips = outlineX(caster.actor(), y, -direction) + outlineX(foe.actor(), y, direction);
    play(id, tips / 2, ground, { height: ground - y, ...opts });
    at(0.6, () => (caster.pose = foe.pose = "idle"));
  } else if (id === "projectiles/arcane-meteor") {
    caption = `${caster.name} calls a meteor down on ${foe.name}.`;
    caster.pose = "cast";
    const end = { x: foe.x, y: foe.ground - CHEST };
    const start = { x: end.x + Math.sign(caster.x - foe.x) * 38, y: end.y - 44 };
    const meteor = play(id, end.x, end.y, {
      ...opts, startX: start.x - end.x, startY: start.y - end.y,
    }, undefined, 0);
    at(11 / m.fps, () => { foe.pose = "hurt"; caster.pose = "idle"; });
    at(m.duration, () => {
      meteor.destroy();
      live.delete(meteor);
      foe.pose = "idle";
    });
  } else if (id.startsWith("slashes/")) {
    caption = `${caster.name} swings at ${foe.name}.`;
    caster.pose = "slash";
    const direction = caster.flip ? -1 : 1;
    const handle = play(id, caster.x + 19 * direction, ground - CHEST, { ...opts, flip: caster.flip });
    if (id === "slashes/slash-3") at(m.duration, () => handle.stop());
    at(m.duration, () => { caster.pose = "idle"; });
  } else if (id === "feedback/foot-contact") {
    caption = `${caster.name}'s leading boot kicks up powder on landing.`;
    const airTime = 0.3;
    caster.pose = "jump";
    movers.push((now) => {
      const flight = Math.min(1, now / airTime);
      caster.lift = Math.round(17 * Math.sin(Math.PI * flight));
      return flight < 1;
    });
    at(airTime, () => {
      caster.pose = "land";
      const direction = caster.flip ? -1 : 1;
      const x = outlineX(caster.actor(), ground - 1, direction, caster.x);
      play(id, caster.x, ground, {
        surface: 2, contactX: x - caster.x, contactY: -2, heading: caster.flip ? 180 : 0, ...opts,
      }, caster);
    });
    at(airTime + m.duration, () => { caster.pose = "idle"; });
  } else if (id === "impacts/thunderstrike") {
    caption = `${caster.name} calls down lightning at ${foe.name}'s feet`;
    caster.pose = "cast";
    play(id, foe.x, foe.ground, opts);
    at(0.6, () => (caster.pose = "idle"));
  } else if (id === "spawn/poof") {
    caption = `${caster.name} pops out of a puff of smoke`;
    caster.hidden = true;
    play(id, caster.x, ground, opts);
    at(0.12, () => (caster.hidden = false));
  } else if (id === "feedback/telegraph") {
    caption = `Light runs across ${caster.name}'s stance before the swing.`;
    caster.pose = "draw";
    play(id, caster.x, ground, opts, caster);
    at(m.duration, () => { caster.pose = "slash"; });
    at(m.duration + 0.15, () => { caster.pose = "idle"; });
  } else if (id === "feedback/violet-portal") {
    caption = `${caster.name} opens a portal in the space between the fighters.`;
    caster.pose = "cast";
    play(id, (caster.x + foe.x) / 2, ground - 26, opts);
    at(m.duration, () => { caster.pose = "idle"; });
  } else if (id === "feedback/amber-ward") {
    caption = `${caster.name} raises a ward over the guarding arm.`;
    caster.pose = "guard";
    const direction = caster.flip ? -1 : 1;
    const y = ground - 34;
    const x = outlineX(caster.actor(), y, -direction);
    play(id, x + 3 * direction, y, opts);
    at(m.duration, () => { caster.pose = "idle"; });
  } else if (id === "feedback/rim-light") {
    caption = `${caster.name}'s spell lights the near edge of ${foe.name}.`;
    caster.pose = "cast";
    play(id, foe.x, ground, { sourceAngle: caster.flip ? 0 : 180, ...opts }, foe);
    at(m.duration, () => { caster.pose = "idle"; });
  } else if (id === "critters/wisp") {
    caption = `${caster.name} calls a spirit companion.`;
    caster.pose = "cast";
    play(id, caster.x + (caster.flip ? 28 : -28), ground, opts);
    at(0.55, () => (caster.pose = "idle"));
  } else if (m.wrapsActor || m.replacesActor) {
    const wearer = scenario !== "target" && WORN.has(id) ? caster : foe;
    if (id.startsWith("appear/")) {
      wearer.hidden = true;
      caption = `${wearer.name} enters the arena.`;
    } else if (id.startsWith("disappear/") || id === "explosions/body-burst") {
      caption = `${wearer.name} leaves the arena in ${m.name}.`;
    } else {
      caption = wearer === caster ? `${m.name} on ${caster.name}.` : `${caster.name} casts ${m.name} on ${foe.name}.`;
    }
    caster.pose = wearer === caster ? "channel" : "cast";
    if (wearer === foe) at(0.6, () => caster.pose === "cast" && (caster.pose = "idle"));
    at(wearer === caster ? 0 : 0.25, () => {
      if (STRUCK[id]) wearer.pose = STRUCK[id];
      play(id, wearer.x, wearer.ground, opts, wearer);
    });
  } else {
    // an effect that reads no sprite plays beside the caster, at its chest
    caption = `${m.name} beside ${caster.name}`;
    caster.pose = "cast";
    at(0.6, () => (caster.pose = "idle"));
    play(id, caster.x + (caster.flip ? -24 : 24), ground - CHEST, opts);
  }

  return {
    id,
    caption,
    drawProp,
    get done() {
      const idle = live.size === 0 && timers.length === 0 && movers.length === 0;
      return t > TAKE_MAX || (t > 0.4 && idle);
    },
    live: () => live.size,
    step(dt) {
      t += dt;
      movers = movers.filter((fn) => fn(t));
      for (const timer of timers.filter((x) => x.when <= t)) {
        timers.splice(timers.indexOf(timer), 1);
        timer.fn();
      }
    },
    end() {
      for (const h of live) h.destroy();
      live.clear();
      timers.length = 0;
      movers = [];
    },
  };
}

function outlineX({ at, rgba }, y, direction, from) {
  const row = Math.floor((y - at.y) * at.h / at.shownH);
  const start = from === undefined
    ? direction > 0 ? 0 : at.w - 1
    : Math.floor((from - at.x) * at.w / at.shownW);
  for (let x = start; x >= 0 && x < at.w; x += direction) {
    const sourceX = at.flipX ? at.w - 1 - x : x;
    if (rgba[(row * at.w + sourceX) * 4 + 3] > 127) {
      return at.x + Math.floor(x * at.shownW / at.w);
    }
  }
  return at.x + Math.floor(at.shownW / 2);
}

function makeBolt(direction) {
  const rows = [
    "       dd    ",
    "    ddvvvd   ",
    " ddvvvpppvd  ",
    "dvvvppwwwwpvd",
    " ddvvvpppvd  ",
    "    ddvvvd   ",
    "       dd    ",
  ];
  const bolt = new OffscreenCanvas(rows[0].length, rows.length);
  const paint = bolt.getContext("2d");
  const colors = { d: "#514755", v: "#968191", p: "#d9c7ce", w: "#fff2dc" };
  for (const [row, pixels] of rows.entries()) {
    for (const [column, pixel] of [...pixels].entries()) {
      if (pixel === " ") continue;
      paint.fillStyle = colors[pixel];
      paint.fillRect(direction > 0 ? column : bolt.width - 1 - column, row, 1, 1);
    }
  }
  return bolt;
}

function makeShield() {
  const shield = new OffscreenCanvas(24, 34);
  const ctx = shield.getContext("2d");
  const plate = (inset, fill) => {
    ctx.fillStyle = fill;
    ctx.beginPath();
    ctx.moveTo(12, 1 + inset);
    ctx.lineTo(23 - inset, 6 + inset);
    ctx.lineTo(23 - inset, 21 - inset);
    ctx.lineTo(12, 33 - inset);
    ctx.lineTo(1 + inset, 21 - inset);
    ctx.lineTo(1 + inset, 6 + inset);
    ctx.closePath();
    ctx.fill();
  };
  plate(0, "#d5b678");
  plate(2, "#35455a");
  plate(4, "#4d6680");
  ctx.fillStyle = "#9cc1d3";
  ctx.fillRect(11, 11, 2, 9);
  ctx.fillRect(8, 14, 8, 2);
  return shield;
}

/** Whether the fighter is hidden while `fx` plays: an effect that replaces its actor draws what is left of it, and an
 *  appear or disappear effect draws the body itself while the game hides its sprite (pixfx-effects `appear`,
 *  `disappear`). */
function takesBody(fx, fm) {
  return fm.replacesActor || fx.startsWith("appear/") || fx.startsWith("disappear/");
}

function has(id) {
  return effects().includes(id);
}

function ease(k) {
  return k * k * (3 - 2 * k);
}

