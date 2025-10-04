#ifndef RUSTY_WEAK_HPP
#define RUSTY_WEAK_HPP

#include "rc/weak.hpp"
#include "sync/weak.hpp"

namespace rusty {

// Back-compat alias matching older API defaults
template<typename T>
using Weak = rc::Weak<T>;

// Convenience downgrade helpers in root namespace
template<typename T>
auto downgrade(const Rc<T>& rc) -> rc::Weak<T> {
    return rc::downgrade(rc);
}

template<typename T>
auto downgrade(const Arc<T>& arc) -> sync::Weak<T> {
    return sync::downgrade(arc);
}

} // namespace rusty

#endif // RUSTY_WEAK_HPP
