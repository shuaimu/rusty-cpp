#ifndef RUSTY_ERROR_HPP
#define RUSTY_ERROR_HPP

#include <concepts>
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

} // namespace error
} // namespace rusty

#endif // RUSTY_ERROR_HPP
