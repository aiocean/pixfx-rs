import { snap } from "./pixels";

export type EffectState = "playing" | "stopping" | "destroyed";

export interface FollowTarget {
  x: number;
  y: number;
  active: boolean;
}

/** One playing effect's place and state: position, follow target, playing/stopping/destroyed. No Phaser, no engine. */
export class EffectCore {
  state: EffectState = "playing";
  private target: FollowTarget | null = null;
  private dx = 0;
  private dy = 0;

  /** @param origin where the effect's origin sits on its canvas. */
  constructor(
    private readonly origin: { originX: number; originY: number },
    private x: number,
    private y: number,
  ) {}

  follow(target: FollowTarget | null, dx: number, dy: number): void {
    this.target = target;
    this.dx = dx;
    this.dy = dy;
  }

  /** Place the effect at a world position; it stops following any target. */
  moveTo(x: number, y: number): void {
    this.target = null;
    this.x = x;
    this.y = y;
  }

  /** Move to the follow target. Returns false when the target went inactive: it is dropped and the effect stays where it was. */
  track(): boolean {
    if (!this.target) return true;
    if (!this.target.active) {
      this.target = null;
      return false;
    }
    this.x = this.target.x + this.dx;
    this.y = this.target.y + this.dy;
    return true;
  }

  /** Playing → stopping; false in any other state. */
  stop(): boolean {
    if (this.state !== "playing") return false;
    this.state = "stopping";
    return true;
  }

  /** → destroyed; false if it already was. */
  destroy(): boolean {
    if (this.state === "destroyed") return false;
    this.state = "destroyed";
    return true;
  }

  topLeft(): { x: number; y: number } {
    return { x: snap(this.x) - this.origin.originX, y: snap(this.y) - this.origin.originY };
  }
}
