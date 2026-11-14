// Pixi's host objects for the surfaces Rust paints: atlas pages on WebGL (every surface a region of a few big
// textures, filled straight from the painted bytes with `texSubImage2D`, so the sprites of a layer go out in few
// batches), a buffer texture per surface on other renderers, and the sprites showing them.
import type { Container, Renderer } from "pixi.js";
import { BufferImageSource, Rectangle, Sprite, Texture, TextureSource, WebGLRenderer } from "pixi.js";
import type { GfxImages, GfxTexture, GfxTextures } from "pixfx-web";

/** Side of an atlas page, at most: big enough for many surfaces, small enough for every WebGL device. */
const ATLAS_SIZE = 4096;

/** The textures a surface's boxes are shown through, by `"{key}/{frame}"`. */
export type TextureOf = (key: string, frame: string) => Texture | undefined;

/** A row of an atlas page holding regions of one height. */
interface Shelf {
  y: number;
  h: number;
  /** Where the next region of the row starts. */
  x: number;
}

/** One atlas page, filled shelf by shelf. */
interface Page {
  source: TextureSource;
  shelves: Shelf[];
  /** Where the next shelf starts. */
  top: number;
  /** Regions given back, by size (`"{w}x{h}"`), for reuse. */
  free: Map<string, { x: number; y: number }[]>;
}

/** The surfaces' textures, and the lookup images use to show them. */
export interface PixiTextures {
  textures: GfxTextures;
  textureOf: TextureOf;
}

/** Textures for `renderer`: an atlas on WebGL, a buffer texture per surface otherwise. */
export function pixiTextures(renderer: Renderer): PixiTextures {
  if (!(renderer instanceof WebGLRenderer)) return pixiBufferTextures();
  const shown = new Map<string, Texture>();
  return { textures: pixiAtlas(renderer, shown), textureOf: lookup(shown) };
}

/** A buffer texture per surface, for any renderer. */
export function pixiBufferTextures(): PixiTextures {
  const shown = new Map<string, Texture>();
  return { textures: pixiBuffers(shown), textureOf: lookup(shown) };
}

function lookup(shown: Map<string, Texture>): TextureOf {
  return (key, frame) => shown.get(`${key}/${frame}`);
}

/** A texture over `w × h` of `source` at `(x, y)`. */
function region(source: TextureSource, x: number, y: number, w: number, h: number): Texture {
  return new Texture({ source, frame: new Rectangle(x, y, w, h) });
}

/** Make `t` show `w × h` from its corner, in place, so sprites showing it follow. */
function resize(t: Texture | undefined, w: number, h: number): void {
  if (!t || (t.frame.width === w && t.frame.height === h)) return;
  t.frame.width = w;
  t.frame.height = h;
  t.orig.width = w;
  t.orig.height = h;
  t.update();
}

function pixiAtlas(renderer: WebGLRenderer, shown: Map<string, Texture>): GfxTextures {
  const gl = renderer.gl;
  const size = Math.min(ATLAS_SIZE, Number(gl.getParameter(gl.MAX_TEXTURE_SIZE)));
  const pages: Page[] = [];
  let next = 0;

  const newPage = (): Page => {
    const source = new TextureSource({ width: size, height: size, scaleMode: "nearest", autoGenerateMipmaps: false });
    const page: Page = { source, shelves: [], top: 0, free: new Map() };
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

  /** Put `bytes` (`w × h` RGBA, straight alpha, top row first) at `(x, y)` of `source`, premultiplied as Pixi draws. */
  const upload = (source: TextureSource, x: number, y: number, w: number, h: number, bytes: Uint8ClampedArray): void => {
    // bound through Pixi, so its record of what is bound stays true; the unpack flag is put back as Pixi left it
    renderer.texture.bindSource(source, 0);
    const premultiplied = gl.getParameter(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL) === true;
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, true);
    gl.texSubImage2D(gl.TEXTURE_2D, 0, x, y, w, h, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength));
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, premultiplied);
  };

  return (width, height) => {
    const { page, x, y } = place(width, height);
    const half = width / 2;
    const key = `s${next++}`;
    shown.set(`${key}/c`, region(page.source, x, y, half, height));
    shown.set(`${key}/l`, region(page.source, x + half, y, half, height));
    const texture: GfxTexture = {
      key,
      color: "c",
      light: "l",
      fill(c, l, w, h) {
        upload(page.source, x, y, w, h, c);
        upload(page.source, x + half, y, w, h, l);
        resize(shown.get(`${key}/c`), w, h);
        resize(shown.get(`${key}/l`), w, h);
      },
      destroy() {
        shown.get(`${key}/c`)?.destroy(false);
        shown.get(`${key}/l`)?.destroy(false);
        shown.delete(`${key}/c`);
        shown.delete(`${key}/l`);
        const bucket = `${width}x${height}`;
        const list = page.free.get(bucket);
        if (list) list.push({ x, y });
        else page.free.set(bucket, [{ x, y }]);
      },
    };
    return texture;
  };
}

function pixiBuffers(shown: Map<string, Texture>): GfxTextures {
  let next = 0;
  return (width, height) => {
    const key = `s${next++}`;
    const half = width / 2;
    const bytes = new Uint8Array(width * height * 4);
    const source = new BufferImageSource({ resource: bytes, width, height, scaleMode: "nearest", autoGenerateMipmaps: false });
    shown.set(`${key}/c`, region(source, 0, 0, half, height));
    shown.set(`${key}/l`, region(source, half, 0, half, height));
    return {
      key,
      color: "c",
      light: "l",
      fill(c, l, w, h) {
        for (let row = 0; row < h; row++) {
          const from = row * w * 4;
          bytes.set(c.subarray(from, from + w * 4), row * width * 4);
          bytes.set(l.subarray(from, from + w * 4), (row * width + half) * 4);
        }
        source.update();
        resize(shown.get(`${key}/c`), w, h);
        resize(shown.get(`${key}/l`), w, h);
      },
      destroy() {
        shown.get(`${key}/c`)?.destroy(false);
        shown.get(`${key}/l`)?.destroy(false);
        shown.delete(`${key}/c`);
        shown.delete(`${key}/l`);
        source.destroy();
      },
    };
  };
}

/** Sprites in `container` (which sorts children by `zIndex`), top-left origin, hidden until shown. */
export function pixiImages(container: Container, textureOf: TextureOf): GfxImages<null> {
  return (blend) => {
    const sprite = new Sprite(Texture.EMPTY);
    sprite.visible = false;
    sprite.blendMode = blend === "add" ? "add" : "normal";
    container.addChild(sprite);
    return {
      show(key, frame, x, y, w, h, depth) {
        const t = textureOf(key, frame);
        if (!t) {
          sprite.visible = false;
          return;
        }
        if (sprite.texture !== t) sprite.texture = t;
        sprite.position.set(x, y);
        sprite.setSize(w, h);
        if (sprite.zIndex !== depth) sprite.zIndex = depth;
        sprite.visible = true;
      },
      hide() {
        sprite.visible = false;
      },
      destroy() {
        sprite.destroy();
      },
    };
  };
}
