//! Root of the --crate-graph fixture: implements a dependency's dyn trait.

pub struct RootBackend {
    pub k: u32,
}

impl dep_core::Backend for RootBackend {
    fn poll(&mut self) -> u32 {
        self.k
    }
}

pub fn run() -> u32 {
    let mut widget = dep_core::Widget::new(3);
    widget.bump();
    let backend: Box<dyn dep_core::Backend> = Box::new(RootBackend { k: 5 });
    let ids = [dep_core::Id(1), dep_core::Id(2), dep_core::Id(1)];
    dep_core::total(&widget)
        + dep_core::with_backend(backend)
        + dep_core::scan(&ids) as u32
        + dep_core::sched_total() as u32
}
