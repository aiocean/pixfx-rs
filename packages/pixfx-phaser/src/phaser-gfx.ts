// Phaser's host objects for the surfaces Rust paints: atlas pages on WebGL (every surface a region of a few big
// textures, filled straight from the painted bytes with `texSubImage2D`, so the images of a layer go out in few
// batches), a canvas texture per surface on Phaser's Canvas renderer, and the images showing them.
import Phaser from "phaser";
import type { GfxImages, GfxTexture, GfxTextures } from "./gfx";

/** Side of an atlas page, at most: big enough for many surfaces, small enough for every WebGL device. */
const ATLAS_SIZE = 4096;

/** A row of an atlas page holding regions of one height. */
interface Shelf {
  y: number;
  h: number;
  /** Where the next region of the row starts. */
  x: number;
}

/** One atlas page: a GL texture known to Phaser by its key, filled shelf by shelf. */
interface Page {
  key: string;
  gl: Phaser.Renderer.WebGL.Wrappers.WebGLTextureWrapper;
  texture: Phaser.Textures.Texture;
  shelves: Shelf[];
  /** Where the next shelf starts. */
  top: number;
  /** Regions given back, by size (`"{w}x{h}"`), for reuse. */
  free: Map<string, { x: number; y: number }[]>;
}

/** Atlas pages in Phaser's WebGL renderer: each surface is a region, its frames named after it. */
export function phaserAtlas(renderer: Phaser.Renderer.WebGL.WebGLRenderer, textures: Phaser.Textures.TextureManager): GfxTextures {
  const gl = renderer.gl;
  const size = Math.min(ATLAS_SIZE, Number(gl.getParameter(gl.MAX_TEXTURE_SIZE)));
  const pages: Page[] = [];
  let next = 0;

  const newPage = (): Page => {
    const glTexture = renderer.createTexture2D(0, gl.NEAREST, gl.NEAREST, gl.CLAMP_TO_EDGE, gl.CLAMP_TO_EDGE, gl.RGBA, undefined, size, size, true, false, true);
    const key = `pixfx:atlas${pages.length}`;
    const texture = textures.addGLTexture(key, glTexture);
    if (!texture) throw new Error(`pixfx: texture key "${key}" is already in use`);
    const page: Page = { key, gl: glTexture, texture, shelves: [], top: 0, free: new Map() };
    pages.push(page);
    return page;
  };

  /** A `w × h` region: one given back, else on a shelf of its height, else on a new shelf or page. */
  const place = (w: number, h: number): { page: Page; x: number; y: number } => {
    if (w > size || h > size) throw new Error(`pixfx: a ${w}×${h} surface is bigger than an atlas page (${size})`);
    const bucket = `${w}x${h}`;
    for (const page of pages) {
      const spot = page.free.get(bucket)?.pop();
      if (spot) return { page, ...spot };
    }
    for (const page of pages) {
      const shelf = page.shelves.find((s) => s.h === h && s.x + w <= size);
      if (shelf) {
        const x = shelf.x;
        shelf.x += w;
        return { page, x, y: shelf.y };
      }
      if (page.top + h <= size) {
        page.shelves.push({ y: page.top, h, x: w });
        page.top += h;
        return { page, x: 0, y: page.top - h };
      }
    }
    const page = newPage();
    page.shelves.push({ y: 0, h, x: w });
    page.top = h;
    return { page, x: 0, y: 0 };
  };

  return (width, height) => {
    const { page, x, y } = place(width, height);
    const half = width / 2;
    const id = next++;
    const color = `s${id}c`;
    const light = `s${id}l`;
    const colorFrame = page.texture.add(color, 0, x, y, half, height);
    const lightFrame = page.texture.add(light, 0, x + half, y, half, height);
    if (!colorFrame || !lightFrame) throw new Error(`pixfx: atlas frame ${id} is already in use`);
    const texture: GfxTexture = {
      key: page.key,
      color,
      light,
      fill(c, l, w, h) {
        renderer.glTextureUnits.bind(page.gl, 0);
        // Phaser draws premultiplied from textures stored bottom row first (a frame's v0 is 1 - y / height): the
        // bytes are straight alpha, top row first, so they go in flipped, at the region's rows counted from the bottom
        renderer.glWrapper.updateTexturing({ texturing: { flipY: true, premultiplyAlpha: true } });
        const row = size - y - h;
        gl.texSubImage2D(gl.TEXTURE_2D, 0, x, row, w, h, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array(c.buffer, c.byteOffset, c.byteLength));
        gl.texSubImage2D(gl.TEXTURE_2D, 0, x + half, row, w, h, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array(l.buffer, l.byteOffset, l.byteLength));
        colorFrame.setSize(w, h, x, y);
        lightFrame.setSize(w, h, x + half, y);
      },
      destroy() {
        page.texture.remove(color);
        page.texture.remove(light);
        const bucket = `${width}x${height}`;
        const list = page.free.get(bucket);
        if (list) list.push({ x, y });
        else page.free.set(bucket, [{ x, y }]);
      },
    };
    return texture;
  };
}

/** A canvas texture per surface, for Phaser's Canvas renderer. */
export function phaserCanvasTextures(textures: Phaser.Textures.TextureManager): GfxTextures {
  let next = 0;
  return (width, height) => {
    const key = `pixfx:surface${next++}`;
    const tex = textures.createCanvas(key, width, height);
    if (!tex) throw new Error(`pixfx: texture key "${key}" is already in use`);
    const half = width / 2;
    return {
      key,
      color: "color",
      light: "light",
      fill(color, light, w, h) {
        tex.context.putImageData(new ImageData(color, w, h), 0, 0);
        tex.context.putImageData(new ImageData(light, w, h), half, 0);
        tex.remove("color");
        tex.remove("light");
        tex.add("color", 0, 0, 0, w, h);
        tex.add("light", 0, half, 0, w, h);
        tex.refresh();
      },
      destroy() {
        textures.remove(tex);
      },
    };
  };
}

/** The textures for `scene`'s renderer: an atlas on WebGL, canvas textures on the Canvas renderer. */
export function phaserTextures(scene: Phaser.Scene): GfxTextures {
  const renderer = scene.sys.game.renderer;
  return renderer instanceof Phaser.Renderer.WebGL.WebGLRenderer ? phaserAtlas(renderer, scene.textures) : phaserCanvasTextures(scene.textures);
}

/** Images of one scene, top-left origin, hidden until shown. */
export function phaserImages(scene: Phaser.Scene): GfxImages {
  return (blend) => {
    const img = scene.add.image(0, 0, "__DEFAULT").setOrigin(0, 0).setVisible(false);
    if (blend === "add") img.setBlendMode(Phaser.BlendModes.ADD);
    // what was set last, so a frame showing the same thing queues no depth sort
    let depth = Number.NaN;
    let scroll = Number.NaN;
    return {
      show(key, frame, x, y, w, h, d, s) {
        if (img.texture.key !== key) img.setTexture(key, frame);
        else img.setFrame(frame);
        img.setPosition(x, y).setDisplaySize(w, h).setVisible(true);
        if (d !== depth) img.setDepth((depth = d));
        if (s !== scroll) img.setScrollFactor((scroll = s));
      },
      hide() {
        img.setVisible(false);
      },
      destroy() {
        img.destroy();
      },
    };
  };
}
