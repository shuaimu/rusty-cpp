#pragma once

// This file now just includes traits.hpp which provides the unified
// is_send, is_sync, and is_explicitly_send definitions.
// This file exists for backward compatibility.

#include "traits.hpp"

namespace rusty {

// ==================================================================
// HELPER MACRO FOR USER TYPES
// ==================================================================

// Convenience macro to mark types as Send
// Usage: RUSTY_MARK_SEND(MyType)
#define RUSTY_MARK_SEND(Type) \
    namespace rusty { \
        template<> struct is_explicitly_send<Type> : std::true_type {}; \
        template<> struct is_send<Type> : std::true_type {}; \
    }

// Template version - marks Template<T> as Send if T is Send
// Usage: RUSTY_MARK_SEND_TEMPLATE(MyContainer, T)
#define RUSTY_MARK_SEND_TEMPLATE(Template, T) \
    namespace rusty { \
        template<typename T> \
        struct is_explicitly_send<Template<T>> : is_send<T> {}; \
        template<typename T> \
        struct is_send<Template<T>> : is_send<T> {}; \
    }

} // namespace rusty
