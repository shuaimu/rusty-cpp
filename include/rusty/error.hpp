#ifndef RUSTY_ERROR_HPP
#define RUSTY_ERROR_HPP

#include <concepts>
#include <cstddef>
#include <format>
#include <string>
#include <string_view>
#include <type_traits>
#include <utility>

namespace rusty {
namespace error {

template<typename T>
using DescriptionReturnT = decltype(std::declval<const T&>().description());

template<typename T>
concept HasSafeDescriptionView =
    requires(const T& v) {
        { v.description() } -> std::convertible_to<std::string_view>;
    } &&
    !std::same_as<std::remove_cvref_t<DescriptionReturnT<T>>, std::string>;

template<typename T>
std::string_view description(const T& value)
requires HasSafeDescriptionView<T>
{
    return static_cast<std::string_view>(value.description());
}

template<typename T>
std::string_view description(const T&)
requires(!HasSafeDescriptionView<T>)
{
    return std::string_view{};
}

template<typename Error, typename Len, typename Expected>
auto invalid_length(Len&& len, Expected&& expected) {
    if constexpr (requires {
                      Error::invalid_length(
                          static_cast<size_t>(std::forward<Len>(len)),
                          std::forward<Expected>(expected));
                  }) {
        return Error::invalid_length(
            static_cast<size_t>(std::forward<Len>(len)),
            std::forward<Expected>(expected));
    } else {
        return Error::custom(std::format(
            "invalid length {}",
            static_cast<size_t>(std::forward<Len>(len))));
    }
}

} // namespace error
} // namespace rusty

#endif // RUSTY_ERROR_HPP
