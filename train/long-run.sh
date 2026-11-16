#!/bin/bash
# A training run that outlives its shell: builds the rules, runs the tests, then trains `RUN` (extra arguments go to
# ppo.py), resuming from runs/RUN/last.pt whenever the trainer dies, up to 6 tries. Prints TRAIN-DONE at the end.
#   nohup caffeinate -is ./long-run.sh frames > ~/train-frames.log 2>&1 < /dev/null &
set -u
RUN=${1:?usage: long-run.sh RUN [ppo.py arguments]}
shift
cd "$(dirname "$0")" || exit 1
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_BUILD_DIR=${CARGO_BUILD_BUILD_DIR:-/tmp/pixfx-build-train} CARGO_INCREMENTAL=0
echo "SETUP $(date)"
uv sync -q || { echo "FAIL uv sync"; exit 1; }
uv run maturin develop --release -m ../crates/pixfx-duel-py/Cargo.toml 2>&1 | tail -2
uv run pytest -q 2>&1 | tail -3
uv run pytest -q >/dev/null 2>&1 || { echo "FAIL pytest"; exit 1; }
for try in 1 2 3 4 5 6; do
  if [ -f "runs/$RUN/last.pt" ]; then
    echo "TRAIN resume try $try $(date)"
    uv run python ppo.py --run "$RUN" --resume "runs/$RUN/last.pt" "$@"
  else
    echo "TRAIN start try $try $(date)"
    uv run python ppo.py --run "$RUN" "$@"
  fi
  code=$?
  echo "EXIT $code $(date)"
  [ $code -eq 0 ] && { echo "TRAIN-DONE"; exit 0; }
  sleep 10
done
echo "FAIL gave up"
exit 1
