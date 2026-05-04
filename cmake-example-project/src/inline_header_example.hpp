#pragma once

// Inline Rust DSL example in a header-like file.
// V1 rule: this is safe for shared APIs because other translation units can include it.
// In inline mode, includes are author-managed.
// This example demonstrates:
// - a free function (`greet`)
// - a struct with inherent methods (`Counter`)
// - a trait and trait method implementation (`Named::describe`)
// - a function calling the trait method (`describe_counter`)

#include <cstdint>
#include <tuple>
#include <string_view>
#include <utility>
#include <rusty/rusty.hpp>

#if RUSTYCPP_RUST
pub trait Named {
    fn describe(&self) -> String;
}

pub struct Counter {
    value: i32,
}

impl Counter {
    pub fn new(value: i32) -> Self {
        Self { value }
    }

    pub fn inc(&mut self, delta: i32) {
        self.value += delta;
    }

    pub fn value(&self) -> i32 {
        self.value
    }
}

impl Named for Counter {
    fn describe(&self) -> String {
        let mut out = String::from("Counter(");
        out.push_str(self.value.to_string().as_str());
        out.push_str(")");
        out
    }
}

pub fn describe_counter(counter: &Counter) -> String {
    counter.describe()
}

pub fn greet(name: &str) -> String {
    let mut out = String::from("Hello, ");
    out.push_str(name);
    out
}
#endif
/*RUSTYCPP:GEN-BEGIN id=cmake_example.header.greet version=1 rust_sha256=fad014ce14890a11f4d04a13707a51b2eefc1481ad3c7f11294ac94292b17ee3*/
struct Counter;
rusty::String describe_counter(const Counter& counter);
rusty::String greet(std::string_view name);

PRO_DEF_MEM_DISPATCH(MemNamed_describe, describe);

struct NamedFacade : pro::facade_builder
    ::add_convention<MemNamed_describe, rusty::String() const>
    ::build {};

struct Counter {
    int32_t value_field;

    static Counter new_(int32_t value);
    void inc(int32_t delta);
    int32_t value() const;
    rusty::String describe() const;
};

rusty::String describe_counter(const Counter& counter) {
    return counter.describe();
}

rusty::String greet(std::string_view name) {
    auto out = rusty::String::from("Hello, ");
    out.push_str(name);
    return std::move(out);
}


Counter Counter::new_(int32_t value) {
    return Counter{.value_field = std::move(value)};
}

void Counter::inc(int32_t delta) {
    this->value_field += delta;
}

int32_t Counter::value() const {
    return this->value_field;
}

rusty::String Counter::describe() const {
    auto out = rusty::String::from("Counter(");
    out.push_str(rusty::to_string(this->value_field).as_str());
    out.push_str(")");
    return std::move(out);
}
/*RUSTYCPP:GEN-END id=cmake_example.header.greet*/
