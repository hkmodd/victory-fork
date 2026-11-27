//! D3D12.X on host D3D12.
//!
//! D3D12.X records commands from code inlined into the title, which writes
//! packets into the command list while its put pointer is below a limit and
//! calls the list's method otherwise. Our lists keep the limits at zero, so
//! every command arrives here as a call, to be run on host D3D12. The
//! title's GPU addresses are CPU addresses (console memory is unified).
//!
//! [`device`] is the device object titles create, [`dxgi`] its adapter side,
//! [`resource`] heaps and resources, [`fence`] fences, [`queue`] command
//! queues, [`list`] command allocators and lists, [`descriptor`]
//! descriptor heaps, [`root`] root signatures, [`query`] query heaps,
//! [`signature`] command signatures, [`pipeline`] pipeline states,
//! [`frame`] frame pacing, [`object`] what all console graphics objects
//! share, [`console`] which console model the title sees. Planned:
//! `present` (swap chain).

#[cfg(windows)]
mod ambient;
#[cfg(all(windows, feature = "diagnostics"))]
mod backreads;
#[cfg(windows)]
mod lightprobe;
#[cfg(windows)]
mod texlate;
#[cfg(windows)]
mod texstore;
#[cfg(all(windows, feature = "diagnostics"))]
mod capture;
#[cfg(windows)]
mod cardmeter;
#[cfg(windows)]
mod compose;
#[cfg(all(windows, feature = "diagnostics"))]
mod pix;
#[cfg(all(windows, feature = "diagnostics"))]
mod renderdoc;
#[cfg(all(windows, feature = "diagnostics"))]
mod shot;
pub mod console;
#[cfg(all(windows, feature = "diagnostics"))]
mod copystat;
#[cfg(windows)]
mod heap;
#[cfg(all(windows, not(feature = "diagnostics")))]
#[global_allocator]
static HEAP: heap::Chosen = heap::Chosen;
#[cfg(windows)]
pub use heap::is_mimalloc;
#[cfg(all(windows, feature = "diagnostics"))]
mod faultbook;
#[cfg(windows)]
mod cores;
pub mod descriptor;
pub mod device;
#[cfg(windows)]
mod drvwatch;
#[cfg(windows)]
mod fpsbox;
#[cfg(windows)]
mod stats;
#[cfg(windows)]
mod spread;
#[cfg(windows)]
mod draw;
pub mod dxgi;
#[cfg(windows)]
pub mod external;
pub mod fence;
pub mod frame;
#[cfg(all(windows, feature = "diagnostics"))]
mod book;
#[cfg_attr(not(windows), allow(dead_code))]
mod heap_pass;
#[cfg(windows)]
pub mod framelog;
#[cfg(windows)]
mod gpudetile;
#[cfg(windows)]
mod heap_worker;
#[cfg(windows)]
mod host;
#[cfg(windows)]
mod indirect;
#[cfg(windows)]
mod inplace;
#[cfg(windows)]
mod port;
#[cfg(windows)]
mod port_cbv;
#[cfg(windows)]
pub mod port_descriptors;
#[cfg(windows)]
mod port_journal;
#[cfg(windows)]
mod port_rt;
#[cfg(windows)]
mod land;
mod launch;
pub mod list;
pub mod object;
pub mod pipeline;
#[cfg(windows)]
mod picked;
#[cfg(windows)]
mod plaincopy;
#[cfg(windows)]
mod pool;
#[cfg(all(windows, feature = "diagnostics"))]
mod popwatch;
#[cfg(windows)]
mod primitive;
#[cfg(all(windows, feature = "diagnostics"))]
mod profile;
pub mod query;
pub mod queue;
#[cfg(windows)]
mod queue2;
#[cfg(all(windows, feature = "diagnostics"))]
pub mod replay;
#[cfg(windows)]
mod residency;
pub mod resource;
pub mod root;
#[cfg(windows)]
mod safecopy;
#[cfg(windows)]
mod sampler;
#[cfg(windows)]
pub mod shader_cache;
#[cfg(windows)]
mod memory;
#[cfg(windows)]
mod shadow;
#[cfg(windows)]
mod tiles;
pub mod signature;
#[cfg(windows)]
pub mod streamline;
mod switch;
#[cfg(all(windows, feature = "diagnostics"))]
mod ab;
#[cfg(windows)]
mod tdr;
#[cfg(windows)]
mod texture;
pub mod timeline;
#[cfg(windows)]
pub mod titlewrite;
#[cfg(windows)]
mod upload;
#[cfg(windows)]
mod watch;
#[cfg(windows)]
mod texback;
#[cfg(windows)]
mod writeback;

#[cfg(windows)]
pub use host::on_failure;
pub use host::log_defrag_counters;

/// Whether `switch` is on now, if it is one of those `VICTORY_AB` turns on and
/// off while the title runs (`ab`).
#[cfg(all(windows, feature = "diagnostics"))]
pub fn switched(switch: &str) -> Option<bool> {
    ab::pick(switch)
}

/// What the frame is at (`copystat::phase`), for the sampling profiler.
#[cfg(all(windows, feature = "diagnostics"))]
pub fn frame_phase() -> u32 {
    copystat::phase()
}

/// Frames the title has begun (its frame waits that ended), for logs.
pub fn frames_begun() -> u64 {
    frame::current()
}
#[cfg(windows)]
pub use inplace::{title_memory_freeing, title_memory_section, title_memory_section_gone, title_memory_view};
#[cfg(windows)]
pub use watch::{
    kernel_writing, title_memory_allocated, title_memory_committed, title_memory_freed, title_memory_reading,
    title_memory_remapped, title_memory_remapping, title_memory_remapping_range, title_memory_splitting,
    title_memory_unmapping, title_memory_written, writing as kernel_will_write,
};
