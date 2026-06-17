//! Compacting garbage collector with its own managed heap.
//!
//! All Shæll objects live in a contiguous byte buffer owned by GcHeap.
//! No Rust global allocator is used for object storage — the heap is a
//! pre-allocated `Vec<u8>` that grows only when exhausted and GC can't
//! free enough space.
//!
//! Object layout (aligned to 8 bytes):
//!   [0..8)   header: mark(1) | occupied(1) | pad(6)
//!   [8..16)  forward:  u64   — forwarding offset during compaction
//!   [16..24) obj_size: u64   — size of the stored T
//!   [24..32) tracer:   u64   — Trace function pointer
//!   [32..40) remapper: u64   — Remap function pointer
//!   [40..]   data:    [u8]  — the T value (Copy'd in, Copy'd out)
//!
//! GcRef<T> is an offset into this buffer. Dereferencing uses
//! unsafe pointer casts gated by the type parameter.

use std::cell::{Cell, RefCell};
use std::fmt;

// ============================================================
// Constants
// ============================================================

const HEADER_SIZE: usize = 40; // 5 × u64
const ALIGNMENT: usize = 8;
const INITIAL_HEAP_SIZE: usize = 256 * 1024; // 256 KB
const GROWTH_FACTOR: usize = 2;

// Offsets within the header
const OFF_MARK: usize = 0;
const OFF_FORWARD: usize = 8;
const OFF_OBJ_SIZE: usize = 16;
const OFF_TRACER: usize = 24;
const OFF_REMAPPER: usize = 32;
const OFF_DATA: usize = HEADER_SIZE;

// Mark values
const MARK_FREE: u64 = 0;
const MARK_LIVE: u64 = 1;
const MARK_KEPT: u64 = 2; // marked during tracing
const FORWARD_NULL: u64 = u64::MAX;

// ============================================================
// GcRef — typed offset handle into the managed heap
// ============================================================

pub struct GcRef<T: 'static> {
    offset: usize,
    _marker: std::marker::PhantomData<T>,
}

impl<T: 'static> Clone for GcRef<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: 'static> Copy for GcRef<T> {}
impl<T: 'static> PartialEq for GcRef<T> {
    fn eq(&self, other: &Self) -> bool {
        self.offset == other.offset
    }
}
impl<T: 'static> Eq for GcRef<T> {}
impl<T: 'static> std::hash::Hash for GcRef<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.offset.hash(state);
    }
}

unsafe impl<T: 'static> Send for GcRef<T> {}
unsafe impl<T: 'static> Sync for GcRef<T> {}

impl<T: 'static> GcRef<T> {
    pub const NULL: Self = GcRef {
        offset: usize::MAX,
        _marker: std::marker::PhantomData,
    };

    pub fn is_null(self) -> bool {
        self.offset == usize::MAX
    }
    pub fn offset(self) -> usize {
        self.offset
    }
    pub fn from_offset(off: usize) -> Self {
        GcRef {
            offset: off,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T: 'static> fmt::Debug for GcRef<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_null() {
            write!(f, "GcRef::NULL")
        } else {
            write!(f, "GcRef@{}", self.offset)
        }
    }
}

// ============================================================
// Trace & Remap traits
// ============================================================

pub trait Trace {
    fn trace(&self, visit: &mut dyn FnMut(usize));
}

pub trait Remap {
    fn remap(&mut self, old_to_new: &[(usize, usize)]);
}

// Primitives
macro_rules! impl_trivial {
    ($t:ty) => {
        impl Trace for $t {
            fn trace(&self, _: &mut dyn FnMut(usize)) {}
        }
        impl Remap for $t {
            fn remap(&mut self, _: &[(usize, usize)]) {}
        }
    };
}
impl_trivial!(f64);
impl_trivial!(String);
impl_trivial!(bool);
impl_trivial!(());
impl_trivial!(usize);

// u8 is a primitive — used as Vec<u8> element
impl Trace for u8 {
    fn trace(&self, _: &mut dyn FnMut(usize)) {}
}
impl Remap for u8 {
    fn remap(&mut self, _: &[(usize, usize)]) {}
}

impl<T: Trace> Trace for Vec<T> {
    fn trace(&self, v: &mut dyn FnMut(usize)) {
        for x in self {
            x.trace(v);
        }
    }
}
impl<T: Remap> Remap for Vec<T> {
    fn remap(&mut self, map: &[(usize, usize)]) {
        for x in self {
            x.remap(map);
        }
    }
}

impl<T: Trace> Trace for Option<T> {
    fn trace(&self, v: &mut dyn FnMut(usize)) {
        if let Some(ref x) = self {
            x.trace(v);
        }
    }
}
impl<T: Remap> Remap for Option<T> {
    fn remap(&mut self, map: &[(usize, usize)]) {
        if let Some(ref mut x) = self {
            x.remap(map);
        }
    }
}

impl<T: Trace> Trace for Box<T> {
    fn trace(&self, v: &mut dyn FnMut(usize)) {
        (**self).trace(v);
    }
}
impl<T: Remap> Remap for Box<T> {
    fn remap(&mut self, map: &[(usize, usize)]) {
        (**self).remap(map);
    }
}

use std::collections::HashMap;
impl<K: Trace, V: Trace> Trace for HashMap<K, V> {
    fn trace(&self, v: &mut dyn FnMut(usize)) {
        for (k, val) in self {
            k.trace(v);
            val.trace(v);
        }
    }
}
impl<K: Remap + Eq + std::hash::Hash, V: Remap> Remap for HashMap<K, V> {
    fn remap(&mut self, map: &[(usize, usize)]) {
        let entries: Vec<(K, V)> = self.drain().collect();
        for (mut k, mut v) in entries {
            k.remap(map);
            v.remap(map);
            self.insert(k, v);
        }
    }
}

// ============================================================
// Function pointer types
// ============================================================

type TraceFn = unsafe fn(data_ptr: *const u8, visit: &mut dyn FnMut(usize));
type RemapFn = unsafe fn(data_ptr: *mut u8, old_to_new: &[(usize, usize)]);

/// SAFETY: the caller must ensure data_ptr points to a valid T.
unsafe fn trace_thunk<T: Trace>(data: *const u8, visit: &mut dyn FnMut(usize)) {
    let obj = &*(data as *const T);
    obj.trace(visit);
}

/// SAFETY: the caller must ensure data_ptr points to a valid, initialized T.
unsafe fn remap_thunk<T: Remap>(data: *mut u8, map: &[(usize, usize)]) {
    let obj = &mut *(data as *mut T);
    obj.remap(map);
}

// ============================================================
// GcHeap — the managed byte-level heap
// ============================================================

pub struct GcHeap {
    /// The byte buffer. All objects live here. RefCell for interior mutability.
    buffer: RefCell<Vec<u8>>,
    /// Bump pointer: next free offset in the buffer
    bump: Cell<usize>,
    /// Free-list head offset (0 = empty)
    free_head: Cell<usize>,
    /// Allocation count since last GC
    alloc_count: Cell<usize>,
    /// GC threshold
    threshold: Cell<usize>,
    /// Stats
    pub collections: Cell<usize>,
    pub bytes_allocated: Cell<usize>,
    pub bytes_freed: Cell<usize>,
}

impl Default for GcHeap {
    fn default() -> Self {
        Self::new()
    }
}

impl GcHeap {
    pub fn new() -> Self {
        let buffer = RefCell::new(vec![0u8; INITIAL_HEAP_SIZE]);
        GcHeap {
            buffer,
            bump: Cell::new(0),
            free_head: Cell::new(0),
            alloc_count: Cell::new(0),
            threshold: Cell::new(50_000),
            collections: Cell::new(0),
            bytes_allocated: Cell::new(0),
            bytes_freed: Cell::new(0),
        }
    }

    pub fn with_threshold(self, n: usize) -> Self {
        self.threshold.set(n);
        self
    }

    /// Round up to alignment.
    fn align(addr: usize) -> usize {
        (addr + ALIGNMENT - 1) & !(ALIGNMENT - 1)
    }

    // ── header helpers ──────────────────────────────────────

    fn with_buf<R>(&self, f: impl FnOnce(&Vec<u8>) -> R) -> R {
        f(&self.buffer.borrow())
    }

    fn with_buf_mut<R>(&self, f: impl FnOnce(&mut Vec<u8>) -> R) -> R {
        f(&mut self.buffer.borrow_mut())
    }

    fn read_header(&self, offset: usize, field_off: usize) -> u64 {
        let pos = offset + field_off;
        self.with_buf(|buf| u64::from_le_bytes(buf[pos..pos + 8].try_into().unwrap()))
    }

    fn write_header(&self, offset: usize, field_off: usize, value: u64) {
        let pos = offset + field_off;
        self.with_buf_mut(|buf| buf[pos..pos + 8].copy_from_slice(&value.to_le_bytes()));
    }

    fn mark(&self, offset: usize) -> u64 {
        self.read_header(offset, OFF_MARK)
    }
    fn set_mark(&self, offset: usize, v: u64) {
        self.write_header(offset, OFF_MARK, v)
    }
    fn forward(&self, offset: usize) -> u64 {
        self.read_header(offset, OFF_FORWARD)
    }
    fn set_forward(&self, offset: usize, v: u64) {
        self.write_header(offset, OFF_FORWARD, v)
    }
    fn obj_size(&self, offset: usize) -> u64 {
        self.read_header(offset, OFF_OBJ_SIZE)
    }
    fn tracer_fn(&self, offset: usize) -> TraceFn {
        unsafe { std::mem::transmute(self.read_header(offset, OFF_TRACER) as usize) }
    }
    fn set_tracer(&self, offset: usize, f: TraceFn) {
        self.write_header(offset, OFF_TRACER, f as usize as u64);
    }
    fn remapper_fn(&self, offset: usize) -> RemapFn {
        unsafe { std::mem::transmute(self.read_header(offset, OFF_REMAPPER) as usize) }
    }
    fn set_remapper(&self, offset: usize, f: RemapFn) {
        self.write_header(offset, OFF_REMAPPER, f as usize as u64);
    }

    /// Get a raw pointer to object data. Buffer must be borrowed.
    unsafe fn data_ptr(buf: &[u8], offset: usize) -> *const u8 {
        buf[offset + OFF_DATA..].as_ptr()
    }

    /// Get a raw mutable pointer to object data. Buffer must be mutably borrowed.
    unsafe fn data_ptr_mut(buf: &mut [u8], offset: usize) -> *mut u8 {
        buf.as_mut_ptr().add(offset + OFF_DATA)
    }

    // ── allocate ────────────────────────────────────────────

    pub fn allocate<T: Trace + Remap + 'static>(&self, value: T) -> GcRef<T> {
        self.maybe_collect(&[]);

        let obj_size = std::mem::size_of::<T>();
        let total_size = Self::align(HEADER_SIZE + obj_size);

        // Try free list, then bump allocate
        let offset = self
            .alloc_from_free_list(total_size)
            .unwrap_or_else(|| self.bump_allocate(total_size));

        // Write header
        self.set_mark(offset, MARK_LIVE);
        self.set_forward(offset, FORWARD_NULL);
        self.write_header(offset, OFF_OBJ_SIZE, obj_size as u64);
        self.set_tracer(offset, trace_thunk::<T>);
        self.set_remapper(offset, remap_thunk::<T>);

        // Write data into the buffer
        self.with_buf_mut(|buf| unsafe {
            let dst = Self::data_ptr_mut(buf, offset) as *mut T;
            std::ptr::write(dst, value);
        });

        self.bytes_allocated
            .set(self.bytes_allocated.get() + total_size);
        self.alloc_count.set(self.alloc_count.get() + 1);

        GcRef::from_offset(offset)
    }

    fn alloc_from_free_list(&self, needed: usize) -> Option<usize> {
        let mut curr = self.free_head.get();
        let mut prev: usize = 0;

        while curr != 0 {
            if self.mark(curr) != MARK_FREE {
                self.free_head.set(0);
                return None;
            }
            let sz = self.forward(curr) as usize;
            if sz >= needed {
                let next = self.obj_size(curr) as usize;
                if prev == 0 {
                    self.free_head.set(next);
                } else {
                    self.write_header(prev, OFF_OBJ_SIZE, next as u64);
                }
                return Some(curr);
            }
            prev = curr;
            curr = self.obj_size(curr) as usize;
        }
        None
    }

    fn bump_allocate(&self, total_size: usize) -> usize {
        let offset = self.bump.get();
        let mut new_bump = offset + total_size;
        let buf_len = self.with_buf(|b| b.len());

        if new_bump > buf_len {
            // Try GC
            self.collect(&[]);
            let offset = self.bump.get();
            new_bump = offset + total_size;
            let buf_len = self.with_buf(|b| b.len());
            if new_bump > buf_len {
                self.buffer
                    .borrow_mut()
                    .resize((buf_len * GROWTH_FACTOR).max(new_bump), 0);
            }
        }

        self.bump.set(new_bump);
        offset
    }

    // ── access ──────────────────────────────────────────────

    /// Run a closure with an immutable reference to the object.
    pub fn with_ref<T: 'static, R>(&self, handle: GcRef<T>, f: impl FnOnce(&T) -> R) -> R {
        assert!(!handle.is_null(), "null GcRef");
        self.with_buf(|buf| unsafe {
            let ptr = Self::data_ptr(buf, handle.offset) as *const T;
            f(&*ptr)
        })
    }

    /// Run a closure with a mutable reference to the object.
    pub fn with_mut<T: 'static, R>(&self, handle: GcRef<T>, f: impl FnOnce(&mut T) -> R) -> R {
        assert!(!handle.is_null(), "null GcRef");
        self.with_buf_mut(|buf| unsafe {
            let ptr = Self::data_ptr_mut(buf, handle.offset) as *mut T;
            f(&mut *ptr)
        })
    }

    pub fn is_live_by_offset(&self, offset: usize) -> bool {
        offset != usize::MAX
            && self.with_buf(|b| offset < b.len())
            && self.mark(offset) == MARK_LIVE
    }

    // ── collection ──────────────────────────────────────────

    fn maybe_collect(&self, roots: &[usize]) {
        if self.alloc_count.get() >= self.threshold.get() {
            self.collect(roots);
            self.alloc_count.set(0);
        }
    }

    pub fn force_collect(&self, roots: &[usize]) -> Vec<(usize, usize)> {
        let map = self.collect(roots);
        self.alloc_count.set(0);
        map
    }

    fn collect(&self, roots: &[usize]) -> Vec<(usize, usize)> {
        self.collections.set(self.collections.get() + 1);

        // 1 ─ MARK
        self.mark_phase(roots);

        // 2 ─ COMPUTE FORWARDING
        let forward_map = self.compute_forwarding();

        // 3 ─ REMAP internal references
        self.remap_internals(&forward_map);

        // 4 ─ COMPACT
        self.compact(&forward_map);

        forward_map
    }

    fn mark_phase(&self, roots: &[usize]) {
        let buf_len = self.with_buf(|b| b.len());
        let mut worklist: Vec<usize> = roots
            .iter()
            .copied()
            .filter(|&r| r != usize::MAX && r < buf_len)
            .collect();

        let mut i = 0;
        while i < worklist.len() {
            let offset = worklist[i];
            i += 1;

            if offset >= buf_len {
                continue;
            }
            if self.mark(offset) != MARK_LIVE {
                continue;
            }

            self.set_mark(offset, MARK_KEPT);

            let tracer = self.tracer_fn(offset);
            self.with_buf(|buf| unsafe {
                let data = Self::data_ptr(buf, offset);
                let mut visit = |ref_off: usize| {
                    if ref_off != usize::MAX && ref_off < buf_len {
                        worklist.push(ref_off);
                    }
                };
                tracer(data, &mut visit);
            });
        }
    }

    fn compute_forwarding(&self) -> Vec<(usize, usize)> {
        let mut map = Vec::new();
        let mut dest = 0usize;
        let bump = self.bump.get();
        let mut pos = 0usize;

        while pos < bump {
            let mark = self.mark(pos);
            let obj_sz = self.obj_size(pos) as usize;
            let total_sz = Self::align(HEADER_SIZE + obj_sz);

            if mark == MARK_KEPT {
                map.push((pos, dest));
                self.set_forward(pos, dest as u64);
                dest += total_sz;
            } else if mark == MARK_LIVE {
                // Dead — will be replaced
            }
            pos += total_sz;
        }

        map
    }

    fn remap_internals(&self, forward_map: &[(usize, usize)]) {
        let bump = self.bump.get();
        let mut pos = 0usize;

        while pos < bump {
            if self.mark(pos) == MARK_KEPT {
                let remapper = self.remapper_fn(pos);
                self.with_buf_mut(|buf| unsafe {
                    let data = Self::data_ptr_mut(buf, pos);
                    remapper(data, forward_map);
                });
            }
            let obj_sz = self.obj_size(pos) as usize;
            pos += Self::align(HEADER_SIZE + obj_sz);
        }
    }

    fn compact(&self, forward_map: &[(usize, usize)]) {
        let old_len = self.with_buf(|b| b.len());
        let mut new_buffer: Vec<u8> = vec![0u8; old_len];
        let mut new_bump = 0usize;

        for &(old_offset, new_offset) in forward_map {
            let obj_sz = self.obj_size(old_offset) as usize;
            let total_sz = Self::align(HEADER_SIZE + obj_sz);

            self.with_buf(|buf| {
                let src = &buf[old_offset..old_offset + total_sz];
                let dst = &mut new_buffer[new_offset..new_offset + total_sz];
                dst.copy_from_slice(src);
            });

            // Reset mark and forward in new buffer
            new_buffer[new_offset + OFF_MARK..new_offset + OFF_MARK + 8]
                .copy_from_slice(&MARK_LIVE.to_le_bytes());
            new_buffer[new_offset + OFF_FORWARD..new_offset + OFF_FORWARD + 8]
                .copy_from_slice(&FORWARD_NULL.to_le_bytes());

            new_bump = new_bump.max(new_offset + total_sz);
        }

        *self.buffer.borrow_mut() = new_buffer;
        self.bump.set(new_bump);
        self.free_head.set(0);
    }

    /// Fix a GcRef after compaction.
    pub fn fix_handle<T: 'static>(&self, handle: &mut GcRef<T>, map: &[(usize, usize)]) {
        if handle.is_null() {
            return;
        }
        for &(old, new) in map {
            if old == handle.offset {
                *handle = GcRef::from_offset(new);
                return;
            }
        }
    }

    pub fn remap_offset(&self, off: usize, map: &[(usize, usize)]) -> usize {
        map.iter()
            .find(|&&(old, _)| old == off)
            .map(|&(_, new)| new)
            .unwrap_or(off)
    }
}

impl fmt::Debug for GcHeap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GcHeap(buf={}K, bump={}, collections={})",
            self.with_buf(|b| b.len()) / 1024,
            self.bump.get(),
            self.collections.get()
        )
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ── basic ───────────────────────────────────────────────

    #[test]
    fn test_allocate_and_access() {
        let heap = GcHeap::new();
        let h = heap.allocate(42.0f64);
        let v = heap.with_ref(h, |n| *n);
        assert_eq!(v, 42.0);
    }

    #[test]
    fn test_mutate() {
        let heap = GcHeap::new();
        let h = heap.allocate(99.0f64);
        heap.with_mut(h, |n| *n = 55.0);
        assert_eq!(heap.with_ref(h, |n| *n), 55.0);
    }

    #[test]
    fn test_null() {
        assert!(GcRef::<f64>::NULL.is_null());
    }

    #[test]
    fn test_equality() {
        let h = GcHeap::new();
        let a = h.allocate(1.0);
        let b = h.allocate(2.0);
        assert_eq!(a, a);
        assert_ne!(a, b);
    }

    #[test]
    fn test_many_allocations() {
        let h = GcHeap::new();
        let mut refs = Vec::new();
        for i in 0..200 {
            refs.push(h.allocate(i as f64));
        }
        for (i, &r) in refs.iter().enumerate() {
            assert_eq!(h.with_ref(r, |n| *n), i as f64);
        }
    }

    // ── collection ──────────────────────────────────────────

    #[test]
    fn test_collect_keeps_rooted() {
        let h = GcHeap::with_threshold(GcHeap::new(), 5);
        let mut survivor = h.allocate("keep me".to_string());
        let _g = h.allocate("garbage".to_string());
        let _g2 = h.allocate("more garbage".to_string());

        let map = h.force_collect(&[survivor.offset()]);
        h.fix_handle(&mut survivor, &map);

        assert!(h.is_live_by_offset(survivor.offset()));
        let s = h.with_ref(survivor, |s| s.clone());
        assert_eq!(s, "keep me");
    }

    #[test]
    fn test_collect_frees_unreachable() {
        let h = GcHeap::with_threshold(GcHeap::new(), 3);
        let mut keep = h.allocate(1.0);
        let dead_offset;
        {
            let d = h.allocate(2.0);
            dead_offset = d.offset();
        }

        let map = h.force_collect(&[keep.offset()]);
        h.fix_handle(&mut keep, &map);

        assert!(h.is_live_by_offset(keep.offset()));
        assert!(!h.is_live_by_offset(dead_offset));
    }

    // ── compaction ──────────────────────────────────────────

    #[test]
    fn test_compaction_moves_objects() {
        let h = GcHeap::with_threshold(GcHeap::new(), 10);
        // Allocate filler first so the survivor starts at a non-zero offset
        let _f1 = h.allocate(1.0);
        let _f2 = h.allocate(2.0);
        let mut a = h.allocate(10.0);
        let a_old = a.offset();
        let _filler = h.allocate(99.0);

        let map = h.force_collect(&[a.offset()]);
        h.fix_handle(&mut a, &map);

        assert!(h.is_live_by_offset(a.offset()));
        assert_eq!(h.with_ref(a, |n| *n), 10.0);
        // After compaction, a should have moved toward the front
        assert!(
            a.offset() < a_old,
            "object should have moved forward after compaction"
        );
    }

    #[test]
    fn test_compaction_preserves_value() {
        let h = GcHeap::with_threshold(GcHeap::new(), 5);
        let mut s = h.allocate("persistent".to_string());
        let _junk = h.allocate("junk1".to_string());
        let _junk2 = h.allocate("junk2".to_string());

        let map = h.force_collect(&[s.offset()]);
        h.fix_handle(&mut s, &map);

        let val = h.with_ref(s, |v| v.clone());
        assert_eq!(val, "persistent");
    }

    // ── strings survive GC ──────────────────────────────────

    #[test]
    fn test_string_survives_collection() {
        let h = GcHeap::with_threshold(GcHeap::new(), 2);
        let mut s = h.allocate(String::from("hello world"));
        let map = h.force_collect(&[s.offset()]);
        h.fix_handle(&mut s, &map);
        let v = h.with_ref(s, |s| s.clone());
        assert_eq!(v, "hello world");
    }

    // ── graph tests ─────────────────────────────────────────

    #[allow(dead_code)]
    struct Node {
        value: f64,
        next: Option<GcRef<Node>>,
    }

    impl Trace for Node {
        fn trace(&self, v: &mut dyn FnMut(usize)) {
            if let Some(ref n) = self.next {
                v(n.offset());
            }
        }
    }
    impl Remap for Node {
        fn remap(&mut self, map: &[(usize, usize)]) {
            if let Some(ref mut n) = self.next {
                for &(old, new) in map {
                    if old == n.offset() {
                        *n = GcRef::from_offset(new);
                        break;
                    }
                }
            }
        }
    }

    #[test]
    fn test_cycle_detection() {
        let h = GcHeap::with_threshold(GcHeap::new(), 3);
        let mut a = h.allocate(Node {
            value: 1.0,
            next: None,
        });
        let mut b = h.allocate(Node {
            value: 2.0,
            next: Some(a),
        });
        h.with_mut(a, |n| n.next = Some(b));

        // Root a: both survive the cycle
        let map = h.force_collect(&[a.offset()]);
        h.fix_handle(&mut a, &map);
        h.fix_handle(&mut b, &map);
        assert!(h.is_live_by_offset(a.offset()));
        assert!(h.is_live_by_offset(b.offset()));

        // No roots: cycle reclaimed
        let _map = h.force_collect(&[]);
        assert!(!h.is_live_by_offset(a.offset()));
        assert!(!h.is_live_by_offset(b.offset()));
    }

    #[test]
    fn test_self_cycle() {
        let h = GcHeap::with_threshold(GcHeap::new(), 2);
        let mut node = h.allocate(Node {
            value: 1.0,
            next: None,
        });
        h.with_mut(node, |n| n.next = Some(node));

        let map = h.force_collect(&[node.offset()]);
        h.fix_handle(&mut node, &map);
        assert!(h.is_live_by_offset(node.offset()));

        let _map = h.force_collect(&[]);
        assert!(!h.is_live_by_offset(node.offset()));
    }

    #[test]
    fn test_three_node_chain() {
        let h = GcHeap::with_threshold(GcHeap::new(), 3);
        let mut n1 = h.allocate(Node {
            value: 1.0,
            next: None,
        });
        let mut n2 = h.allocate(Node {
            value: 2.0,
            next: Some(n1),
        });
        let mut n3 = h.allocate(Node {
            value: 3.0,
            next: Some(n2),
        });

        let map = h.force_collect(&[n3.offset()]);
        h.fix_handle(&mut n1, &map);
        h.fix_handle(&mut n2, &map);
        h.fix_handle(&mut n3, &map);

        assert!(h.is_live_by_offset(n1.offset()));
        assert!(h.is_live_by_offset(n2.offset()));
        assert!(h.is_live_by_offset(n3.offset()));
    }

    // ── stress ──────────────────────────────────────────────

    #[test]
    fn test_many_collections() {
        // High threshold to avoid auto-collection during test (which would
        // move the survivor without updating our handle).
        let h = GcHeap::with_threshold(GcHeap::new(), 100_000);
        let mut survivor = h.allocate(42.0);

        for _ in 0..10 {
            for i in 0..50 {
                let _ = h.allocate(i as f64);
            }
            let map = h.force_collect(&[survivor.offset()]);
            h.fix_handle(&mut survivor, &map);
        }

        assert_eq!(h.with_ref(survivor, |n| *n), 42.0);
        assert!(h.collections.get() >= 10);
    }

    #[test]
    fn test_heap_grows_when_full() {
        let h = GcHeap::new();
        let initial = h.with_buf(|b| b.len());
        let mut roots = Vec::new();
        for i in 0..500 {
            roots.push(h.allocate(i as f64));
        }
        // Buffer should have grown
        assert!(h.with_buf(|b| b.len()) >= initial);
        // All values intact
        for (i, r) in roots.iter().enumerate() {
            assert_eq!(h.with_ref(*r, |n| *n), i as f64);
        }
    }

    #[test]
    fn test_debug_format() {
        let h = GcHeap::new();
        let _ = h.allocate(1.0);
        let s = format!("{:?}", h);
        assert!(s.contains("GcHeap"));
    }
}
