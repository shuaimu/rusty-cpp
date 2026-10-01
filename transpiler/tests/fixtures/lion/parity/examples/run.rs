// The Rust half of the Lion parity cases: ../regen.sh records this output in
// ../../MANIFEST.toml, and tests/lion_parity.rs prints the same lines from C++.
fn main() {
    println!("slab_insert_get {}", lion_parity::slab_insert_get());
    println!("slab_remove {}", lion_parity::slab_remove());
    println!("slab_overwrite_and_get_mut {}", lion_parity::slab_overwrite_and_get_mut());
    println!("slab_values {}", lion_parity::slab_values());
    println!("wheel_insert_levels {}", lion_parity::wheel_insert_levels());
    println!("wheel_remove {}", lion_parity::wheel_remove());
    println!("wheel_fire_order {}", lion_parity::wheel_fire_order());
    println!("wheel_advance_cascade {}", lion_parity::wheel_advance_cascade());
    println!("wheel_reschedule {}", lion_parity::wheel_reschedule());
}
