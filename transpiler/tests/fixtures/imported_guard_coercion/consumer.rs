
#[cfg_attr(any(), cpp_import_namespace(example))]
use crate::provider::{FactoryBase, FactoryProxy};
use std::sync::{Arc, Mutex};
pub struct State {
    factory: Mutex<Option<Arc<Mutex<FactoryProxy>>>>,
}
pub fn exercise(factory: FactoryProxy) -> i32 {
    let state = State {
        factory: Mutex::new(Some(Arc::new(Mutex::new(factory)))),
    };
    let owner = state.factory.lock().unwrap().clone();
    let owner = owner.unwrap();
    let result: i32 = {
        let mut guard = owner.lock().unwrap();
        let bound: &mut Box<dyn FactoryBase> = &mut guard;
        bound.next()
    };
    result
}
