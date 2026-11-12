import type { FxLike } from "./engine";
import { snap } from "./pixels";

export type EffectState = "playing" | "stopping" | "done" | "destroyed";

export interface FollowTarget {
  x: number;
  y: number;
  active: boolean;
}

/** One playing effect: position, follow target, stop/complete/destroy state. No Phaser, no rendering. */
export class EffectCore {
  state: EffectState = "playing";
  private target: FollowTarget | null = null;
  private dx = 0;
  private dy = 0;

  constructor(
    private readonly fx: FxLike,
    private x: number,
    private y: number,
  ) {}

  follow(target: FollowTarget | null, dx: number, dy: number): void {
    this.target = target;
    this.dx = dx;
    this.dy = dy;
  }

  tick(dt: number): "running" | "completed" {
    if (this.state === "done" || this.state === "destroyed") return "running";
    if (this.target) {
      if (this.target.active) {
        this.x = this.target.x + this.dx;
        this.y = this.target.y + this.dy;
      } else {
        this.target = null;
        this.stop();
      }
    }
    this.fx.update(dt);
    if (this.fx.done()) {
      this.state = "done";
      return "completed";
    }
    return "running";
  }

  stop(): void {
    if (this.state !== "playing") return;
    this.state = "stopping";
    this.fx.stop();
  }

  destroy(): void {
    if (this.state === "destroyed") return;
    this.state = "destroyed";
    this.fx.free();
  }

  topLeft(): { x: number; y: number } {
    return { x: snap(this.x) - this.fx.originX(), y: snap(this.y) - this.fx.originY() };
  }
}
