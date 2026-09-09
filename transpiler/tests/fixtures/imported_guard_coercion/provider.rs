
pub trait FactoryBase: Send {
    fn next(&mut self) -> i32;
}
pub type FactoryProxy = Box<dyn FactoryBase>;
pub struct Counter {
    pub value: i32,
}
impl FactoryBase for Counter {
    fn next(&mut self) -> i32 {
        self.value += 1;
        self.value
    }
}
pub fn make_factory() -> FactoryProxy {
    Box::new(Counter { value: 40 })
}
