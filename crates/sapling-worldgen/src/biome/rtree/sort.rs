//! MSVC STL's `std::sort` (introsort: insertion sort up to 32 elements, Tukey's ninther over 40,
//! the fat partition, heap sort when too deep), which BDS uses to build its biome tree. Elements
//! that compare equal end up in whatever order this algorithm leaves them, so it is reproduced
//! step by step rather than replaced by a stable sort.

const ISORT_MAX: usize = 32;

pub fn sort<T: Copy>(items: &mut [T], less: &impl Fn(&T, &T) -> bool) {
    let len = items.len() as isize;
    sort_unchecked(items, 0, items.len(), len, less);
}

fn sort_unchecked<T: Copy>(v: &mut [T], mut first: usize, mut last: usize, mut ideal: isize, less: &impl Fn(&T, &T) -> bool) {
    loop {
        if last - first <= ISORT_MAX {
            insertion_sort(v, first, last, less);
            return;
        }
        if ideal <= 0 {
            make_heap(v, first, last, less);
            sort_heap(v, first, last, less);
            return;
        }
        let (mid_first, mid_last) = partition_by_median_guess(v, first, last, less);
        ideal = (ideal >> 1) + (ideal >> 2);
        if mid_first - first < last - mid_last {
            sort_unchecked(v, first, mid_first, ideal, less);
            first = mid_last;
        } else {
            sort_unchecked(v, mid_last, last, ideal, less);
            last = mid_first;
        }
    }
}

fn insertion_sort<T: Copy>(v: &mut [T], first: usize, last: usize, less: &impl Fn(&T, &T) -> bool) {
    if first == last {
        return;
    }
    for mid in first + 1..last {
        let value = v[mid];
        if less(&value, &v[first]) {
            v.copy_within(first..mid, first + 1);
            v[first] = value;
        } else {
            let mut hole = mid;
            while less(&value, &v[hole - 1]) {
                v[hole] = v[hole - 1];
                hole -= 1;
            }
            v[hole] = value;
        }
    }
}

fn med3<T: Copy>(v: &mut [T], first: usize, mid: usize, last: usize, less: &impl Fn(&T, &T) -> bool) {
    if less(&v[mid], &v[first]) {
        v.swap(mid, first);
    }
    if less(&v[last], &v[mid]) {
        v.swap(last, mid);
        if less(&v[mid], &v[first]) {
            v.swap(mid, first);
        }
    }
}

fn guess_median<T: Copy>(v: &mut [T], first: usize, mid: usize, last: usize, less: &impl Fn(&T, &T) -> bool) {
    let count = last - first;
    if 40 < count {
        let step = (count + 1) >> 3;
        let two_step = step << 1;
        med3(v, first, first + step, first + two_step, less);
        med3(v, mid - step, mid, mid + step, less);
        med3(v, last - two_step, last - step, last, less);
        med3(v, first + step, mid, last - step, less);
    } else {
        med3(v, first, mid, last, less);
    }
}

fn partition_by_median_guess<T: Copy>(v: &mut [T], first: usize, last: usize, less: &impl Fn(&T, &T) -> bool) -> (usize, usize) {
    let mid = first + ((last - first) >> 1);
    guess_median(v, first, mid, last - 1, less);
    let mut pfirst = mid;
    let mut plast = pfirst + 1;
    while first < pfirst && !less(&v[pfirst - 1], &v[pfirst]) && !less(&v[pfirst], &v[pfirst - 1]) {
        pfirst -= 1;
    }
    while plast < last && !less(&v[plast], &v[pfirst]) && !less(&v[pfirst], &v[plast]) {
        plast += 1;
    }
    let mut gfirst = plast;
    let mut glast = pfirst;
    loop {
        while gfirst < last {
            if less(&v[pfirst], &v[gfirst]) {
            } else if less(&v[gfirst], &v[pfirst]) {
                break;
            } else if plast != gfirst {
                v.swap(plast, gfirst);
                plast += 1;
            } else {
                plast += 1;
            }
            gfirst += 1;
        }
        while first < glast {
            if less(&v[glast - 1], &v[pfirst]) {
            } else if less(&v[pfirst], &v[glast - 1]) {
                break;
            } else {
                pfirst -= 1;
                if pfirst != glast - 1 {
                    v.swap(pfirst, glast - 1);
                }
            }
            glast -= 1;
        }
        if glast == first && gfirst == last {
            return (pfirst, plast);
        }
        if glast == first {
            if plast != gfirst {
                v.swap(pfirst, plast);
            }
            plast += 1;
            v.swap(pfirst, gfirst);
            pfirst += 1;
            gfirst += 1;
        } else if gfirst == last {
            glast -= 1;
            pfirst -= 1;
            if glast != pfirst {
                v.swap(glast, pfirst);
            }
            plast -= 1;
            v.swap(pfirst, plast);
        } else {
            glast -= 1;
            v.swap(gfirst, glast);
            gfirst += 1;
        }
    }
}

/// `_Pop_heap_hole_by_index` then `_Push_heap_by_index`, as `_Make_heap_unchecked` and
/// `_Pop_heap_unchecked` use them.
fn adjust_heap<T: Copy>(v: &mut [T], base: usize, mut hole: usize, bottom: usize, value: T, less: &impl Fn(&T, &T) -> bool) {
    let top = hole;
    let mut idx = hole;
    let max_sequence_non_leaf = (bottom - 1) >> 1;
    while idx < max_sequence_non_leaf {
        idx = 2 * idx + 2;
        if less(&v[base + idx], &v[base + idx - 1]) {
            idx -= 1;
        }
        v[base + hole] = v[base + idx];
        hole = idx;
    }
    if idx == max_sequence_non_leaf && bottom % 2 == 0 {
        v[base + hole] = v[base + bottom - 1];
        hole = bottom - 1;
    }
    let mut parent;
    while top < hole {
        parent = (hole - 1) >> 1;
        if !less(&v[base + parent], &value) {
            break;
        }
        v[base + hole] = v[base + parent];
        hole = parent;
    }
    v[base + hole] = value;
}

fn make_heap<T: Copy>(v: &mut [T], first: usize, last: usize, less: &impl Fn(&T, &T) -> bool) {
    let bottom = last - first;
    let mut hole = bottom >> 1;
    while hole > 0 {
        hole -= 1;
        let value = v[first + hole];
        adjust_heap(v, first, hole, bottom, value, less);
    }
}

fn sort_heap<T: Copy>(v: &mut [T], first: usize, mut last: usize, less: &impl Fn(&T, &T) -> bool) {
    while last - first >= 2 {
        last -= 1;
        let value = v[last];
        v[last] = v[first];
        adjust_heap(v, first, 0, last - first, value, less);
    }
}
