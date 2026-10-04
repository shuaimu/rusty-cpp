//! Insert, remove, advance and fire cases over lion-slab and
//! lion-timer-wheel, compiled twice: by rustc (`examples/run.rs`, whose
//! output ../regen.sh records in ../MANIFEST.toml) and by the transpiler's
//! C++ (`transpiler/tests/lion_parity.rs`). Every case folds what it observed
//! into one number, so the two runs agree only if every intermediate
//! observation agrees. Nothing here depends on `HashMap` iteration order (the
//! slab and the timer wheel's maps are std `HashMap`s since Lion's U1): the
//! one case that iterates, `slab_values`, only sums.

use lion_slab::Slab;
use lion_timer_wheel::TimerWheel;

const MODULUS: u64 = 1_000_000_007;

/// An order-sensitive fold of one observation into the running value.
fn mix(acc: u64, value: u64) -> u64 {
    (acc * 131 + value % MODULUS) % MODULUS
}

fn observe(acc: u64, value: Option<u64>) -> u64 {
    match value {
        Some(v) => mix(mix(acc, 1), v),
        None => mix(acc, 2),
    }
}

fn slab_get(slab: &Slab<u64>, key: u64) -> Option<u64> {
    match slab.get(key) {
        Some(v) => Some(*v),
        None => None,
    }
}

/// Insert five keys, read each back, and read two keys never inserted.
pub fn slab_insert_get() -> u64 {
    let mut slab: Slab<u64> = Slab::new();
    let mut key: u64 = 1;
    while key <= 5 {
        slab.insert(key * 7, key * 10);
        key = key + 1;
    }
    let mut acc: u64 = 0;
    key = 1;
    while key <= 5 {
        acc = observe(acc, slab_get(&slab, key * 7));
        key = key + 1;
    }
    acc = observe(acc, slab_get(&slab, 0));
    acc = observe(acc, slab_get(&slab, 36));
    acc
}

/// Remove a present key twice and a missing key once; the other keys stay.
pub fn slab_remove() -> u64 {
    let mut slab: Slab<u64> = Slab::new();
    slab.insert(1, 100);
    slab.insert(2, 200);
    slab.insert(3, 300);
    slab.insert(4, 400);
    let mut acc: u64 = 0;
    acc = observe(acc, slab.remove(2));
    acc = observe(acc, slab.remove(2));
    acc = observe(acc, slab.remove(99));
    let mut key: u64 = 1;
    while key <= 4 {
        acc = observe(acc, slab_get(&slab, key));
        key = key + 1;
    }
    // A removed key can be inserted again.
    slab.insert(2, 222);
    acc = observe(acc, slab_get(&slab, 2));
    acc
}

/// Insert over an existing key, then update a value in place.
pub fn slab_overwrite_and_get_mut() -> u64 {
    let mut slab: Slab<u64> = Slab::new();
    slab.insert(7, 1);
    slab.insert(7, 2);
    let mut acc: u64 = 0;
    acc = observe(acc, slab_get(&slab, 7));
    if let Some(v) = slab.get_mut(7) {
        *v = *v + 40;
    }
    let missing = if slab.get_mut(8).is_none() { 1 } else { 0 };
    acc = mix(acc, missing);
    acc = observe(acc, slab_get(&slab, 7));
    acc
}

/// Many inserts and removes; the surviving values are summed through
/// `values()` (order-free) and read back key by key.
pub fn slab_values() -> u64 {
    let mut slab: Slab<u64> = Slab::new();
    let mut key: u64 = 0;
    while key < 1000 {
        slab.insert(key, key * 3 + 1);
        key = key + 1;
    }
    key = 0;
    while key < 1000 {
        if key % 3 == 0 {
            let _ = slab.remove(key);
        }
        key = key + 1;
    }
    let mut sum: u64 = 0;
    let mut count: u64 = 0;
    for value in slab.values() {
        sum = sum + *value;
        count = count + 1;
    }
    let mut acc = mix(mix(0, sum), count);
    key = 0;
    while key < 1000 {
        acc = observe(acc, slab_get(&slab, key));
        key = key + 1;
    }
    acc
}

/// Deadlines on each of the wheel's four levels (deltas below 2^8, 2^16 and
/// 2^24, and above), read back before anything fires.
pub fn wheel_insert_levels() -> u64 {
    let mut wheel = TimerWheel::new();
    let mut acc: u64 = 0;
    acc = mix(acc, if wheel.is_empty() { 1 } else { 0 });
    acc = observe(acc, wheel.next_deadline());
    wheel.insert(1, 200);
    wheel.insert(2, 40_000);
    wheel.insert(3, 9_000_000);
    wheel.insert(4, 3_000_000_000);
    wheel.insert(5, 150);
    let mut rid: u64 = 1;
    while rid <= 6 {
        acc = observe(acc, wheel.get_deadline(rid));
        rid = rid + 1;
    }
    acc = observe(acc, wheel.next_deadline());
    acc = mix(acc, if wheel.is_empty() { 1 } else { 0 });
    acc
}

/// Remove the earliest timer, a later one and a missing one; the next
/// deadline moves on and the removed timers never fire.
pub fn wheel_remove() -> u64 {
    let mut wheel = TimerWheel::new();
    wheel.insert(10, 30);
    wheel.insert(11, 60);
    wheel.insert(12, 90);
    wheel.insert(13, 70_000);
    let mut acc: u64 = 0;
    acc = observe(acc, wheel.next_deadline());
    wheel.remove(10);
    acc = observe(acc, wheel.next_deadline());
    wheel.remove(12);
    wheel.remove(999);
    acc = observe(acc, wheel.get_deadline(10));
    acc = observe(acc, wheel.get_deadline(11));
    acc = observe(acc, wheel.get_deadline(12));
    let mut now: u64 = 0;
    while now <= 80_000 {
        while let Some(fired) = wheel.try_pop_expired(now) {
            acc = mix(mix(acc, fired), now);
        }
        now = now + 50;
    }
    acc = mix(acc, if wheel.is_empty() { 1 } else { 0 });
    acc = observe(acc, wheel.next_deadline());
    acc
}

/// Level-0 timers, ties included, fired by small steps of `now`: what
/// fires, in which order, and at which `now`.
pub fn wheel_fire_order() -> u64 {
    let mut wheel = TimerWheel::new();
    wheel.insert(1, 5);
    wheel.insert(2, 5);
    wheel.insert(3, 3);
    wheel.insert(4, 9);
    wheel.insert(5, 0);
    wheel.insert(6, 200);
    let mut acc: u64 = 0;
    let mut now: u64 = 0;
    while now <= 210 {
        let mut fired_now: u64 = 0;
        while let Some(fired) = wheel.try_pop_expired(now) {
            acc = mix(mix(acc, fired), now);
            fired_now = fired_now + 1;
        }
        acc = mix(acc, fired_now);
        now = now + 1;
    }
    // Not yet due: nothing fires and the deadline stays.
    wheel.insert(7, 1_000);
    acc = observe(acc, wheel.try_pop_expired(999));
    acc = observe(acc, wheel.next_deadline());
    acc = observe(acc, wheel.try_pop_expired(1_000));
    acc
}

/// Timers on levels 1-3 brought due by large advances, so they cascade
/// down the wheel before they fire; then small steps past each deadline.
pub fn wheel_advance_cascade() -> u64 {
    let mut wheel = TimerWheel::new();
    let deadlines: [u64; 8] = [300, 301, 65_000, 65_536, 70_000, 16_777_215, 16_777_300, 40_000_000];
    let mut i: usize = 0;
    while i < 8 {
        wheel.insert(i as u64 + 100, deadlines[i]);
        i = i + 1;
    }
    let mut acc: u64 = 0;
    let steps: [u64; 9] = [256, 299, 300, 64_999, 65_600, 66_000, 16_777_216, 16_800_000, 40_000_000];
    let mut s: usize = 0;
    while s < 9 {
        let now = steps[s];
        while let Some(fired) = wheel.try_pop_expired(now) {
            acc = mix(mix(acc, fired), now);
        }
        acc = observe(acc, wheel.next_deadline());
        s = s + 1;
    }
    acc = mix(acc, if wheel.is_empty() { 1 } else { 0 });
    acc
}

/// Re-inserting a timer moves its deadline (earlier and later); it fires
/// once, at the last deadline it was given.
pub fn wheel_reschedule() -> u64 {
    let mut wheel = TimerWheel::new();
    wheel.insert(1, 100);
    wheel.insert(2, 120);
    wheel.insert(1, 50);
    let mut acc: u64 = 0;
    acc = observe(acc, wheel.next_deadline());
    wheel.insert(1, 500);
    acc = observe(acc, wheel.next_deadline());
    acc = observe(acc, wheel.get_deadline(1));
    let mut now: u64 = 0;
    while now <= 600 {
        while let Some(fired) = wheel.try_pop_expired(now) {
            acc = mix(mix(acc, fired), now);
        }
        now = now + 10;
    }
    acc = mix(acc, if wheel.is_empty() { 1 } else { 0 });
    acc
}
