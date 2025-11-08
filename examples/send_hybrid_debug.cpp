// Debug the hybrid Send trait

#include <type_traits>
#include <iostream>

namespace rusty {

template<typename T>
struct is_explicitly_not_send : std::false_type {};

template<typename T, typename = void>
struct has_send_marker : std::false_type {};

template<typename T>
struct has_send_marker<T, std::void_t<decltype(T::is_send)>> : std::true_type {};

template<typename T>
struct is_send {
    static constexpr bool value = []() {
        if constexpr (is_explicitly_not_send<T>::value) {
            return false;
        }
        else if constexpr (has_send_marker<T>::value) {
            return T::is_send;
        }
        else {
            return std::is_move_constructible_v<T>;
        }
    }();
};

class ThreadSafeQueue {
public:
    static constexpr bool is_send = true;
    ThreadSafeQueue() = default;
    ThreadSafeQueue(ThreadSafeQueue&&) = default;
};

} // namespace rusty

int main() {
    std::cout << "has_send_marker<ThreadSafeQueue>: "
              << rusty::has_send_marker<rusty::ThreadSafeQueue>::value << "\n";

    std::cout << "ThreadSafeQueue::is_send: "
              << rusty::ThreadSafeQueue::is_send << "\n";

    std::cout << "is_send<ThreadSafeQueue>: "
              << rusty::is_send<rusty::ThreadSafeQueue>::value << "\n";

    return 0;
}
