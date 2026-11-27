## AMD report: RX 7900 XTX / i7-13700KF / Windows 10, `2c5a395` against avatarmanz `c9c932c`

A community benchmark on an RX 7900 XTX, the card the README asks for. Fields as the README requests:

- **Commits:** this repo at `2c5a395` + avatarmanz/VIctory-7900XTX#165 (Agility SDK runtime export) + the `slot_story` build fix (see "Build note"). Compared with avatarmanz/VIctory-7900XTX `main` at `c9c932c` (renderer port 7 merged) + the same #165. Both built with `--profile diag --features diagnostics`. Neither tree has the port 1 ordering yet (`VICTORY_PORT_CONSTANTS_ORDER`, avatarmanz `7ab13ed`).
- **GPU and driver:** RX 7900 XTX, driver 32.0.32015.7029. CPU: i7-13700KF. OS: Windows 10 Pro 22H2 (19045), Agility SDK 1.620 runtime next to the executable.
- **Console profile:** Series X. **Output:** 4K with `VICTORY_RENDER_SCALE=0.5`, and 2560x1440.
- **Session:** 25 min a run, same save and same scripted input in every run. No device removal and no `ERROR` line in any run; I stopped each at 25 min.
- **Settings:** both sides the same base switches and renderer ports 1, 2, 5, 6, 7 (`VICTORY_PORT_CONSTANTS`, `_DESCRIPTORS`, `_CBV`, `_GUARD`, `_JOURNAL`). The RDNA side adds the rest of the "fast" preset as of `2c5a395`: `VICTORY_PORT_BUFFERS=1`, `VICTORY_INPLACE=2`, `VICTORY_ASK_SPREAD=1`, `VICTORY_RESIDENCY=1`, `VICTORY_SYNC_FINE=1`, `VICTORY_CARD_BUFFERS_MIN_MB=1`.

### Results

Frame time is the mean of the 5-second host lines with 3,000 to 4,999 draws (125 to 144 lines a run, about 4,800 draws on average). Prep is the CPU preparation of a frame from the same lines. Card is the median of the 3D engine busy samples (Windows performance counter, a sample about every 8 s) above 20%, so loading and menus are left out.

| Output | RDNA `2c5a395`, fast preset | avatarmanz `c9c932c`, same ports |
|---|---:|---:|
| 4K, render scale 0.5 | 35.0 ms, prep 23.2 ms, card 97% | 33.8 ms, prep 28.1 ms, card 92% |
| 2560x1440 | 33.5 ms, prep 24.0 ms, card 94% | 32.8 ms, prep 27.6 ms, card 88% |
| 2560x1440, without `PORT_BUFFERS` and `INPLACE` | 35.2 ms, prep 27.2 ms | |

### Shader and pipeline caches

Every run started with warm caches. Both trees read the same `victory.toml`, so they share one cache folder: the shader translations and the host pipeline library (`pipelines/host-pipelines.bin`) were already filled by earlier runs on the same save that day. The counters below are the last ones each run logged (`shaders from the cache and translated so far`, and `loaded` / `made` of the pipeline library). I did not check the driver's own disk cache.

| Run, in the order they ran | Shaders from the cache / translated in the run | Pipelines loaded from the library / compiled in the run | State |
|---|---:|---:|---|
| avatarmanz, 4K | 3103 / 2 | 2839 / 3 | warm |
| avatarmanz, 1440p | 3105 / 7 | 2844 / 4 | warm |
| RDNA, 4K | 3115 / 4 | 2853 / 0 | warm |
| RDNA, 1440p | 3119 / 1 | 2863 / 1 | warm |
| RDNA, 1440p without the two switches | 3120 / 130 | 2871 / 94 | warm for the first 13 min, then partly cold |

In the last run the game reached scenes the cache had not seen yet: 92 of its 94 pipeline compiles came between 13.7 and 17.1 min in, and the translated shaders went from 1 to 130 by 17.9 min. 11 of its 144 measured lines fall between 13.7 and 18.5 min; without them the run's mean stays at 35.2 ms (prep 27.3 ms).

### What it looks like from here

- On the RDNA tree, `VICTORY_PORT_BUFFERS=1` + `VICTORY_INPLACE=2` take 3.2 ms off the CPU preparation at 1440p (27.2 -> 24.0 ms) and the frame gets 1.7 ms faster (35.2 -> 33.5 ms).
- In the cross-build comparison, CPU preparation was faster (3.6-4.9 ms less), GPU utilization was higher (97% against 92% at 4K, 94% against 88% at 1440p), and overall frame time was slightly slower (0.7 ms at 1440p, 1.2 ms at 4K).
- At 4K the same two switches, on the same RDNA build, take 4.9 ms off the CPU preparation; the frame-time difference is small (see the next section).

### 4K A/B on the same RDNA build: `VICTORY_PORT_BUFFERS` and `VICTORY_INPLACE`

The same build as above (`2c5a395` + #165 + the `slot_story` fix, built as `7c3616d`), output 4K with render scale 0.5, ports 1, 2, 5, 6, 7 and the rest of the fast preset unchanged; only `VICTORY_PORT_BUFFERS=1` + `VICTORY_INPLACE=2` against both unset. `VICTORY_AB` cannot flip them on this tree (both are read once at start), so two 25-minute runs back to back on the same save, `warm --used` before each. Unlike the runs above, the boat was driven this time (scripted, following the mission marker), so the scenes are not the same in the two runs.

| 5-second lines with 3,000-4,999 draws | BUFFERS + INPLACE | both unset |
|---|---:|---:|
| lines (samples) | 44 | 108 |
| frame mean / median / p90 | 36.2 / 33.8 / 40.3 ms | 37.1 / 36.3 / 40.0 ms |
| CPU preparation | 21.2 ms | 26.1 ms |
| GPU utilization (median) | 96% | 91% |
| shader misses / pipelines compiled by the end | 414 / 82 | 3 / 6 |

With the two on, CPU preparation was 4.9 ms faster and GPU utilization higher; the frame was 0.9 ms faster on the mean and 2.5 ms on the median, with p90 the same. The "on" run went partly cold (the drive reached places the cache had not seen: 414 shader misses and 82 pipeline compiles) and has fewer measured lines, so the frame-time difference should be read as small, and the preparation difference as the solid part, in line with the 1440p pair above. Before the "off" run the `warm --used` pass found 83,574 pipelines known to the book, 3,542 of them still to make, which is why that run compiled almost nothing during play.

### Port 1 ordering in the preset

The "fast" preset now sets `VICTORY_PORT_CONSTANTS_ORDER=0` (before `8ba4f34` it was `VICTORY_PORT_CONSTANTS_ORDER_ALONE=0`). avatarmanz turned the ordering on in `7ab13ed` because without it trees and palms drop out, and measured that the batch of `ORDER_ALONE=0` was not enough (run pop-15, `3e04ca0` / `15c3d0e`). An option in between is avatarmanz/VIctory-7900XTX#438 (`VICTORY_PORT_CONSTANTS_ORDER_BATCH=1`, off by default), which batches only executions with nothing but state between them: in an A/B run on the avatarmanz tree on my 7900 XTX at 4K it took the median from 57.0 to 44.2 ms with about the same flicker. It is not a general win yet: an RTX 5090 replay under Proton got slower with it (comment on #438).

### Build note

The diagnostics build at `2c5a395` needs the `slot_story` fix that `rtx5070-nvidia.md` describes; I sent it tested as a separate PR (branch `rdna/fix-diag-dynamicbits`, reading a slot through `first_taken`).

*AI-assisted (Claude Code). Numbers from my run logs only, no game data or images.*
