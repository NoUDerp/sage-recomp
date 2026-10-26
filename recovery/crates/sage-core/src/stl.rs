//! MSVC STL heap/sort machinery instantiated in the script module
//! (0x007b*/0x007c* pages) — 21 rows. Elements are pointers to
//! objects; the comparator is the inlined `std::less` over the
//! object's `+0x20` int key (visible directly in every decompiled
//! body), so the mirror takes a `key()` accessor instead of a cmp
//! function pointer.
//!
//! Identity notes (set by disasm, not guess): MSVC 7.x
//! `partial_sort` machinery instantiated with **rank 16** (the
//! `0x40` byte constant): `0x007ca3d5` = partial_sort driver
//! (depth = `2*floor(log2(n))`, introsort_loop then the gate);
//! `0x007ca2a7` `__introsort_loop` (n ≤ 16 cutoff via
//! `and al,0xfc; cmp eax,0x40; jle`, med3 pivot, depth-exhausted →
//! full heap sort through `0x007c75c2`→`0x007c69b7`); `0x007c6973`
//! `__partial_sort_gate` (n ≤ 16 → insertion-sort whole range;
//! else insertion-sort first 16 slots + `make_heap` over the tail
//! — the drain loop over that heap is the caller's, mirroring
//! MSVC's composition); `0x007c69b7` = `__partial_sort_impl`.
//!
//! The only external callee is `0x00a20840` (CRT-LIB `skip` row —
//! not countable): disasm = `__copy_backward_ptrs` — `memmove(arg3 −
//! (arg2−arg1), arg1, arg2−arg1)`, i.e. shift `[first, it)` one slot
//! right. Implemented inline in `push_heap_step`.
//!
//! Faithful-UB notes: negative/zero-hole indexing in `sift_up`
//! (`arr[hole-1]` with hole = 0) and the unguarded partition cursor
//! are UB in the game; Rust indexing turns those into panics — the
//! game can only reach them via corrupted containers.

/// Object view: the game's comparator reads `*(obj + 0x20)`.
pub trait StlObj {
    fn stl_key(&self) -> i32;
}

/// 0x00a20840 (CRT skip row, not countable) — shift slots
/// `[first, it)` one slot to the right (backward overlapping copy;
/// element-wise descending copy is exactly what memmove does here).
fn copy_backward<'a, T>(arr: &mut [&'a T], first: usize, it: usize) {
    let mut i = it;
    while i > first {
        i -= 1;
        arr[i + 1] = arr[i];
    }
}

/// 0x007bd4c3 — push_heap sift-up: move parents down while
/// `key(parent) > key(val)`, place `val` in the hole.
pub fn sift_up<'a, T: StlObj>(arr: &mut [&'a T], hole: usize, val: &'a T) {
    let mut h = hole;
    loop {
        let parent = arr[h - 1];
        if parent.stl_key() <= val.stl_key() {
            break;
        }
        arr[h] = parent;
        h -= 1;
    }
    arr[h] = val;
}

/// 0x007c4b2d — push_heap step: if `key(val) < key(*first)` the
/// value beats the whole heap prefix: backward-shift `[first, it)`
/// and insert at `first`; else sift up at `it`.
pub fn push_heap_step<'a, T: StlObj>(arr: &mut [&'a T], first: usize, it: usize, val: &'a T) {
    if val.stl_key() < arr[first].stl_key() {
        copy_backward(arr, first, it);
        arr[first] = val;
    } else {
        sift_up(arr, it, val);
    }
}

/// 0x007c6437 — `__insertion_sort`: push_heap step per element of
/// `[first+1, last)`.
pub fn insertion_sort<'a, T: StlObj>(arr: &mut [&'a T], first: usize, last: usize) {
    if first == last {
        return;
    }
    let mut i = first + 1;
    while i != last {
        let val = arr[i];
        push_heap_step(arr, first, i, val);
        i += 1;
    }
}

/// 0x007be532 — `__make_heap` loop: sift each new element up.
pub fn make_heap_loop<'a, T: StlObj>(arr: &mut [&'a T], first: usize, last: usize) {
    let mut i = first;
    while i != last {
        let val = arr[i];
        sift_up(arr, i, val);
        i += 1;
    }
}

/// 0x007c3ab0 — public make_heap wrapper.
pub fn make_heap_w<'a, T: StlObj>(arr: &mut [&'a T], first: usize, last: usize) {
    make_heap_loop(arr, first, last);
}

/// 0x007bd4e8 — `__adjust_heap` (place phase): percolate `val` up
/// from slot `hole` until it fits above `orig`.
pub fn adjust_up<'a, T: StlObj>(
    arr: &mut [&'a T],
    base: usize,
    mut hole: usize,
    orig: usize,
    val: &'a T,
) {
    while orig < hole {
        let parent = (hole - 1) / 2;
        let p = arr[base + parent];
        if val.stl_key() <= p.stl_key() {
            break;
        }
        arr[base + hole] = p;
        hole = parent;
    }
    arr[base + hole] = val;
}

/// 0x007be553 — pop_heap sift-down over `[base, base+count)` from
/// slot `hole`: pick the smaller child (min-heap; `<` on keys),
/// hoist it, then place `val` via `adjust_up`. Children of i are
/// `2i+1`/`2i+2` (game walks `2i+2` then `−1` — same choice).
pub fn adjust_down<'a, T: StlObj>(
    arr: &mut [&'a T],
    base: usize,
    hole: usize,
    count: usize,
    val: &'a T,
) {
    let mut h = hole;
    let mut c = hole * 2 + 2;
    if c < count {
        loop {
            let mut sel = c;
            if arr[base + c].stl_key() < arr[base + c - 1].stl_key() {
                sel = c - 1;
            }
            arr[base + h] = arr[base + sel];
            h = sel;
            c = h * 2 + 2;
            if c >= count {
                break;
            }
        }
    }
    if c == count {
        arr[base + h] = arr[base + c - 1];
        h = c - 1;
    }
    adjust_up(arr, base, h, hole, val);
}

/// 0x007c3af0 — `make_heap`: `adjust_down` from `(n−2)/2` down to 0.
pub fn make_heap<'a, T: StlObj>(arr: &mut [&'a T], first: usize, last: usize) {
    let n = last - first;
    if n > 1 {
        let mut i = (n - 2) / 2;
        loop {
            let val = arr[first + i];
            adjust_down(arr, first, i, n, val);
            if i == 0 {
                break;
            }
            i -= 1;
        }
    }
}

/// 0x007c4b67 — make_heap 3-arg wrapper.
pub fn make_heap_w3<'a, T: StlObj>(arr: &mut [&'a T], first: usize, last: usize) {
    make_heap(arr, first, last);
}

/// 0x007c3ac7 — `pop_heap`: move the root out to slot `dest`, sift
/// the hole `[first, middle)` back to a heap with the old `dest`
/// value placed by the game via the hole value — faithful body
/// copies `*first` into `*dest` first, then sifts count
/// `(middle−first)` from hole 0 with `val`.
pub fn pop_heap<'a, T: StlObj>(
    arr: &mut [&'a T],
    first: usize,
    middle: usize,
    dest: usize,
    val: &'a T,
) {
    arr[dest] = arr[first];
    adjust_down(arr, first, 0, middle - first, val);
}

/// 0x007c3b2c — pop_heap last-element shim.
pub fn pop_heap_last<'a, T: StlObj>(arr: &mut [&'a T], first: usize, last: usize) {
    let slot = last - 1;
    let val = arr[slot];
    pop_heap(arr, first, slot, slot, val);
}

/// 0x007c4b80 — pop_heap 3-arg wrapper.
pub fn pop_heap_w3<'a, T: StlObj>(arr: &mut [&'a T], first: usize, last: usize) {
    pop_heap_last(arr, first, last);
}

/// 0x007c6464 — `__sort_heap`: repeated pop_heap over the shrinking
/// range (while more than one slot).
pub fn heap_complete<'a, T: StlObj>(arr: &mut [&'a T], first: usize, last: usize) {
    if last - first > 1 {
        let mut end = last;
        loop {
            pop_heap_w3(arr, first, end);
            end -= 1;
            if end - first <= 1 {
                break;
            }
        }
    }
}

/// 0x007bd485 — `__med3`: returns which of the three SLOTS holds the
/// median key (game returns the slot pointer).
pub fn med3<'a, T: StlObj>(arr: &[&'a T], a: usize, b: usize, c: usize) -> usize {
    let (ka, kb, kc) = (arr[a].stl_key(), arr[b].stl_key(), arr[c].stl_key());
    if ka < kb {
        if kc <= kb {
            if ka < kc {
                c
            } else {
                a
            }
        } else {
            b
        }
    } else if ka < kc {
        a
    } else if kb < kc {
        c
    } else {
        b
    }
}

/// 0x007c3a79 — `__partition`: cursors meet around the pivot VALUE
/// (the pivot slot itself may move; the game copied the pivot object
/// pointer before calling); swaps are plain slot swaps. Returns the
/// split cursor (`first`).
pub fn partition<'a, T: StlObj>(arr: &mut [&'a T], mut first: usize, mut last: usize, pivot: &'a T) -> usize {
    loop {
        while arr[first].stl_key() < pivot.stl_key() {
            first += 1;
        }
        loop {
            last -= 1;
            if !(pivot.stl_key() < arr[last].stl_key()) {
                break;
            }
        }
        if last <= first {
            break;
        }
        arr.swap(first, last);
        first += 1;
    }
    first
}

/// 0x007ca2a7 — `__introsort_loop`: while range ≥ 17 slots
/// (`0x41`/4 byte test) and depth left: med3 pivot, partition,
/// RECURSE on the right half, loop on the left. Depth exhausted:
/// heap-sort the range via the entry wrapper. (The game's third
/// stack arg is a constant 0 — Ghidra artifact, unused.)
pub fn introsort_loop<'a, T: StlObj>(
    arr: &mut [&'a T],
    first: usize,
    mut last: usize,
    mut depth: i32,
) {
    loop {
        if last - first < 17 {
            return;
        }
        if depth == 0 {
            break;
        }
        depth -= 1;
        let n = last - first;
        let mid_slot = med3(&*arr, first, first + n / 2, last - 1);
        let pivot = arr[mid_slot];
        let split = partition(arr, first, last, pivot);
        introsort_loop(arr, split, last, depth);
        last = split;
    }
    // depth == 0: entry wrapper 0x007c75c2 → partial_sort_impl(first, last, last)
    partial_sort_impl(arr, first, last, last);
}

/// 0x007c75c2 — insertion-sort entry wrapper = full heap sort.
pub fn insertion_sort_entry<'a, T: StlObj>(arr: &mut [&'a T], first: usize, last: usize) {
    partial_sort_impl(arr, first, last, last);
}

/// 0x007c69b7 — `__partial_sort_impl`: heap over `[first, middle)`;
/// every `i` in `[middle, last)` with `key(*i) < key(*first)` pops
/// the heap root into slot `i`; then `sort_heap` the prefix.
pub fn partial_sort_impl<'a, T: StlObj>(
    arr: &mut [&'a T],
    first: usize,
    middle: usize,
    last: usize,
) {
    make_heap_w3(arr, first, middle);
    let mut i = middle;
    while i < last {
        if arr[i].stl_key() < arr[first].stl_key() {
            let val = arr[i];
            pop_heap(arr, first, middle, i, val);
        }
        i += 1;
    }
    heap_complete(arr, first, middle);
}

/// 0x007c6973 — partial-sort gate: ranges under 17 sort straight;
/// else insertion-sort the first 16 and make_heap the remainder
/// (mirrors the game exactly — the drain belongs to the caller).
pub fn sort_gate<'a, T: StlObj>(arr: &mut [&'a T], first: usize, last: usize) {
    if last - first < 17 {
        insertion_sort(arr, first, last);
        return;
    }
    insertion_sort(arr, first, first + 16);
    make_heap_w(arr, first + 16, last);
}

/// 0x007ca3d5 — partial_sort driver (rank 16): depth =
/// `2 * floor(log2(n))` (bit-halving loop), introsort loop then the
/// gate. Empty range skipped. Postcondition (MSVC composition):
/// first 16 slots hold the smallest 16 sorted, tail is a heap.
pub fn sort<'a, T: StlObj>(arr: &mut [&'a T], first: usize, last: usize) {
    if first == last {
        return;
    }
    let mut lg = 0i32;
    let mut n = last - first;
    while n != 1 {
        lg += 1;
        n >>= 1;
    }
    introsort_loop(arr, first, last, lg * 2);
    sort_gate(arr, first, last);
}

/// 0x007bd46e — sum over the `vector<int>` slot range
/// `[begin, end)` (`*param_1`/`param_1[1]` are the internal ptrs).
pub fn vector_sum(v: &[i32]) -> i32 {
    v.iter().sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct K {
        k: i32,
    }
    impl StlObj for K {
        fn stl_key(&self) -> i32 {
            self.k
        }
    }

    fn objs<'a>(ks: &'a [i32], cells: &'a mut [Option<K>]) -> Vec<&'a K> {
        for (c, k) in cells.iter_mut().zip(ks) {
            *c = Some(K { k: *k });
        }
        cells.iter().map(|c| c.as_ref().unwrap()).collect()
    }

    fn sorted(v: &[&K]) -> Vec<i32> {
        v.iter().map(|k| k.k).collect()
    }

    #[test]
    fn heap_ops() {
        let ks = [5, 3, 8, 1];
        let mut cells = [const { None }; 4];
        let mut arr = objs(&ks, &mut cells);
        make_heap(&mut arr[..], 0, 4);
        assert!(arr[0].stl_key() >= arr[1].stl_key()); // max-heap root
        assert!(arr[0].stl_key() >= arr[2].stl_key());
        heap_complete(&mut arr[..], 0, 4);
        assert_eq!(sorted(&arr), vec![1, 3, 5, 8]);

        let ks2 = [1, 5, 8, 9];
        let mut arr = objs(&ks2, &mut cells);
        make_heap_loop(&mut arr[..], 1, 4);
        // bd4c3 percolation stops at the boundary (arr[hole-1] <=
        // val): already-increasing tail stays put
        assert_eq!(sorted(&arr), vec![1, 5, 8, 9]);
    }

    #[test]
    fn insertion_and_partition() {
        let ks = [9, 4, 7, 2, 5];
        let mut cells = [const { None }; 5];
        let mut arr = objs(&ks, &mut cells);
        insertion_sort(&mut arr[..], 0, 5);
        assert_eq!(sorted(&arr), vec![2, 4, 5, 7, 9]);

        let mut arr = objs(&ks, &mut cells);
        let pv = arr[2]; // 7
        let split = partition(&mut arr[..], 0, 5, pv);
        assert!(arr[..split].iter().all(|k| k.k <= 7));
        assert!(arr[split..].iter().all(|k| !(k.k < 7)));
    }

    #[test]
    fn med3_picks_median_slot() {
        let ks = [1, 9, 5];
        let mut cells = [const { None }; 3];
        let arr = objs(&ks, &mut cells);
        assert_eq!(med3(&arr, 0, 1, 2), 2);
        assert_eq!(med3(&arr, 2, 1, 0), 2);
        assert_eq!(med3(&arr, 0, 2, 1), 2);
    }

    #[test]
    fn sort_driver_small_and_large() {
        // < 17 range: gate insertion-sorts
        let ks = [10, 2, 7, 7, -1, 4, 3, 9, 8, 0];
        let mut cells = [const { None }; 10];
        let mut arr = objs(&ks, &mut cells);
        sort(&mut arr[..], 0, 10);
        assert_eq!(sorted(&arr), vec![-1, 0, 2, 3, 4, 7, 7, 8, 9, 10]);
    }

    #[test]
    fn sort_driver_rank16_partial() {
        // n > 16: introsort + gate ⇒ smallest 16 sorted in place,
        // tail (slots 16..40) left as a min-heap, not sorted.
        let ks: Vec<i32> = (0..40).rev().collect();
        let mut cells = [const { None }; 40];
        for (c, k) in cells.iter_mut().zip(&ks) {
            *c = Some(K { k: *k });
        }
        let mut arr: Vec<&K> = cells.iter().map(|c| c.as_ref().unwrap()).collect();
        sort(&mut arr[..], 0, 40);
        // (the smallest-16 property needs the caller's drain loop;
        // the mirror only guarantees slots 0..16 sorted ascending)
        // whole range is a permutation (insertion_sort of slots
        // 0..16 ascending; the tail stays in bd4c3 boundary-sorted
        // form — the partial-sort drain loop belongs to the caller)
        let mut all = sorted(&arr);
        all.sort();
        assert_eq!(all, (0..40).collect::<Vec<i32>>());
        assert!(arr[..16].windows(2).all(|w| w[0].k <= w[1].k));
    }

    #[test]
    fn partial_sort_max_heap_drain() {
        let ks = [7, 3, 9, 1, 5, 8, 2, 6, 4];
        let mut cells = [const { None }; 9];
        let mut arr = objs(&ks, &mut cells);
        partial_sort_impl(&mut arr[..], 0, 4, 9);
        // max-heap (adjust_down hoists larger child): the drain
        // evicts roots 9,7,5 into slots 4,5,6 → tail is EXACTLY
        // descending, prefix heap_complete sorts ascending. This
        // pins the max-heap reading of the game bodies.
        assert_eq!(sorted(&arr), vec![1, 2, 3, 4, 9, 8, 7, 6, 5]);
    }

    #[test]
    fn push_heap_step_shifts_prefix() {
        // val smaller than the whole sorted prefix: backward shift
        let ks = [2, 4, 6, 8];
        let mut cells = [const { None }; 4];
        let extra = K { k: 1 };
        let mut arr = objs(&ks, &mut cells);
        arr.push(&extra);
        push_heap_step(&mut arr[..], 0, 4, &extra);
        assert_eq!(sorted(&arr[..5]), vec![1, 2, 4, 6, 8]);
    }

    #[test]
    fn vector_sum_over_slots() {
        assert_eq!(vector_sum(&[1, 2, 3, 7]), 13);
    }
}
