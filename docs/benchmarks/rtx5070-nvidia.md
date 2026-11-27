## NVIDIA report: RTX 5070 12 GB / Ryzen 7 7800X3D / 32 GB, `2c5a395`

This is a community benchmark on an NVIDIA RTX card, which the README asks for. Fields as the README requests:

- **Commit:** `2c5a395` (main), built from source with `cargo build --locked --release -p victory`.
- **GPU and driver:** RTX 5070 12 GB, driver 617.42. CPU: Ryzen 7 7800X3D. RAM: 32 GB DDR5 + 64 GB page file. OS: Windows 11 26300.
- **Settings:** `scripts/rdna-preset.json` "base" + "fast", applied the way `scripts/play-rdna.ps1` applies them. I launched through my own wrapper (RAM guard + log), windowed, and not through `launch.bat` / `launcher.py`.
- **Scene:** prologue save starting at the arrest cutscene.

### Results

| Run | Console profile | Output | Duration | Result |
|---|---|---|---|---|
| 1, fast preset as shipped | Series S | 1080p | ~6 min | **Black picture** for the whole run (audio plays, the game runs). 1.7–3.6 fps, 820–1,230 draws a frame with 420–770 dropped "without a host pipeline". Game peak 21.2 GB private; free RAM down to 0.3 GB. |
| 2, fast preset + `PORT_CONSTANTS=0` + `VIEW_BYTES=0` | Series X | 4K | ~3 min in the world | **Picture shows.** 1.6–2.4 fps, **GPU-bound:** 380–560 ms a frame on the GPU, card ~94% busy; preparing ~40 ms, copy-back ~30 ms; ~1,500 draws. Game 21.5 GB private, ~1.5 GB free. |

In both runs there was no exception and no device removal; I stopped both. In run 2 I turned off `PORT_CONSTANTS` and set `VIEW_BYTES=0` up front because of two earlier NVIDIA reports:
- `PORT_CONSTANTS=1` makes this kind of card read system-memory heaps slowly: wait for the card 38 → 119 ms, 6.9 fps (fxceless/victory-fork#1379, and TaynanGT/Vice-Tay#115 on an RTX 3080 Ti).
- On avatarmanz `main`, `VIEW_BYTES` made recording take ~1,000 ms a frame on NVIDIA until it was set to `0` (fxceless/victory-fork#1379).

### What it looks like from here

- **Series S black picture.** This matches what I saw on other trees without Series S second-plane composition. The NVIDIA forks add `VICTORY_COMPOSE=1` for that, and keep the title on its own upscaler (`VICTORY_OWN_UPSCALER=1`), because otherwise it runs FSR 4.
- **Series X GPU time.** On the same PC, the NVIDIA-focused trees that skip the RT resolve/traced-shadow programs (`VICTORY_SKIP_PIPELINES` with `rt_off_hashes.txt`) and stay off FSR 4 sit at 5–7 ms of GPU work a frame at 720p internal. Here the card is the limit, at 400+ ms. The likely costs are those RT programs and the FSR 4 compute passes at 4K output; #1379 measured FSR 4 alone keeping a 5080 94% busy. I haven't split which one dominates here.
- **RAM:** with `INPLACE=2` + `PORT_BUFFERS=1`, the game's private memory was about the same as the other trees on this PC (~20–23 GB), so 32 GB is at the edge.

### Build note

`--profile diag --features diagnostics` doesn't compile at `2c5a395`. In `crates/victory-gpu/src/descriptor/host.rs`, `slot_story` (diagnostics only) calls `.get(..)` on `DynamicBits`, which has no `get` since the packed-bitset change (13e1f5e). Something like this would fix it:

```rust
let i = (index - host.count) as usize;
let bits = &host.next.lock().unwrap_or_else(|e| e.into_inner()).1;
let taken = (i < bits.len).then(|| bits.words[i / 64] >> (i % 64) & 1 == 1);
```

(I didn't test that snippet; I built release instead.)

*AI-assisted (Claude Code). Numbers and log lines only, no game data or images.*
