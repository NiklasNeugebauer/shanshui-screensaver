# omarchy-shanshui

An infinite, procedurally generated Chinese landscape scroll as the [Omarchy](https://omarchy.org) screensaver — a Rust port of Lingdong Huang's [shan-shui-inf](https://github.com/LingDong-/shan-shui-inf), drawn with `tiny-skia` straight onto a Wayland surface.

The scene never repeats and is never the same twice: each run picks a seed, and the landscape is generated lazily to the right as it scrolls past.

## Install

```bash
make install
```

That builds with `cargo` and copies two files into `~/.local/bin`:

- `shanshui` — the binary
- `omarchy-launch-screensaver` — a shim that shadows the stock launcher

`make uninstall` removes both. Nothing under `/usr/share/omarchy/` is touched.

Needs a Rust toolchain; via mise that is `mise use -g rust@latest`.

## How it hooks into Omarchy

The idle service runs `bash -lc omarchy-launch-screensaver`, and a login shell resolves `~/.local/bin` before `/usr/share/omarchy/bin`, so the shim is picked up with no further configuration. It keeps the stock behaviour: it exits early if a screensaver is already running, honours the `screensaver-off` toggle unless called with `force` (what the "Screensaver" menu entry does), and launches through `hyprctl dispatch`. If the binary is missing it hands over to the stock launcher.

The binary opens one fullscreen window per monitor with the app-id `org.omarchy.screensaver`, which is what the Hyprland rules and the idle service's window watcher look for. `pkill -f '[o]rg.omarchy.screensaver'` still stops it. The whole process exits on any key, mouse button, scroll, pointer movement, focus loss, or SIGTERM.

Each monitor gets its own seed, so they show different landscapes.

## Flags

| Flag | Default | Meaning |
|---|---|---|
| `--seed <string>` | current time | scene seed; the same seed reproduces the same landscape |
| `--speed <px/s>` | `24` | scroll speed in logical pixels per second |
| `--fps <n>` | `30` | frame cap; frames where nothing moved by a whole pixel are skipped anyway |
| `--zoom <f>` | fit height | pixels per world unit |
| `--class <name>` | `org.omarchy.screensaver` | Wayland app-id |
| `--windowed` | — | a single normal window instead of fullscreen |
| `--png <file>` | — | render one frame and exit |
| `--width`/`--height`/`--x` | `1920`/`1080`/`0` | size and scroll offset for `--png` and `--bench` |
| `--bench <secs>` | — | run the pipeline headlessly and report CPU use |

```bash
shanshui --png scene.png --seed 20260101 --x 9000
```

Set `SHANSHUI_ARGS` to pass flags through the shim.

## Performance

The world is rasterized once into 512 px-wide tiles on a background thread that stays ahead of the window; each frame is a blit. Generation costs about 1% of a core; the blit is memory-bandwidth bound, so it scales with the pixel count:

| Resolution | CPU (one core) |
|---|---|
| 1920×1080 | ~8% |
| 2560×1440 | ~10% |
| 3840×2160 | ~21% |

`--fps` scales that down roughly linearly. Measure your own with `--bench`.

## Fidelity

The generator is a function-by-function port, including the original's f64 Blum-Blum-Shub PRNG and p5.js Perlin noise, so a seed produces the exact same scene as the JavaScript. `cargo test` checks that against figures taken from running the upstream `index.html` under node — chunk count, polyline count and coordinate checksum all match.

Three deliberate differences:

- The `Pizza Hut` sign the original sometimes puts on a one-storey roof is skipped; there is no text rendering (the random draw that decides it is still made, so the scene is otherwise identical).
- Elements are drawn whenever their geometry overlaps the view rather than when their anchor is within one chunk of it, so nothing pops in at the edges.
- The paper texture uses its own PRNG stream instead of continuing the scene's.

## Credit

The landscape itself is [shan-shui-inf](https://github.com/LingDong-/shan-shui-inf) by [Lingdong Huang](https://lingdong.works), MIT licensed. This repository is the port and the screensaver plumbing around it. See `LICENSE`.
