use cpp::interop::host as cpp_host;

pub struct RustAccumulator {
    total: i32,
}

impl RustAccumulator {
    pub fn new(seed: i32) -> Self {
        Self { total: seed }
    }

    // This demonstrates Rust -> C++ member-call interop:
    // `cpp_host::Counter::add(counter, delta)` lowers to `counter.add(delta)`.
    // The free call uses the index's export namespace (`interop::api`) while
    // retaining `interop.host` as the imported named-module identity.
    pub unsafe fn pull_from_cpp<T>(&mut self, counter: &mut T, delta: i32) -> i32 {
        let updated = cpp_host::Counter::add(counter, delta);
        self.total += updated;
        self.total
    }

    pub fn add_cpp_bias(&mut self) -> i32 {
        self.total = unsafe { cpp_host::apply_bias(self.total) };
        self.total
    }

    pub fn bump(&mut self, step: i32) -> i32 {
        self.total += step;
        self.total
    }

    pub fn current(&self) -> i32 {
        self.total
    }
}
