# VIctory on an RX 7900 XTX

A fork of [VIctory](https://github.com/Marvisak/VIctory), a compatibility layer
that runs an Xbox Series title natively on Windows from files the user supplies.

## The goal

**A stable 30 fps or more without ray tracing, then as high a frame rate and as
low frame times as the PC allows**, on an AMD RX 7900 XTX first and on PCs with
32 GB (later 16 GB) of RAM.

The way there is **porting, not emulating**: patching the title's renderer so it
behaves the way a PC renderer does, rather than having the layer watch and copy
the console's shared memory every frame. Shaders are translated ahead of time
into a versioned store that is kept between builds. Ray tracing stays optional
and off by default. The picture comes after the frame rate: texture pop-in and
lighting that is too bright or too dark when walking indoors are next in line.

**This repository contains no game files, code or assets.** You need your own
copy of the game. It is not affiliated with or endorsed by the game's developer
or publisher.

## Where it stands (2026-11-26)

Tested on one machine only: Windows 11, Intel i9-12900K, 64 GB RAM, Radeon
RX 7900 XTX, ray tracing off.

| | |
|---|---|
| City street, Series S profile, renderer patches 1, 2, 5, 6 and 7 on | about 45 ms a frame (about 22 fps, 44.9 ms in run merge-5) |
| Same, patches 1, 2, 5 and 6 | 45.6 ms, p95 47 ms |
| Same, patches 1, 2 and 5 | 47.3 ms (about 21 fps), p95 49 ms |
| Series X profile, patches 1+2 | about 53 ms (about 19 fps) |
| Same scene before the patches | 57-75 ms (13-17 fps) |
| Shaders the title uses that translate | all of them (25,790 of 25,985; the rest are unused geometry shaders) |
| Memory | about 30 GB of private memory, 10.5 GB of VRAM: **64 GB of RAM recommended** for now |

It started at 280 ms a frame in the same scenes three days earlier. What sets
the frame now is the one thread that submits the title's work to the card
(about 40 ms); the card itself needs about 36 ms and now waits about 7 ms a frame with nothing queued.

### Renderer patches (off by default while they are tested)

| Switch | What it does |
|---|---|
| `VICTORY_PORT_CONSTANTS=1` | The title's 174 MB constants heap is read by the card where the title writes it: no uploads, no fills |
| `VICTORY_PORT_DESCRIPTORS=1` | The title's descriptor-table stores are patched (byte-checked) to report the slots they write, instead of a scan of ~600,000 slots a frame |
| `VICTORY_PORT_CBV=1` | Translated shaders read constant buffers in place as raw bytes (~22,000 fills a frame to none; -4.7 ms) |
| `VICTORY_PORT_GUARD=1` | The write guards' lock is shared by faults and refreshes, with a claim per 64 KiB block; the submission's planning runs beside it (title threads' waits 29 ms to ~2 ms a frame; -2 ms) |
| `VICTORY_PORT_JOURNAL=1` | The heaps' records are asked for at the title's submit, and the journal's notes are split into 64 parts with locks of their own (the submit thread's waits ~5 ms + 1 ms to ~0.3 ms; -1 to -3 ms) |

### Other switches

| Switch | What it does |
|---|---|
| `--console series-s` | The Series S profile: 1080p output, about 5 ms faster, a softer picture |
| `VICTORY_OUTPUT=1920x1080` | The title renders to that output size itself (Series X profile; about 16 ms less card time) |
| `VICTORY_FRAME_INTERVAL_MS=16.667` | Lets the title go above 30 fps (menus and videos reach 60; the game's clock stays right) |
| `VICTORY_AMBIENT_SETTLE` | How fast light settles after walking indoors (on by default, `250,3`; `0` turns it off) |
| `VICTORY_TEXTURE_STORE=1` | Converted textures kept between runs (about 8 ms a frame for now: off) |
| `VICTORY_DLSS=1` (with `VICTORY_FSR_4K`) | The title's FidelityFX upscale is done by NVIDIA DLSS Super Resolution through Streamline, loaded at run time (signature checked, nothing proxied, no frame generation); the DLSS mode follows the render size the title picks. Without Streamline or DLSS support, the PC FidelityFX path as before. Not measured yet |
| `VICTORY_STREAMLINE_DIR` | The folder with Streamline's `sl.interposer.dll` and `sl.dlss.dll`. Without it `run` looks in `%LOCALAPPDATA%\victory\nvidia-sdk\Streamline\bin\x64`, then beside the program and the title (their `nvidia-sdk\Streamline\bin\x64`, then the folder itself); none found, it asks once [Y/n] (only at a terminal; `n` is remembered) to fetch them, otherwise DLSS stays off |
| `victory get-dlss` | Fetches NVIDIA's DLSS files (Streamline 2.14.16, about 70 MB, NVIDIA's licences, saved beside them) from NVIDIA's own release at github.com/NVIDIA-RTX/Streamline into `%LOCALAPPDATA%\victory\nvidia-sdk` (`scripts\get-nvidia-sdks.ps1`, embedded; takes only the DLLs out of the zip by range requests). Clears an earlier "no" |
| `VICTORY_DLSS_PRESET` | DLSS preset: `default`, `e`, `f`, `j`, `k`, `l`, `m` |
| `VICTORY_DLSS_JITTER`, `VICTORY_DLSS_MV` | `x,y` multipliers of the jitter and motion vectors given to DLSS (default `1,1`), for sign checks |
| `VICTORY_DLSS_REACTIVE=0` | The title's reactive mask is not given to DLSS |
| `VICTORY_STREAMLINE_LOG=<folder>` | Streamline's verbose log in that folder |
| `victory upscaler-check [--device] [--output 3840x2160]` | Loads Streamline and says whether this GPU runs DLSS; with `--device`, DLSS upscales stand-in textures three times |

### Switches merged from fxceless/victory-fork (2026-11-26)

Off by default unless noted. The numbers are their author's, on their machine (an RTX 4070 and the
first boat mission); their README is the source of each.

| Switch | What it does |
|---|---|
| `VICTORY_PARALLEL_REFRESH=1` | A submission's texture checks run side by side on the helper threads: 8.7 and 9.4 ms less of the preparing thread's work a frame |
| `VICTORY_SETTLE_ROOM=1` | Flights get more room before they are settled: the preparing thread waits 4.6 ms a frame for the card instead of 17.7 |
| `VICTORY_HEAP_WORKER=1` | The texture pass on a thread of its own (pull request #1007): 6.2-6.7 to 8.65-9.53 world fps |
| `VICTORY_LOOKUP_CACHE=1` | Asks for a buffer's host copy answered without the lock (pull request #1237): about 14 ms a frame |
| `VICTORY_FILLS_PLAIN=1` | With `VICTORY_COPY_PLAIN=1`: a list's fills under 256 bytes copied in place (estimated 3 ms a frame, not measured) |
| `VICTORY_STEP_INDEX=1` | A step's copy found by an index instead of a lock and a search (estimated 1.5-4 ms a frame, not measured) |
| `VICTORY_LIGHT_REFRESH=1` | With `VICTORY_ABSENT_ONCE=1 VICTORY_ABSENT_MS=told`: a refresh of blocks known unwritten answered with no lock and no kernel call (not measured) |
| `VICTORY_QUEUE2=1` | Fetches of what shaders wrote go to a copy queue (copy-back wait about 125 to 7 ms a frame) |
| `VICTORY_INDIRECT_GATHER=1` | An indirect record that misses its window gets a scratch slot, clamped in one pass per list |
| `VICTORY_TEXTURE_BACK=1` | Textures of the ocean's kinds that shaders write go back to title memory (pull request #999): boats float |
| `VICTORY_BVH_NO_HIT=1` | Ray-traced passes with nothing built answer "no hit" at the first node test (translator change: cache rebuilt once) |
| `VICTORY_DISPATCH_SPREAD` | Compute dispatches of more than 65,535 groups spread across z (after upstream pull request #2312; on there, costs nothing measurable) |
| `VICTORY_TARGET_VARIANTS=0` | Turns off the pipeline made for the bound render-target format when a pipeline declares another (on by default) |
| `VICTORY_REASK_BLANK=1` | Texture slots that read zeros for good are asked again (diagnostics builds) |
| `VICTORY_REAPER`, `VICTORY_BAKE_SOURCES`, `VICTORY_RETIRE_LEAN` | Host objects let go on a thread of their own; skin-bake textures uploaded before the draw (pull request #1264); slot rewrites let go at once (measured slower) |
| `VICTORY_WIDE_REFRESH`, `VICTORY_VIEW_KNOWN`, `VICTORY_CARD_HASH_WIDE`, `VICTORY_NEAR_WRITES`, `VICTORY_SYNC_BESIDE`, `VICTORY_HEAP_SYNC_CUT` | Measured at no gain there, left off |
| `VICTORY_DENY_PIPELINES`, `VICTORY_CARD_METER`, `VICTORY_CARD_METER_PIPELINES`, `VICTORY_FPS_COUNTER=0`, `VICTORY_VIDEO_LOOK=0`, `VICTORY_RENDERDOC=<dll>` | Instruments: a pipeline never built; the card's time by pipeline; no fps in the caption; no video-memory line a second; RenderDoc hook |
| `VICTORY_REMAP_METER=0` | The page-mapping meter of the frame log (counts and lengths always) stops reading thread cycles around each map and unmap call (on by default) |
| `VICTORY_CLZERO_LOOP15=0` | The title's 15-byte `clzero` loop is no longer emulated whole in one fault (on by default) |
| `VICTORY_DISK_CORES=slow` | DirectStorage workers asked to keep to the slow cores (not measured) |

### Merged from fxceless/victory-fork (2026-11-27, 00:30 UTC)

Upstream's notes of commit b1d777a8 (their numbers, on their machine):

- **Measured (one run, 3,300 to 4,100 draws): usual world frame 113 ms, the card 71 ms** with the
  wide raw loads guarded at a view's end, the fast-math flag off, the 4 MiB window grid, joins of 4
  parts and 48 lists, texture room 90 % with 3,072 MB kept for targets, and the kept thread pool.
- **Water:** the black and grey patches came from forcing the two water programs into the wave-wide
  form. Left out, open water shows waves and the sun's path.
- **Translator (shader caches must be rebuilt; keep `early-pipelines.txt`):** wide raw loads are on by
  default and fall back to one load a dword where four would pass a heap view's end; `ds_bpermute`
  reads within its own 32-lane half (after Belobug PR #363); pixel-shader quad swizzles read through
  quad operations (after Belobug 03629f3f); a helper lane's first-lane read is its own value (after
  upstream 0601955); NaN tests are made on the bits; `VICTORY_SHADER_FAST_BITS` picks the fast-math
  flags (still off: all flags gave black blotches); ray node tests answer "no hit" by default
  (`VICTORY_BVH_NO_HIT=0` off).
- **Merged by hand:** #1397 (paired wave64 lowering, off; per program with `VICTORY_WAVE_PAIRS_ONLY`;
  both water programs are refused by it today: "a floating-point reduction without an exact paired
  tree"), #1399 (one GPU-write step per range per list), #1400 (lock-free empty slots, one-block
  write faults, root constants on the stack), and a switch for #1392 (`VICTORY_NO_WAIT_PARTIAL=0`;
  here it also turns off this fork's own fill of such textures, `VICTORY_TEXTURE_FILL`).
- **New, off by default:** `VICTORY_CARD_FILLS` (constant buffers and copies where shaders wrote, on
  the card; measured worse: usual frame +11 ms, card +6 ms), `VICTORY_MESH_LAUNCH` (#1381's idea,
  from its HLSL), and a kept pool for the texture checks ahead (`VICTORY_KEPT_THREADS=0` off).

Here the descriptor heap's slots, the guard's write faults and the heap locks are this fork's own
(#1400's empty-slot and one-block changes are not taken), and the pipeline workers stay at the lowest
priority (`VICTORY_PIPELINE_LOWEST=normal`: at normal priority, as upstream). This fork has no
`fill_ahead`: `VICTORY_CARD_FILLS` learns where shaders wrote from this fork's own landings, and the
kept pool serves the work spread over threads. Upstream's quad reads replace this fork's (same
helper-lane fix); DPP8 inside a quad keeps this fork's per-lane quad read.

### Merged from fxceless/victory-fork (2026-11-27, 01:20 UTC)

Upstream's notes of commits 3221ac42, 0e006f00 and 1f5d17a6 (their numbers, on their machine):

- **The flights are no longer settled for title bytes changed under shader writes
  (`VICTORY_MASKED_PATCH`, on by default, `0` off, `alt` twenty seconds each way):** a page with more
  changed runs than the merge's cap goes to the card as a masked patch (offset, byte mask, value per
  dword) after the flights. In an on/off split: waiting for the card 1 to 2 ms against 11 to 32, the
  usual frame 10 to 15 ms shorter.
- **Indirect mesh draws are launched by an amplification stage (`VICTORY_MESH_LAUNCH`, now on by
  default, `0` off; after pull request #1381):** no clamp pass before each record; the card 5 ms less a
  frame in an on/off split. The translated mesh shaders follow the same switch (shader caches must be
  rebuilt). **In this fork it is off by default** (`=1` turns it on): on the RTX 4080 with this
  fork's set the world ran at 14.35 fps with it and 17.96 without (warm pipeline library, 195-300 s).

Here the patch is made where this fork merges a page under shader writes (the pages are merged on
several threads): a page whose copies of their own would pass `VICTORY_MERGE_RUNS` goes as a patch of
its exact changed bytes instead of the flights being settled, while pages under the cap still go as
this fork's word pairs and runs. The copies are walked for patches only when one was made since the
last submission, the patched bytes are left out of this fork's landings until sent as the changed
ranges are, and two copies in one buffer get a barrier between their patches.

### Merged from fxceless/victory-fork (2026-11-27, 03:45 UTC)

Upstream's notes of commits 8a32d0ba to 67429060 (their numbers, on their machine):

- **The ray-tracing structure builder runs** (`VICTORY_BVH_BUILDER`, now on by default; `=0` turns it
  off): its last three programs, the group prefix sums, translate (a shared-memory read at a uniform
  address counts as lane-uniform, after PR #36). **Known problem: about 30-40 s after the builder
  starts, the title crashes on a write to address 0 (two runs, with and without
  `VICTORY_BVH_QUARANTINE`).** Until that is traced, set `VICTORY_BVH_BUILDER=0` to play.
- **Water:** the two water programs run in the paired wave64 form (pull request #1419 plus exact
  reductions here; set `VICTORY_WAVE_PAIRS_ONLY=5dc3b25986622949,6f225b641c5b0708`,
  `VICTORY_WAVE_PAIRS_FP32=non-ieee-rne-ftz`): the water has its green colour. The tile start values the
  corner thread skips on an early return are now set by every thread first (`VICTORY_LDS_INIT_HOIST=0`
  off), for the flickering dark tiles; not yet confirmed in a game run.
- **New, off by default:** `VICTORY_GPU_DETILE` (textures detiled on the card), `VICTORY_INPLACE_CONSTANTS`
  (the title's constants heap read by the card in place; did not open in its first run),
  `VICTORY_PATCH_LEAN`, `VICTORY_DUMP_REFUSED=<dir>` (saves shaders that do not translate).

### Merged from fxceless/victory-fork (2026-11-27, 06:30 UTC)

Upstream's notes of commit 9a9091d1 (their numbers, on their machine):

- **The structure builder's crash is fixed** upstream: it was the title's own hang watchdog (a store to
  address 0 after 30 s without a finished frame): their layer brought every copy of title memory the
  builder had named up to date in every list, and snapshot, copied back and compared up to 4 GiB per
  list. There a list now syncs only the copies its own dispatches name and those shaders reached through
  pointers (`VICTORY_MEMORY_SYNC_ALL=1` / `VICTORY_MEMORY_WHOLE_WRITES=1` restore the old ways); up to
  4,095 copies. Here title memory by address keeps this fork's arenas: a list declares written only what
  its dispatches name (as before), so those two switches have nothing to switch.
- **Written-page notes:** translated stores and atomics flag each 64 KiB page they store to. Here the
  pages of title buffers the builder was given to write, and of windows of title memory, are pulled back
  into title memory a few frames later (`VICTORY_MEMORY_PULL=0`: not); arenas' pages stay on the card.
- **Rays now see the structures:** a ray-tracing structure view has no resource, its address is in the
  view description (`RaytracingAccelerationStructure.Location`); shaders were given address 0, read an
  empty node and every ray missed. Instance and procedural nodes now return all ones as the hardware does
  (AMD GPURT), not a miss.
- Upstream's result: no crash past the builder (runs of 5 to 9 minutes) and ray-traced shadows later in a
  run, **but 0.2 to 3 frames a second while it runs**; `VICTORY_BVH_BUILDER=0` plays at the usual speed.

What is known to be wrong:

- Trees, signs and far objects flicker in the distance in some places.
- Lighting is too bright or too dark after walking indoors, until the title has
  run for a while; rooms with little direct light are too dark without `-RayTracing`.
- Textures of what just came into view appear late while the title streams.
- The first run of a new build stutters while the driver makes pipelines
  (`victory warm` makes them ahead).
- Recording a frame in the world makes the title's own watchdog end it.

## How to run it

You need Windows 10/11 x64, a CPU with AVX2, a Direct3D 12 GPU with shader
model 6.6 and mesh shaders, Rust (the toolchain is pinned), the MSVC build
tools, and your own dump of the title.

```powershell
git clone https://github.com/avatarmanz/VIctory-7900XTX
cd VIctory-7900XTX
copy victory.example.toml victory.toml      # then set your paths, outside this folder
cargo build --locked --profile diag -p victory --features diagnostics
scripts\play.ps1 -Exe <path of your copy's executable>
```

`scripts\play.ps1 -Safe` leaves out the newest, faster ways; try it first when
something is wrong. `-RayTracing` turns the title's ray-traced indirect light on.

On a hybrid-graphics PC, the runtime and offline pipeline checks prefer a
high-performance adapter with Shader Model 6.6. The runtime logs its adapter
name, PCI vendor and device IDs, and dedicated VRAM. Set
`VICTORY_D3D12_ADAPTER` to a zero-based DXGI index or part of the adapter's
name to select it explicitly. With the launcher, pass
`-More @{VICTORY_D3D12_ADAPTER='NVIDIA'}`.

Before the first run, ahead of time (both can be stopped and go on later):

```powershell
target\diag\victory.exe shaders <the shader archives>   # every shader translated, minutes
target\diag\victory.exe warm --used                     # pipelines earlier runs drew with, for a new driver or build
```

Commands, debugging switches and upstream's own description:
[docs/README-fxceless.md](docs/README-fxceless.md). Every `VICTORY_...` switch is
described in a comment where it is read.

## What is different here

In short; the reasons and the measurements are in [docs/findings.md](docs/findings.md).

- **List calls are written down, not carried out, on the title's threads**
  (`VICTORY_THIN`): eight threads of ours translate them.
- **Submissions are prepared by a thread of ours and joined**
  (`VICTORY_ASYNC_SUBMIT`, `VICTORY_JOIN_PARTS`), with what can be spread over
  threads spread: refreshes, compares (AVX2), descriptor records, landings.
- **What shaders wrote comes back beside the write guards**, by the dwords that
  changed, on a thread of its own (`VICTORY_RELAY`, `VICTORY_LAND`).
- **Buffer fills that need bytes still on the card are done on the card**
  (`VICTORY_CARD_STEPS`) instead of waiting for it.
- **The title's writes are caught more cheaply**: one view opened at a fault,
  blocks that rest, places in the title's code that look before they write.
- **Textures go up on a copy queue of their own** and a slot shows one only after
  its upload ran, which also ended a device loss that came every 15 minutes or so.
- **Boats float**: textures that shaders write and the title's CPU reads go back
  to title memory (from fxceless and Geckomacabre, on by default here).
- **Frame record and replay** with per-pass times and shader variants compared
  turn by turn ([docs/replay.md](docs/replay.md)).

## Credits

This is a tree of other people's work with more on top. The history is kept, and
commits taken from elsewhere name the commit and its author.

- **[VIctory](https://github.com/Marvisak/VIctory)** by Marvisak and its
  contributors: the loader, the runtime services, the Direct3D 12 back end and
  the shader recompiler.
- **[fxceless/victory-fork](https://github.com/fxceless/victory-fork)**: the fork
  this one started from, and the people whose pull requests it carries.
- **[DexNom/VIctory](https://github.com/DexNom/VIctory)**: ray tracing fixes,
  cube gradients, sparse textures.
- **[Belobug/VIctory-Belobug-AMD](https://github.com/Belobug/VIctory-Belobug-AMD)**:
  the ambient light idea, pipeline workers, and work by romainhedouin.
- **[FQY55kai/ZJOU-VICTORY](https://github.com/FQY55kai/ZJOU-VICTORY)**.
- **[myokex/victory-amd](https://github.com/myokex/victory-amd)**: the guard over
  every view of a page (with Spexivus, joroyt and tightfall), and Joshua Odom's
  reads into title memory.
- **Geckomacabre** and **cxy0072019-sketch**: the texture read-back.
- **[XCL](https://github.com/ivall/xcl)** (GPL-2.0-only) was read for how the same
  title behaves under another layer; no code of it is here.
- This fork's changes were made with [Claude Code](https://claude.com/claude-code).

## License

GPL-3.0-or-later, as VIctory. See [LICENSE](LICENSE); the one file that is not
covered is named in [docs/asset-policy.md](docs/asset-policy.md).
