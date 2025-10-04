#ifndef RUSTY_REFCELL_HPP
#define RUSTY_REFCELL_HPP

#include <memory>
#include <stdexcept>
#include <cassert>

// RefCell<T> - Interior mutability with runtime borrow checking
// Provides interior mutability with dynamic borrow checking
//
// Guarantees:
// - Single-threaded only (not thread-safe)
// - Runtime borrow checking (panics on violation)
// - Multiple immutable borrows OR single mutable borrow
// - Borrows tracked via RAII guards

// @safe
namespace rusty {

template<typename T>
class RefCell;

// Forward declarations for borrow guards
template<typename T>
class Ref;

template<typename T>
class RefMut;

// BorrowState - tracks the current borrow state
enum class BorrowState : int {
    Unborrowed = 0,
    Reading = 1,     // Positive values = number of readers
    Writing = -1     // Negative value = writing
};

template<typename T>
class RefCell {
private:
    mutable T value;
    mutable int borrow_state;  // 0 = unborrowed, >0 = # readers, -1 = writing
    
    friend class Ref<T>;
    friend class RefMut<T>;
    
    void add_reader() const {
        if (borrow_state < 0) {
            throw std::runtime_error("RefCell<T>: already mutably borrowed");
        }
        borrow_state++;
    }
    
    void remove_reader() const {
        assert(borrow_state > 0);
        borrow_state--;
    }
    
    void add_writer() const {
        if (borrow_state != 0) {
            if (borrow_state > 0) {
                throw std::runtime_error("RefCell<T>: already immutably borrowed");
            } else {
                throw std::runtime_error("RefCell<T>: already mutably borrowed");
            }
        }
        borrow_state = -1;
    }
    
    void remove_writer() const {
        assert(borrow_state == -1);
        borrow_state = 0;
    }
    
public:
    // Constructors
    RefCell() : value(), borrow_state(0) {}
    explicit RefCell(T val) : value(std::move(val)), borrow_state(0) {}
    
    // Factory method (Rust-style)
    static RefCell<T> new_(T value) {
        return RefCell<T>(std::move(value));
    }
    
    // Destructor - checks for leaked borrows in debug mode
    ~RefCell() {
#ifdef DEBUG
        if (borrow_state != 0) {
            // In Rust this would panic
            assert(false && "RefCell<T> dropped while borrowed");
        }
#endif
    }
    
    // Immutably borrow the value
    // @lifetime: (&'a) -> Ref<'a, T>
    Ref<T> borrow() const {
        add_reader();
        return Ref<T>(*this);
    }
    
    // Mutably borrow the value
    // @lifetime: (&'a mut) -> RefMut<'a, T>
    RefMut<T> borrow_mut() const {
        add_writer();
        return RefMut<T>(*this);
    }
    
    // Try to immutably borrow (returns true on success)
    // @lifetime: (&'a) -> bool
    bool can_borrow() const {
        return borrow_state >= 0;
    }
    
    // Try to mutably borrow (returns true on success)
    // @lifetime: (&'a mut) -> bool
    bool can_borrow_mut() const {
        return borrow_state == 0;
    }
    
    // Replace the value
    // @lifetime: (&'a, T) -> T
    T replace(T new_value) const {
        if (borrow_state != 0) {
            throw std::runtime_error("RefCell<T>: cannot replace while borrowed");
        }
        T old = std::move(value);
        value = std::move(new_value);
        return old;
    }
    
    // Swap values with another RefCell
    // @lifetime: (&'a, &'a) -> void
    void swap(RefCell& other) const {
        if (borrow_state != 0 || other.borrow_state != 0) {
            throw std::runtime_error("RefCell<T>: cannot swap while borrowed");
        }
        std::swap(value, other.value);
    }
    
    // Take the value, leaving default in its place
    template<typename U = T>
    typename std::enable_if_t<std::is_default_constructible_v<U>, T>
    take() const {
        return replace(T{});
    }
    
    // Get a copy of the value (only for copyable types)
    template<typename U = T>
    typename std::enable_if_t<std::is_copy_constructible_v<U>, T>
    get() const {
        return borrow().clone();
    }
    
    // No copy or move - RefCell itself is not copyable/movable
    RefCell(const RefCell&) = delete;
    RefCell& operator=(const RefCell&) = delete;
    RefCell(RefCell&&) = delete;
    RefCell& operator=(RefCell&&) = delete;
};

// Ref<T> - RAII guard for immutable borrow
template<typename T>
class Ref {
private:
    const RefCell<T>* cell;
    
    friend class RefCell<T>;
    explicit Ref(const RefCell<T>& c) : cell(&c) {}
    
public:
    // Destructor - releases the borrow
    ~Ref() {
        if (cell) {
            cell->remove_reader();
        }
    }
    
    // Move constructor
    Ref(Ref&& other) noexcept : cell(other.cell) {
        other.cell = nullptr;
    }
    
    // No copy constructor - can't duplicate borrows
    Ref(const Ref&) = delete;
    Ref& operator=(const Ref&) = delete;
    Ref& operator=(Ref&&) = delete;
    
    // Access the value
    const T& operator*() const {
        return cell->value;
    }
    
    const T* operator->() const {
        return &cell->value;
    }
    
    // Clone the value (only for copyable types)
    template<typename U = T>
    typename std::enable_if_t<std::is_copy_constructible_v<U>, T>
    clone() const {
        return cell->value;
    }
};

// RefMut<T> - RAII guard for mutable borrow
template<typename T>
class RefMut {
private:
    const RefCell<T>* cell;
    
    friend class RefCell<T>;
    explicit RefMut(const RefCell<T>& c) : cell(&c) {}
    
public:
    // Destructor - releases the borrow
    ~RefMut() {
        if (cell) {
            cell->remove_writer();
        }
    }
    
    // Move constructor
    RefMut(RefMut&& other) noexcept : cell(other.cell) {
        other.cell = nullptr;
    }
    
    // No copy constructor - can't duplicate borrows
    RefMut(const RefMut&) = delete;
    RefMut& operator=(const RefMut&) = delete;
    RefMut& operator=(RefMut&&) = delete;
    
    // Access the value
    T& operator*() const {
        return const_cast<T&>(cell->value);
    }
    
    T* operator->() const {
        return const_cast<T*>(&cell->value);
    }
    
    // Clone the value (only for copyable types)
    template<typename U = T>
    typename std::enable_if_t<std::is_copy_constructible_v<U>, T>
    clone() const {
        return cell->value;
    }
};

// Helper functions
template<typename T>
RefCell<T> make_refcell(T value) {
    return RefCell<T>(std::move(value));
}

} // namespace rusty

#endif // RUSTY_REFCELL_HPP