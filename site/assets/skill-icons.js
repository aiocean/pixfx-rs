// Where each school's skill icons sit in the icon atlases (site/assets/skills-32.png and skills-24.png, made by
// site/art/skill-icons.sh from the sheet skill-icons.prompt.txt asks for): a square grid read left to right, top row
// first, in the prompt's order. Each school maps names to its own cells; cell 12, a blue bolt, is spare.

/** The atlas's columns, and its rows. */
export const ICON_COLUMNS = 5;

/** Each school's spells to their cell in the atlas. */
export const ICON_CELLS = {
  thunder: { bolt: 0, palm: 1, step: 2, seal: 3, orb: 4, bell: 5, heal: 6, spark: 7, burst: 8, quake: 9, hex: 10, tribulation: 11 },
  frost: {
    lotus: 13, shatter: 14, icicle: 15, collapse: 16, frostfire: 17, void: 18, mirror: 19, ward: 20, shard: 21,
    jet: 22, flurry: 23, prison: 24,
  },
};
