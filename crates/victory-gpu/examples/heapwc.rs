//! Which D3D12 memory shows in this process as private commit and as
//! write-combined regions (the 13 GB of 64 and 32 MB write-combined
//! allocations of the avm tree's memory map): makes 16 of each kind, made
//! resident, and prints the change in private bytes, in committed
//! write-combined bytes and their allocation sizes, and in bytes in memory.
//!
//! `cargo run --profile diag -p victory-gpu --example heapwc`

use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_12_0;
use windows::Win32::Graphics::Direct3D12::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows_sys::Win32::System::Memory::*;
use windows_sys::Win32::System::ProcessStatus::{K32GetPerformanceInfo, K32GetProcessMemoryInfo, PERFORMANCE_INFORMATION, PROCESS_MEMORY_COUNTERS_EX};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

const N: usize = 16;
const SIZE: u64 = 64 << 20;

/// Private bytes, working set.
fn process() -> (u64, u64) {
    // SAFETY: plain query into a sized struct.
    unsafe {
        let mut c: PROCESS_MEMORY_COUNTERS_EX = std::mem::zeroed();
        c.cb = size_of_val(&c) as u32;
        K32GetProcessMemoryInfo(GetCurrentProcess(), (&raw mut c).cast(), c.cb);
        (c.PrivateUsage as u64, c.WorkingSetSize as u64)
    }
}

/// System commit charge, available physical memory.
fn system() -> (u64, u64) {
    // SAFETY: plain query into a sized struct.
    unsafe {
        let mut i: PERFORMANCE_INFORMATION = std::mem::zeroed();
        i.cb = size_of_val(&i) as u32;
        K32GetPerformanceInfo(&mut i, i.cb);
        let page = i.PageSize as u64;
        (i.CommitTotal as u64 * page, i.PhysicalAvailable as u64 * page)
    }
}

/// Committed write-combined bytes, private committed bytes, and the
/// write-combined allocations by whole size.
fn walk() -> (u64, u64, std::collections::BTreeMap<usize, usize>) {
    let mut wc = 0u64;
    let mut private = 0u64;
    let mut whole: std::collections::HashMap<usize, (usize, bool)> = std::collections::HashMap::new();
    let mut at = 0usize;
    loop {
        // SAFETY: valid out pointer.
        let mut m: MEMORY_BASIC_INFORMATION = unsafe { std::mem::zeroed() };
        if unsafe { VirtualQuery(at as *const _, &mut m, size_of_val(&m)) } == 0 {
            break;
        }
        if m.State != MEM_FREE {
            let e = whole.entry(m.AllocationBase as usize).or_default();
            e.0 += m.RegionSize;
            if m.State == MEM_COMMIT && m.Protect & PAGE_WRITECOMBINE != 0 {
                wc += m.RegionSize as u64;
                e.1 = true;
            }
            if m.State == MEM_COMMIT && m.Type == MEM_PRIVATE {
                private += m.RegionSize as u64;
            }
        }
        let next = (m.BaseAddress as usize).saturating_add(m.RegionSize);
        if next <= at {
            break;
        }
        at = next;
    }
    let mut sizes = std::collections::BTreeMap::new();
    for (size, w) in whole.values() {
        if *w {
            *sizes.entry(*size >> 20).or_insert(0) += 1;
        }
    }
    (wc, private, sizes)
}

fn buffer(size: u64, flags: D3D12_RESOURCE_FLAGS) -> D3D12_RESOURCE_DESC {
    D3D12_RESOURCE_DESC {
        Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
        Width: size,
        Height: 1,
        DepthOrArraySize: 1,
        MipLevels: 1,
        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
        Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
        Flags: flags,
        ..Default::default()
    }
}

fn texture() -> D3D12_RESOURCE_DESC {
    D3D12_RESOURCE_DESC {
        Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
        Width: 4096,
        Height: 4096,
        DepthOrArraySize: 1,
        MipLevels: 1,
        Format: DXGI_FORMAT_R8G8B8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
        ..Default::default()
    }
}

fn main() {
    let mut device: Option<ID3D12Device> = None;
    // SAFETY: the default adapter.
    unsafe { D3D12CreateDevice(None, D3D_FEATURE_LEVEL_12_0, &mut device).expect("device") };
    let device = device.unwrap();
    let props = |t: D3D12_HEAP_TYPE, page: D3D12_CPU_PAGE_PROPERTY, pool: D3D12_MEMORY_POOL| D3D12_HEAP_PROPERTIES {
        Type: t,
        CPUPageProperty: page,
        MemoryPoolPreference: pool,
        ..Default::default()
    };
    let default = props(D3D12_HEAP_TYPE_DEFAULT, D3D12_CPU_PAGE_PROPERTY_UNKNOWN, D3D12_MEMORY_POOL_UNKNOWN);
    let upload = props(D3D12_HEAP_TYPE_UPLOAD, D3D12_CPU_PAGE_PROPERTY_UNKNOWN, D3D12_MEMORY_POOL_UNKNOWN);
    let gpu_upload = props(D3D12_HEAP_TYPE_GPU_UPLOAD, D3D12_CPU_PAGE_PROPERTY_UNKNOWN, D3D12_MEMORY_POOL_UNKNOWN);
    let wb = props(D3D12_HEAP_TYPE_CUSTOM, D3D12_CPU_PAGE_PROPERTY_WRITE_BACK, D3D12_MEMORY_POOL_L0);
    let wcl0 = props(D3D12_HEAP_TYPE_CUSTOM, D3D12_CPU_PAGE_PROPERTY_WRITE_COMBINE, D3D12_MEMORY_POOL_L0);
    let mut keep: Vec<windows::core::IUnknown> = Vec::new();
    let mut cases: Vec<(&str, Box<dyn Fn() -> Option<windows::core::IUnknown>>)> = Vec::new();
    for (name, p, flags, map) in [
        ("committed DEFAULT buffer 64 MB", default, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS, false),
        ("committed UPLOAD buffer 64 MB, mapped", upload, D3D12_RESOURCE_FLAG_NONE, true),
        ("committed CUSTOM write-back L0 64 MB, mapped", wb, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS, true),
        ("committed CUSTOM write-combine L0 64 MB, mapped", wcl0, D3D12_RESOURCE_FLAG_NONE, true),
        ("committed GPU_UPLOAD 64 MB, mapped", gpu_upload, D3D12_RESOURCE_FLAG_NONE, true),
    ] {
        let device = device.clone();
        cases.push((
            name,
            Box::new(move || {
                let state = if p.Type == D3D12_HEAP_TYPE_UPLOAD { D3D12_RESOURCE_STATE_GENERIC_READ } else { D3D12_RESOURCE_STATE_COMMON };
                let mut r: Option<ID3D12Resource> = None;
                // SAFETY: plain creation.
                unsafe { device.CreateCommittedResource(&p, D3D12_HEAP_FLAG_NONE, &buffer(SIZE, flags), state, None, &mut r) }.ok()?;
                let r = r?;
                if map {
                    let mut data = std::ptr::null_mut();
                    // SAFETY: a CPU-visible buffer; one byte per page written.
                    unsafe {
                        r.Map(0, None, Some(&mut data)).ok()?;
                        for i in (0..SIZE as usize).step_by(4096) {
                            data.cast::<u8>().add(i).write_volatile(1);
                        }
                    }
                }
                Some(r.into())
            }),
        ));
    }
    for (name, flags) in [
        ("heap DEFAULT 64 MB, non-RT textures (the texture pool)", D3D12_HEAP_FLAG_ALLOW_ONLY_NON_RT_DS_TEXTURES),
        ("heap DEFAULT 64 MB, buffers", D3D12_HEAP_FLAG_ALLOW_ONLY_BUFFERS),
    ] {
        let device = device.clone();
        cases.push((
            name,
            Box::new(move || {
                let desc = D3D12_HEAP_DESC { SizeInBytes: SIZE, Properties: default, Flags: flags, ..Default::default() };
                let mut heap: Option<ID3D12Heap> = None;
                // SAFETY: plain creation.
                unsafe { device.CreateHeap(&desc, &mut heap) }.ok()?;
                let heap = heap?;
                if flags == D3D12_HEAP_FLAG_ALLOW_ONLY_NON_RT_DS_TEXTURES {
                    let mut r: Option<ID3D12Resource> = None;
                    // SAFETY: a 64 MB texture placed at the heap's start.
                    unsafe { device.CreatePlacedResource(&heap, 0, &texture(), D3D12_RESOURCE_STATE_COMMON, None, &mut r) }.ok()?;
                    std::mem::forget(r);
                }
                Some(heap.into())
            }),
        ));
    }
    let device1 = device.clone();
    cases.push((
        "committed DEFAULT texture 4096x4096 RGBA8",
        Box::new(move || {
            let mut r: Option<ID3D12Resource> = None;
            // SAFETY: plain creation.
            unsafe { device1.CreateCommittedResource(&default, D3D12_HEAP_FLAG_NONE, &texture(), D3D12_RESOURCE_STATE_COMMON, None, &mut r) }.ok()?;
            Some(r?.into())
        }),
    ));
    for (name, make) in &cases {
        let (s0, a0) = system();
        let (p0, w0) = process();
        let (c0, v0, _) = walk();
        let mut made = 0;
        for _ in 0..N {
            if let Some(o) = make() {
                keep.push(o);
                made += 1;
            }
        }
        let (s1, a1) = system();
        let (p1, w1) = process();
        let (c1, v1, sizes) = walk();
        let mb = |a: u64, b: u64| (b as i64 - a as i64) as f64 / (1 << 20) as f64;
        println!(
            "{name}: made {made}/{N}: private {:+.0} MB, write-combined committed {:+.0} MB, private regions {:+.0} MB, working set {:+.0} MB, system commit {:+.0} MB, available RAM {:+.0} MB; write-combined allocations (MB: count) {:?}",
            mb(p0, p1),
            mb(c0, c1),
            mb(v0, v1),
            mb(w0, w1),
            mb(s0, s1),
            mb(a0, a1),
            sizes
        );
        keep.clear();
    }
}
