// rc_port stub module — Phase B/C bridge.
//
// The full transpiled `rc_port.cppm` (5094 LOC vendored from
// library/alloc/src/rc.rs) doesn't yet compile clean — it hits ~7
// classes of transpiler-side issues catalogued in
// docs/rc_port/STATUS.md (Rc::is trait method, Layout::for_value_raw,
// __TemplateArgs not instantiated for std::string_view/span, etc.).
//
// To get a working `rc_port` library that compiles + has a smoke
// test, this stub re-exports the hand-written `rusty::Rc<T>` from
// `rusty/rc.hpp` under the `rc_port` namespace, mapping the
// two-template-arg `Rc<T, A>` API to the single-arg hand version
// (allocator A is ignored — only Global is supported).
//
// When the transpiler fixes are in, drop this stub and switch CMake
// back to glob the full `rc_port.cppm`.

module;

#include <rusty/rc.hpp>
#include <rusty/alloc.hpp>

export module rc_port;

namespace rc_port {

// Two-template-arg Rc<T, A> mapped to single-arg rusty::Rc<T>. The
// allocator A is ignored at the stub level (only Global is supported).
export template<typename T, typename A = ::rusty::alloc::Global>
using Rc = ::rusty::Rc<T>;

export template<typename T, typename A = ::rusty::alloc::Global>
using Weak = ::rusty::rc::Weak<T>;

} // namespace rc_port
