# shanshui-screensaver

An infinite, procedurally generated Chinese landscape scroll as a Wayland screensaver — a Rust port of Lingdong Huang's [shan-shui-inf](https://github.com/LingDong-/shan-shui-inf), drawn with `tiny-skia` straight onto a Wayland surface.

The scene never repeats and is never the same twice: each run picks a seed, and the landscape is generated lazily to the right as it scrolls past. One fullscreen window per monitor, each with its own seed. The process exits on any key, mouse button, scroll, pointer movement, focus loss, or SIGTERM.

## Install

```bash
make install        # cargo build --release, binary to ~/.local/bin/shanshui
```

Needs a Rust toolchain and a Wayland compositor. Run it with `shanshui`; it fullscreens itself on every monitor. `make uninstall` removes it.

## Flags

| Flag | Default | Meaning |
|---|---|---|
| `--seed <string>` | current time | scene seed; the same seed reproduces the same landscape |
| `--speed <px/s>` | `24` | scroll speed, snapped down to the nearest regular cadence (see below) |
| `--fps <n>` | `30` | upper bound on steps per second, for when `--speed` is very fast |
| `--zoom <f>` | fit height | pixels per world unit |
| `--fade <secs>` | `1.5` | how long the scene takes to fade in over the paper at the start; `0` shows it at once |
| `--class <name>` | `org.omarchy.screensaver` | Wayland app-id, for compositor window rules |
| `--windowed` | — | a single normal window instead of fullscreen |
| `--png <file>` | — | render one frame and exit |
| `--width`/`--height`/`--x` | `1920`/`1080`/`0` | size and scroll offset for `--png` and `--bench` |
| `--bench <secs>` | — | run the pipeline headlessly and report CPU use |

```bash
shanshui --png scene.png --seed 20260101 --x 9000
```

## Scrolling

Motion is paced by the compositor, not by a timer. Each window presents on every
Wayland frame callback and moves the scene by exactly one pixel every *k*
callbacks, where *k* comes from that monitor's refresh rate. The step is
therefore identical every time, which is what makes a slow pan look even; a
timer that advances a fractional number of pixels per tick lands on irregular
pixel boundaries and judders, especially when its period does not divide the
refresh period.

Nothing is shown until a monitor's first screenful is ready; the scene then
fades in over the plain paper, one blended frame per refresh for `--fade`
seconds while the scroll already runs underneath.

The cost is that `--speed` is snapped to `refresh / k` — the fastest regular
cadence that does not exceed what was asked. On a 60 Hz monitor, `--speed 24`
becomes 1 px every 3 refreshes, i.e. 20 px/s. Monitors are paced independently,
so a mixed-refresh setup stays even on both.

## Performance

The world is rasterized once into 512 px-wide tiles, on a background thread per
monitor that generates chunks sequentially and then rasterizes a batch of tiles
across cores. Per frame the only work is a blit.

| | 2256×1504 | 1920×1080 |
|---|---|---|
| CPU (one core) | ~11% | ~7% |
| peak RSS | 57 MB | 44 MB |
| blit | 2.6 ms | 1.5 ms |

CPU scales with `--speed`, not with the refresh rate: a one-pixel step has to be
written into both of the surface's buffers, so the blit runs twice per pixel of
travel and the frames in between are presented untouched. Halving `--speed`
halves the CPU. Measure your own with `--bench`.

The blit is memory-bandwidth bound rather than compute bound — splitting it
across threads buys about 20% of wall time for two to three times the CPU — so
it stays on one thread. Rasterizing tiles is the opposite and is parallel.

## Fidelity

The generator is a function-by-function port, including the original's f64 Blum-Blum-Shub PRNG and p5.js Perlin noise, so a seed produces the exact same scene as the JavaScript. `cargo test` checks that against figures taken from running the upstream `index.html` under node — chunk count, polyline count and coordinate checksum all match.

Three deliberate differences:

- The `Pizza Hut` sign the original sometimes puts on a one-storey roof is skipped; there is no text rendering (the random draw that decides it is still made, so the scene is otherwise identical).
- Elements are drawn whenever their geometry overlaps the view rather than when their anchor is within one chunk of it, so nothing pops in at the edges.
- The paper texture uses its own PRNG stream instead of continuing the scene's.

## Using it as the Omarchy screensaver

[Omarchy](https://omarchy.org) starts its screensaver through `omarchy-launch-screensaver`, both from the idle timeout and from the "Screensaver" menu entry, and fullscreens any window with the app-id `org.omarchy.screensaver`. This repo ships a drop-in replacement for that launcher:

```bash
make install-omarchy    # binary + shim to ~/.local/overrides/bin/omarchy-launch-screensaver
```

The shim only wins if `~/.local/overrides/bin` comes first in the session PATH; Omarchy appends `~/.local/bin` *after* `/usr/bin` on purpose, so a shim there never works. Add the override dir once in a uwsm env file, then log out and back in:

```bash
cat > ~/.config/uwsm/env.d/30-overrides-path <<'EOF'
case ":$PATH:" in
  *":$HOME/.local/overrides/bin:"*) ;;
  *) export PATH="$HOME/.local/overrides/bin${PATH:+:$PATH}" ;;
esac
EOF
```

The shim keeps the stock behaviour (exits early if a screensaver is already running, honours the `screensaver-off` toggle unless called with `force`, launches through `hyprctl dispatch`) and hands over to the stock launcher if the binary is missing. Set `SHANSHUI_ARGS` to pass flags through it. Nothing under `/usr/share/omarchy/` is touched; `make uninstall` removes the binary and the shim.

## Credit

The landscape itself is [shan-shui-inf](https://github.com/LingDong-/shan-shui-inf) by [Lingdong Huang](https://lingdong.works), MIT licensed. This repository is the port and the screensaver plumbing around it. See `LICENSE`.

One exception: the Perlin noise in `src/noise.rs` descends from [p5.js](https://github.com/processing/p5.js) via shan-shui-inf and stays under the LGPL 2.1 (`LICENSE-LGPL-2.1`); the rest of the crate is MIT. Cargo declares this as `MIT AND LGPL-2.1-or-later`.
