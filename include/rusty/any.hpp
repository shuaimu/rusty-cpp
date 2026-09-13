#ifndef RUSTY_ANY_HPP
#define RUSTY_ANY_HPP

#include <exception>
#include <memory>
#include <type_traits>
#include <typeinfo>
#include <utility>
#include <rusty/option.hpp>

namespace rusty::any_types {

// Owning type erasure for Rust's Box<dyn Any>. Unlike std::any, the stored
// value need not be copyable. Borrowed downcasts keep the owner's lifetime.
class BoxAny {
    struct Value {
        virtual ~Value() = default;
        virtual const std::type_info& type() const noexcept = 0;
        virtual void* address() noexcept = 0;
        virtual const void* address() const noexcept = 0;
    };
    template<typename T>
    struct Stored final : Value {
        T value;
        explicit Stored(T value) : value(std::move(value)) {}
        const std::type_info& type() const noexcept override { return typeid(T); }
        void* address() noexcept override { return &value; }
        const void* address() const noexcept override { return &value; }
    };
    std::unique_ptr<Value> value_;
    std::exception_ptr foreign_exception_;
    explicit BoxAny(std::unique_ptr<Value> value) : value_(std::move(value)) {}

public:
    BoxAny() = delete;
    BoxAny(const BoxAny&) = delete;
    BoxAny& operator=(const BoxAny&) = delete;
    BoxAny(BoxAny&&) noexcept = default;
    BoxAny& operator=(BoxAny&&) noexcept = default;

    const BoxAny* operator->() const noexcept { return this; }
    BoxAny* operator->() noexcept { return this; }

    template<typename T>
    static BoxAny new_(T value) {
        return BoxAny(std::make_unique<Stored<T>>(std::move(value)));
    }
    bool has_value() const noexcept { return static_cast<bool>(value_); }

    template<typename T>
    bool is() const noexcept {
        return value_ && value_->type() == typeid(T);
    }
    template<typename T>
    rusty::Option<const T&> downcast_ref() const {
        if (!is<T>()) return rusty::None;
        return rusty::Option<const T&>(*static_cast<const T*>(value_->address()));
    }
    template<typename T>
    rusty::Option<T&> downcast_mut() {
        if (!is<T>()) return rusty::None;
        return rusty::Option<T&>(*static_cast<T*>(value_->address()));
    }

    // A foreign exception can expose a String diagnostic through Any while
    // resume_unwind still rethrows the original C++ exception and its type.
    void retain_foreign_exception(std::exception_ptr exception) {
        foreign_exception_ = std::move(exception);
    }
    std::exception_ptr foreign_exception() const { return foreign_exception_; }
};

} // namespace rusty::any_types
#endif
