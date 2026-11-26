//! What makes the title's page remaps slow in the intro, measured alone on
//! this PC: one thread maps and unmaps 64 KiB views of a reserved section
//! over placeholders exactly as `xmem::map` / `xmem::unmap` do (split,
//! MapViewOfFile3, commit, a write, UnmapViewOfFile2), while three other
//! threads do, phase by phase: nothing; make and free D3D12 upload buffers
//! as the texture staging does (made, mapped, filled, released ~700 later)
//! with the card never reading them; the same with the card copying out of
//! each one before it is let go (as the runtime's uploads are used); upload
//! buffers kept and used again, the card copying out of them; buffers in the
//! card's own memory made, filled by the card and let go. Prints the remap
//! thread's microseconds per call by phase.
//!
//! `cargo run --profile diag -p victory-gpu --example remapchurn [seconds per phase]`

use std::collections::VecDeque;
use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use windows::core::Interface;
use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_12_0;
use windows::Win32::Graphics::Direct3D12::*;
use windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC;
use windows::Win32::System::Threading::{CreateEventW, INFINITE, WaitForSingleObject};
use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows_sys::Win32::System::Memory::*;
use windows_sys::Win32::System::Threading::GetCurrentProcess;

const PAGE: usize = 64 << 10;
const PAGES: usize = 1024;
/// Buffers alive at once per worker before the oldest is let go.
const ALIVE: usize = 700;
/// Bytes the card copies out of a buffer at most.
const COPY: u64 = 1 << 20;

struct Remapper {
    section: windows_sys::Win32::Foundation::HANDLE,
    base: usize,
}

impl Remapper {
    fn new() -> Remapper {
        // SAFETY: plain creation of a pagefile-backed reserved section and a
        // placeholder over the range the views go to (as xmem makes them).
        unsafe {
            let bytes = (PAGES * PAGE) as u64;
            let section = CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                std::ptr::null(),
                PAGE_READWRITE | SEC_RESERVE,
                (bytes >> 32) as u32,
                bytes as u32,
                std::ptr::null(),
            );
            assert!(!section.is_null(), "section");
            let base = VirtualAlloc2(
                GetCurrentProcess(),
                std::ptr::null(),
                PAGES * PAGE,
                MEM_RESERVE | MEM_RESERVE_PLACEHOLDER,
                PAGE_NOACCESS,
                std::ptr::null_mut(),
                0,
            );
            assert!(!base.is_null(), "placeholder");
            Remapper { section, base: base as usize }
        }
    }

    /// One page of the section shown at page `at` of the range: xmem::map.
    fn map(&self, at: usize, index: usize) {
        let base = self.base + at * PAGE;
        // SAFETY: as xmem::map, on our own placeholder.
        unsafe {
            VirtualFree(base as *mut c_void, PAGE, MEM_RELEASE | MEM_PRESERVE_PLACEHOLDER);
            let view = MapViewOfFile3(
                self.section,
                GetCurrentProcess(),
                base as *const c_void,
                (index * PAGE) as u64,
                PAGE,
                MEM_REPLACE_PLACEHOLDER,
                PAGE_READWRITE,
                std::ptr::null_mut(),
                0,
            );
            assert_eq!(view.Value as usize, base, "map: {}", std::io::Error::last_os_error());
            assert!(!VirtualAlloc(base as *const c_void, PAGE, MEM_COMMIT, PAGE_READWRITE).is_null(), "commit");
            std::ptr::write_volatile(base as *mut u8, 1);
        }
    }

    /// xmem::unmap of one view.
    fn unmap(&self, at: usize) {
        let base = self.base + at * PAGE;
        // SAFETY: a view mapped by `map`.
        unsafe {
            UnmapViewOfFile2(GetCurrentProcess(), MEMORY_MAPPED_VIEW_ADDRESS { Value: base as *mut c_void }, MEM_PRESERVE_PLACEHOLDER);
        }
    }
}

fn buffer(device: &ID3D12Device, kind: D3D12_HEAP_TYPE, size: u64) -> ID3D12Resource {
    let heap = D3D12_HEAP_PROPERTIES { Type: kind, ..Default::default() };
    let desc = D3D12_RESOURCE_DESC {
        Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
        Width: size,
        Height: 1,
        DepthOrArraySize: 1,
        MipLevels: 1,
        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
        Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
        ..Default::default()
    };
    let state = if kind == D3D12_HEAP_TYPE_UPLOAD { D3D12_RESOURCE_STATE_GENERIC_READ } else { D3D12_RESOURCE_STATE_COMMON };
    let mut made: Option<ID3D12Resource> = None;
    // SAFETY: as Host::upload_buffer.
    unsafe {
        device.CreateCommittedResource(&heap, D3D12_HEAP_FLAG_NONE, &desc, state, None, &mut made).expect("buffer");
    }
    made.unwrap()
}

/// A shared buffer (system memory, write-back, card reads and writes) made,
/// mapped and filled, as `Host::shared_buffer`.
fn shared(device: &ID3D12Device, size: u64) -> ID3D12Resource {
    let heap = D3D12_HEAP_PROPERTIES {
        Type: D3D12_HEAP_TYPE_CUSTOM,
        CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_WRITE_BACK,
        MemoryPoolPreference: D3D12_MEMORY_POOL_L0,
        ..Default::default()
    };
    let desc = D3D12_RESOURCE_DESC {
        Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
        Width: size,
        Height: 1,
        DepthOrArraySize: 1,
        MipLevels: 1,
        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
        Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
        Flags: D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS,
        ..Default::default()
    };
    let mut made: Option<ID3D12Resource> = None;
    let mut data = std::ptr::null_mut();
    // SAFETY: as Host::shared_buffer.
    unsafe {
        device.CreateCommittedResource(&heap, D3D12_HEAP_FLAG_NONE, &desc, D3D12_RESOURCE_STATE_COMMON, None, &mut made).expect("shared");
        made.as_ref().unwrap().Map(0, None, Some(&mut data)).expect("map");
        std::ptr::write_bytes(data.cast::<u8>(), 0x5a, size as usize);
    }
    made.unwrap()
}

/// An upload buffer made, mapped and filled.
fn upload(device: &ID3D12Device, size: u64) -> ID3D12Resource {
    let b = buffer(device, D3D12_HEAP_TYPE_UPLOAD, size);
    let mut data = std::ptr::null_mut();
    // SAFETY: the buffer's own mapping, `size` bytes.
    unsafe {
        b.Map(0, None, Some(&mut data)).expect("map");
        std::ptr::write_bytes(data.cast::<u8>(), 0x5a, size as usize);
    }
    b
}

/// One worker's way to the card: a queue, a list, a fence, a buffer in the
/// card's memory to copy into.
struct Card {
    queue: ID3D12CommandQueue,
    allocator: ID3D12CommandAllocator,
    list: ID3D12GraphicsCommandList,
    fence: ID3D12Fence,
    event: windows::Win32::Foundation::HANDLE,
    value: u64,
    landing: ID3D12Resource,
}

impl Card {
    fn new(device: &ID3D12Device) -> Card {
        // SAFETY: plain object creation.
        unsafe {
            let queue: ID3D12CommandQueue = device
                .CreateCommandQueue(&D3D12_COMMAND_QUEUE_DESC { Type: D3D12_COMMAND_LIST_TYPE_DIRECT, ..Default::default() })
                .expect("queue");
            let allocator: ID3D12CommandAllocator = device.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT).expect("allocator");
            let list: ID3D12GraphicsCommandList =
                device.CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &allocator, None).expect("list");
            list.Close().expect("close");
            let fence: ID3D12Fence = device.CreateFence(0, D3D12_FENCE_FLAG_NONE).expect("fence");
            let event = CreateEventW(None, false, false, None).expect("event");
            let landing = buffer(device, D3D12_HEAP_TYPE_DEFAULT, COPY);
            Card { queue, allocator, list, fence, event, value: 0, landing }
        }
    }

    /// The card copies up to 1 MiB of `from` into `to` (its landing buffer
    /// if none), and this thread waits for it.
    fn copy(&mut self, from: &ID3D12Resource, size: u64, to: Option<&ID3D12Resource>) {
        let to = to.unwrap_or(&self.landing).clone();
        // SAFETY: our own list, allocator and fence; the copy stays inside both.
        unsafe {
            self.allocator.Reset().expect("reset");
            self.list.Reset(&self.allocator, None).expect("list reset");
            self.list.CopyBufferRegion(&to, 0, from, 0, size.min(COPY));
            self.list.Close().expect("close");
            self.queue.ExecuteCommandLists(&[Some(self.list.cast().expect("cast"))]);
            self.value += 1;
            self.queue.Signal(&self.fence, self.value).expect("signal");
            if self.fence.GetCompletedValue() < self.value && self.fence.SetEventOnCompletion(self.value, self.event).is_ok() {
                WaitForSingleObject(self.event, INFINITE);
            }
        }
    }
}

/// Sizes as the intro's staging: 64 KiB to 1 MiB, about 320 KiB on average.
fn size_of_nth(n: u64) -> u64 {
    [64u64, 128, 176, 256, 320, 448, 704, 1024][(n.wrapping_mul(2654435761) >> 7) as usize % 8] << 10
}

fn main() {
    let secs: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(4);
    let mut device: Option<ID3D12Device> = None;
    // SAFETY: the default adapter.
    unsafe { D3D12CreateDevice(None, D3D_FEATURE_LEVEL_12_0, &mut device).expect("device") };
    let device = device.unwrap();

    let names = [
        "nothing beside",
        "upload buffers made and freed, card never reads them",
        "upload buffers made and freed, card copies out of each",
        "upload buffers kept, card copies out of them",
        "card-memory buffers made, filled by the card, freed",
        "nothing beside",
        "write-watched 2 GB: one writes, two GetWriteWatch(reset)",
        "1 GB view: 64 KiB blocks made read-only and back",
        "nothing beside",
        "6 GB of shared buffers kept (card maps them)",
        "+ shared buffers made, card copies out, freed",
        "+ nothing beside",
    ];
    // The write-watched region (as the title's own memory) and a view of a
    // committed section (as title view memory the guards protect).
    const BIG: usize = 2 << 30;
    // SAFETY: plain allocations, touched once so every page is present.
    let (watched, viewed) = unsafe {
        let w = VirtualAlloc(std::ptr::null(), BIG, MEM_RESERVE | MEM_COMMIT | 0x20_0000, PAGE_READWRITE) as usize;
        assert!(w != 0, "write watch");
        for p in (0..BIG).step_by(4096) {
            std::ptr::write_volatile((w + p) as *mut u8, 1);
        }
        let s = CreateFileMappingW(INVALID_HANDLE_VALUE, std::ptr::null(), PAGE_READWRITE | SEC_COMMIT, 0, (BIG / 2) as u32, std::ptr::null());
        assert!(!s.is_null(), "view section");
        let v = MapViewOfFile(s, FILE_MAP_ALL_ACCESS, 0, 0, BIG / 2).Value as usize;
        assert!(v != 0, "view");
        for p in (0..BIG / 2).step_by(4096) {
            std::ptr::write_volatile((v + p) as *mut u8, 1);
        }
        (w, v)
    };
    let phase = Arc::new(AtomicUsize::new(0));
    let stop = Arc::new(AtomicBool::new(false));
    let churned = Arc::new(AtomicU64::new(0));
    let mut workers = Vec::new();
    for w in 0..3u64 {
        let (device, phase, stop, churned) = (device.clone(), phase.clone(), stop.clone(), churned.clone());
        workers.push(std::thread::spawn(move || {
            let mut card = Card::new(&device);
            let mut alive: VecDeque<ID3D12Resource> = VecDeque::new();
            let mut kept: Vec<ID3D12Resource> = Vec::new();
            let mut n = w * 1_000_003;
            while !stop.load(Ordering::Relaxed) {
                n += 1;
                let size = size_of_nth(n);
                let p = phase.load(Ordering::Relaxed);
                if p != 3 {
                    kept.clear();
                }
                match p {
                    1 | 2 => {
                        let b = upload(&device, size);
                        if p == 2 {
                            card.copy(&b, size, None);
                        }
                        alive.push_back(b);
                        if alive.len() > ALIVE {
                            alive.pop_front();
                        }
                    }
                    3 => {
                        alive.clear();
                        if kept.len() < ALIVE {
                            kept.push(upload(&device, COPY));
                        }
                        let b = kept[(n as usize) % kept.len()].clone();
                        let mut data = std::ptr::null_mut();
                        // SAFETY: the kept buffer's mapping, 1 MiB.
                        unsafe {
                            b.Map(0, None, Some(&mut data)).expect("map");
                            std::ptr::write_bytes(data.cast::<u8>(), 0x5a, size as usize);
                        }
                        card.copy(&b, size, None);
                    }
                    4 => {
                        if kept.is_empty() {
                            kept.push(upload(&device, COPY));
                        }
                        let source = kept[0].clone();
                        let v = buffer(&device, D3D12_HEAP_TYPE_DEFAULT, size);
                        card.copy(&source, size, Some(&v));
                        alive.push_back(v);
                        if alive.len() > ALIVE {
                            alive.pop_front();
                        }
                    }
                    6 if w == 0 => {
                        // The title writing here and there.
                        for i in 0..4096u64 {
                            let page = ((n * 4096 + i).wrapping_mul(2654435761) as usize) % (BIG / 4096);
                            // SAFETY: inside the committed region.
                            unsafe { std::ptr::write_volatile((watched + page * 4096) as *mut u8, 2) };
                        }
                    }
                    6 => {
                        // An asking for the written pages of a 64 MiB slice, as `watch::writes`.
                        let slice = 64 << 20;
                        let at = watched + (n as usize % (BIG / slice)) * slice;
                        let mut found = vec![std::ptr::null_mut::<c_void>(); slice / 4096];
                        let (mut count, mut granularity) = (found.len(), 0u32);
                        // SAFETY: the answer buffer holds `count` entries.
                        unsafe { GetWriteWatch(1, at as *const c_void, slice, found.as_mut_ptr(), &mut count, &mut granularity) };
                    }
                    7 => {
                        let block = ((n.wrapping_mul(2654435761) >> 3) as usize) % (BIG / 2 / PAGE);
                        let at = (viewed + block * PAGE) as *const c_void;
                        let mut old = 0;
                        // SAFETY: our own view; only its protection changes.
                        unsafe {
                            VirtualProtect(at, PAGE, PAGE_READONLY, &mut old);
                            VirtualProtect(at, PAGE, PAGE_READWRITE, &mut old);
                        }
                    }
                    10 => {
                        let b = shared(&device, size);
                        card.copy(&b, size, None);
                        alive.push_back(b);
                        if alive.len() > ALIVE {
                            alive.pop_front();
                        }
                    }
                    _ => {
                        alive.clear();
                        std::thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                }
                churned.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    let r = Remapper::new();
    let mut k = 0usize;
    println!("{:<56} {:>9} {:>9} {:>9} {:>9}", "beside the remaps", "calls", "us/call", "max us", "made/s");
    let mut held = Vec::new();
    for (p, name) in names.iter().enumerate() {
        if p == 9 {
            held = (0..6144).map(|_| shared(&device, 1 << 20)).collect();
        }
        phase.store(p, Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(1500));
        let c0 = churned.load(Ordering::Relaxed);
        let start = Instant::now();
        let (mut calls, mut worst) = (0u64, 0u128);
        while start.elapsed() < Duration::from_secs(secs) {
            let at = k % PAGES;
            let t = Instant::now();
            r.map(at, (k * 7) % PAGES);
            let m = t.elapsed().as_micros();
            let t = Instant::now();
            r.unmap(at);
            let u = t.elapsed().as_micros();
            worst = worst.max(m).max(u);
            calls += 2;
            k += 1;
        }
        let el = start.elapsed();
        println!(
            "{:<56} {:>9} {:>9.1} {:>9} {:>9.0}",
            name,
            calls,
            el.as_micros() as f64 / calls as f64,
            worst,
            (churned.load(Ordering::Relaxed) - c0) as f64 / el.as_secs_f64()
        );
    }
    drop(held);
    stop.store(true, Ordering::Relaxed);
    for w in workers {
        let _ = w.join();
    }
}
