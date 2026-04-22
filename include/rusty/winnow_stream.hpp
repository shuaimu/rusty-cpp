#ifndef RUSTY_WINNOW_STREAM_HPP
#define RUSTY_WINNOW_STREAM_HPP

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <span>
#include <string_view>
#include <tuple>
#include <type_traits>
#include <utility>

#include "rusty/array.hpp"
#include "rusty/option.hpp"
#include "rusty/string.hpp"

namespace winnow::stream {

using Location = std::size_t;

namespace detail {

template<typename T>
struct is_tuple : std::false_type {};

template<typename... Ts>
struct is_tuple<std::tuple<Ts...>> : std::true_type {};

template<typename T>
inline constexpr bool is_tuple_v = is_tuple<std::remove_cvref_t<T>>::value;

inline bool match_at(std::span<const uint8_t> bytes, std::size_t pos, uint8_t value, std::size_t& len_out) {
    if (pos >= bytes.size() || bytes[pos] != value) {
        return false;
    }
    len_out = 1;
    return true;
}

inline bool match_at(
    std::span<const uint8_t> bytes,
    std::size_t pos,
    std::string_view value,
    std::size_t& len_out
) {
    if (value.size() == 0 || pos + value.size() > bytes.size()) {
        return false;
    }
    const auto ptr = reinterpret_cast<const uint8_t*>(value.data());
    if (!std::equal(ptr, ptr + value.size(), bytes.begin() + static_cast<std::ptrdiff_t>(pos))) {
        return false;
    }
    len_out = value.size();
    return true;
}

template<std::size_t Extent>
inline bool match_at(
    std::span<const uint8_t> bytes,
    std::size_t pos,
    std::span<const uint8_t, Extent> value,
    std::size_t& len_out
) {
    if (value.size() == 0 || pos + value.size() > bytes.size()) {
        return false;
    }
    if (!std::equal(value.begin(), value.end(), bytes.begin() + static_cast<std::ptrdiff_t>(pos))) {
        return false;
    }
    len_out = value.size();
    return true;
}

template<typename Pattern>
inline bool match_any(std::span<const uint8_t> bytes, std::size_t pos, const Pattern& pattern, std::size_t& len_out) {
    if constexpr (is_tuple_v<Pattern>) {
        bool matched = false;
        std::size_t matched_len = 0;
        std::apply(
            [&](const auto&... part) {
                (
                    [&] {
                        if (matched) {
                            return;
                        }
                        std::size_t part_len = 0;
                        if (match_at(bytes, pos, part, part_len)) {
                            matched = true;
                            matched_len = part_len;
                        }
                    }(),
                    ...
                );
            },
            pattern
        );
        if (matched) {
            len_out = matched_len;
        }
        return matched;
    } else {
        return match_at(bytes, pos, pattern, len_out);
    }
}

template<typename Pattern>
inline rusty::Option<rusty::range<std::size_t>> find_slice_impl(
    std::span<const uint8_t> bytes,
    const Pattern& pattern
) {
    for (std::size_t i = 0; i < bytes.size(); ++i) {
        std::size_t len = 0;
        if (match_any(bytes, i, pattern, len)) {
            return rusty::Option<rusty::range<std::size_t>>(rusty::range<std::size_t>(i, i + len));
        }
    }
    return rusty::Option<rusty::range<std::size_t>>(rusty::None);
}

} // namespace detail

class ByteView {
public:
    explicit ByteView(std::span<const uint8_t> bytes) : bytes_(bytes) {}

    std::size_t size() const {
        return bytes_.size();
    }

    bool empty() const {
        return bytes_.empty();
    }

    std::span<const uint8_t> as_span() const {
        return bytes_;
    }

    rusty::Option<const uint8_t&> first() const {
        if (bytes_.empty()) {
            return rusty::Option<const uint8_t&>(rusty::None);
        }
        return rusty::Option<const uint8_t&>(bytes_.front());
    }

    rusty::Option<uint8_t> peek_token() const {
        if (bytes_.empty()) {
            return rusty::Option<uint8_t>(rusty::None);
        }
        return rusty::Option<uint8_t>(bytes_.front());
    }

    rusty::Option<const uint8_t&> get(std::size_t index) const {
        if (index >= bytes_.size()) {
            return rusty::Option<const uint8_t&>(rusty::None);
        }
        return rusty::Option<const uint8_t&>(bytes_[index]);
    }

    template<typename Pred>
    rusty::Option<std::size_t> offset_for(Pred&& pred) const {
        for (std::size_t i = 0; i < bytes_.size(); ++i) {
            if (std::forward<Pred>(pred)(bytes_[i])) {
                return rusty::Option<std::size_t>(i);
            }
        }
        return rusty::Option<std::size_t>(rusty::None);
    }

    template<typename Pattern>
    rusty::Option<rusty::range<std::size_t>> find_slice(Pattern&& pattern) const {
        return detail::find_slice_impl(bytes_, std::forward<Pattern>(pattern));
    }

    const uint8_t& operator[](std::size_t idx) const {
        return bytes_[idx];
    }

private:
    std::span<const uint8_t> bytes_;
};

template<typename I>
struct LocatingSlice;

template<>
struct LocatingSlice<std::string_view> {
    struct Checkpoint {
        std::size_t offset{};

        static Checkpoint new_(std::size_t value) {
            return Checkpoint{value};
        }

        std::size_t offset_from(const Checkpoint& start) const {
            return offset - start.offset;
        }
    };

    std::string_view initial;
    std::string_view input;

    static LocatingSlice<std::string_view> new_(std::string_view value) {
        return LocatingSlice<std::string_view>{value, value};
    }

    bool empty() const {
        return input.empty();
    }

    bool is_empty() const {
        return input.empty();
    }

    std::size_t size() const {
        return input.size();
    }

    std::size_t eof_offset() const {
        return input.size();
    }

    bool starts_with(std::string_view prefix) const {
        return input.starts_with(prefix);
    }

    Checkpoint checkpoint() const {
        return Checkpoint::new_(initial.size() - input.size());
    }

    void reset(const Checkpoint& checkpoint) {
        const auto offset = std::min(checkpoint.offset, initial.size());
        input = initial.substr(offset);
    }

    void reset_to_start() {
        input = initial;
    }

    void finish() {
        input = std::string_view{};
    }

    std::size_t current_token_start() const {
        return initial.size() - input.size();
    }

    std::size_t previous_token_end() const {
        const auto start = current_token_start();
        return start == 0 ? 0 : (start - 1);
    }

    rusty::Option<uint8_t> next_token() {
        if (input.empty()) {
            return rusty::Option<uint8_t>(rusty::None);
        }
        const auto value = static_cast<uint8_t>(input.front());
        input.remove_prefix(1);
        return rusty::Option<uint8_t>(value);
    }

    std::string_view next_slice(std::size_t offset) {
        const auto take = std::min(offset, input.size());
        const auto out = input.substr(0, take);
        input.remove_prefix(take);
        return out;
    }

    ByteView as_bstr() const {
        return ByteView(rusty::as_bytes(input));
    }

    template<typename Pattern>
    rusty::Option<rusty::range<std::size_t>> find_slice(Pattern&& pattern) const {
        return as_bstr().find_slice(std::forward<Pattern>(pattern));
    }

    std::size_t offset_from(const std::string_view& start) const {
        return start.size() - input.size();
    }

    std::size_t offset_from(const std::string_view* start) const {
        return start ? offset_from(*start) : 0;
    }
};

template<typename T>
struct TokenSlice {
    struct Checkpoint {
        std::size_t offset{};

        static Checkpoint new_(std::size_t value) {
            return Checkpoint{value};
        }

        std::size_t offset_from(const Checkpoint& start) const {
            return offset - start.offset;
        }
    };

    struct ReverseView {
        std::span<const T> span;

        auto begin() const {
            return std::make_reverse_iterator(span.end());
        }

        auto end() const {
            return std::make_reverse_iterator(span.begin());
        }

        template<typename Pred>
        rusty::Option<const T&> find(Pred&& pred) const {
            for (auto it = begin(); it != end(); ++it) {
                if (std::forward<Pred>(pred)(*it)) {
                    return rusty::Option<const T&>(*it);
                }
            }
            return rusty::Option<const T&>(rusty::None);
        }
    };

    std::span<const T> initial;
    std::span<const T> input;

    static TokenSlice<T> new_(std::span<const T> value) {
        return TokenSlice<T>{value, value};
    }

    bool empty() const {
        return input.empty();
    }

    bool is_empty() const {
        return input.empty();
    }

    std::size_t size() const {
        return input.size();
    }

    std::size_t eof_offset() const {
        return input.size();
    }

    Checkpoint checkpoint() const {
        return Checkpoint::new_(initial.size() - input.size());
    }

    void reset(const Checkpoint& checkpoint) {
        const auto offset = std::min(checkpoint.offset, initial.size());
        input = initial.subspan(offset);
    }

    void reset_to_start() {
        input = initial;
    }

    void finish() {
        input = initial.subspan(initial.size());
    }

    std::size_t offset_from(const TokenSlice<T>& other) const {
        return checkpoint().offset_from(other.checkpoint());
    }

    std::size_t offset_from(const Checkpoint& other) const {
        return checkpoint().offset_from(other);
    }

    ReverseView previous_tokens() const {
        const auto consumed = initial.size() - input.size();
        return ReverseView{initial.subspan(0, consumed)};
    }

    rusty::Option<const T&> first() const {
        if (input.empty()) {
            return rusty::Option<const T&>(rusty::None);
        }
        return rusty::Option<const T&>(input.front());
    }

    rusty::Option<const T&> peek_token() const {
        return first();
    }

    rusty::Option<const T&> next_token() {
        if (input.empty()) {
            return rusty::Option<const T&>(rusty::None);
        }
        const T& value = input.front();
        input = input.subspan(1);
        return rusty::Option<const T&>(value);
    }

    std::span<const T> next_slice(std::size_t offset) {
        const auto take = std::min(offset, input.size());
        const auto out = input.subspan(0, take);
        input = input.subspan(take);
        return out;
    }

    rusty::Option<const T&> get(std::size_t index) const {
        if (index >= input.size()) {
            return rusty::Option<const T&>(rusty::None);
        }
        return rusty::Option<const T&>(input[index]);
    }
};

} // namespace winnow::stream

namespace rusty {

namespace detail {
template<typename T>
inline constexpr bool always_false_v = false;
} // namespace detail

inline std::span<const uint8_t> as_bytes(const ::winnow::stream::LocatingSlice<std::string_view>& stream) {
    return stream.as_bstr().as_span();
}

template<typename T>
auto as_bytes(const T& value) {
    if constexpr (requires { value.as_bytes(); }) {
        return value.as_bytes();
    } else if constexpr (std::is_convertible_v<T, std::string_view>) {
        return rusty::as_bytes(std::string_view(value));
    } else {
        static_assert(detail::always_false_v<T>, "rusty::as_bytes: unsupported type");
    }
}

template<typename Haystack, typename Needle>
bool contains(const Haystack& haystack, const Needle& needle) {
    if constexpr (requires { haystack.contains(needle); }) {
        return haystack.contains(needle);
    } else if constexpr (
        std::is_convertible_v<Haystack, std::string_view>
        && std::is_convertible_v<Needle, std::string_view>
    ) {
        const auto h = std::string_view(haystack);
        const auto n = std::string_view(needle);
        return h.find(n) != std::string_view::npos;
    } else if constexpr (requires { std::begin(haystack); std::end(haystack); }) {
        for (const auto& item : haystack) {
            if constexpr (requires { item == needle; }) {
                if (item == needle) {
                    return true;
                }
            } else if constexpr (requires { needle == item; }) {
                if (needle == item) {
                    return true;
                }
            } else if constexpr (std::is_pointer_v<std::remove_reference_t<Needle>>) {
                // Rust range/collection `contains(&x)` commonly lowers to an address-of
                // temporary in generated C++; compare against the pointed value.
                if (needle == nullptr) {
                    continue;
                }
                if constexpr (requires { item == *needle; }) {
                    if (item == *needle) {
                        return true;
                    }
                } else if constexpr (requires { *needle == item; }) {
                    if (*needle == item) {
                        return true;
                    }
                }
            } else if constexpr (requires { *needle; }) {
                if constexpr (requires { item == *needle; }) {
                    if (item == *needle) {
                        return true;
                    }
                } else if constexpr (requires { *needle == item; }) {
                    if (*needle == item) {
                        return true;
                    }
                }
            }
        }
        return false;
    } else {
        return false;
    }
}

inline bool starts_with(std::string_view value, std::string_view prefix) {
    return value.size() >= prefix.size() && value.substr(0, prefix.size()) == prefix;
}

template<std::size_t N>
bool starts_with(std::string_view value, const std::array<char32_t, N>& any_prefix) {
    if (value.empty()) {
        return false;
    }
    const auto front = static_cast<unsigned char>(value.front());
    for (const auto ch : any_prefix) {
        if (front == static_cast<unsigned char>(ch)) {
            return true;
        }
    }
    return false;
}

template<typename Value, typename Prefix>
bool starts_with(const Value& value, const Prefix& prefix) {
    if constexpr (requires { value.starts_with(prefix); }) {
        return value.starts_with(prefix);
    } else if constexpr (std::is_convertible_v<Value, std::string_view>) {
        return starts_with(std::string_view(value), prefix);
    } else {
        return false;
    }
}

inline Option<uint8_t> next_token(std::string_view& value) {
    if (value.empty()) {
        return Option<uint8_t>(rusty::None);
    }
    const auto token = static_cast<uint8_t>(value.front());
    value.remove_prefix(1);
    return Option<uint8_t>(token);
}

template<typename Stream>
auto next_token(Stream& stream) {
    if constexpr (requires { stream.next_token(); }) {
        return stream.next_token();
    } else if constexpr (std::is_same_v<std::remove_cvref_t<Stream>, std::string_view>) {
        return next_token(stream);
    } else {
        static_assert(detail::always_false_v<Stream>, "rusty::next_token: unsupported stream type");
    }
}

inline Option<uint8_t> peek_token(const std::string_view& value) {
    if (value.empty()) {
        return Option<uint8_t>(rusty::None);
    }
    return Option<uint8_t>(static_cast<uint8_t>(value.front()));
}

template<typename Stream>
auto peek_token(const Stream& stream) {
    if constexpr (requires { stream.peek_token(); }) {
        return stream.peek_token();
    } else if constexpr (std::is_same_v<std::remove_cvref_t<Stream>, std::string_view>) {
        return peek_token(stream);
    } else {
        static_assert(detail::always_false_v<Stream>, "rusty::peek_token: unsupported stream type");
    }
}

inline std::size_t offset_from(std::string_view current, const std::string_view& start) {
    return start.size() >= current.size() ? (start.size() - current.size()) : 0;
}

inline std::size_t offset_from(std::string_view current, const std::string_view* start) {
    return start == nullptr ? 0 : offset_from(current, *start);
}

template<typename T, std::size_t ExtentA, std::size_t ExtentB>
std::size_t offset_from(std::span<const T, ExtentA> current, const std::span<const T, ExtentB>& start) {
    return start.size() >= current.size() ? (start.size() - current.size()) : 0;
}

template<typename T, std::size_t ExtentA, std::size_t ExtentB>
std::size_t offset_from(std::span<const T, ExtentA> current, const std::span<const T, ExtentB>* start) {
    return start == nullptr ? 0 : offset_from(current, *start);
}

template<typename Current, typename Start>
auto offset_from(const Current& current, const Start& start) {
    if constexpr (requires { current.offset_from(start); }) {
        return current.offset_from(start);
    } else if constexpr (
        std::is_pointer_v<std::remove_cvref_t<Start>>
        && requires { *start; current.offset_from(*start); }
    ) {
        return start == nullptr ? 0 : current.offset_from(*start);
    } else if constexpr (
        std::is_convertible_v<Current, std::string_view>
        && std::is_same_v<std::remove_cvref_t<Start>, std::string_view>
    ) {
        return offset_from(std::string_view(current), static_cast<const std::string_view&>(start));
    } else if constexpr (
        std::is_convertible_v<Current, std::string_view>
        && std::is_same_v<std::remove_cvref_t<Start>, std::string_view*>
    ) {
        return offset_from(std::string_view(current), static_cast<const std::string_view*>(start));
    } else if constexpr (std::is_convertible_v<Current, std::string_view>) {
        return offset_from(std::string_view(current), static_cast<const std::string_view&>(start));
    } else {
        static_assert(
            detail::always_false_v<Current>,
            "rusty::offset_from: unsupported stream type"
        );
    }
}

template<typename Pred>
Option<std::size_t> offset_for(std::span<const uint8_t> bytes, Pred&& pred) {
    for (std::size_t i = 0; i < bytes.size(); ++i) {
        if (std::forward<Pred>(pred)(bytes[i])) {
            return Option<std::size_t>(i);
        }
    }
    return Option<std::size_t>(rusty::None);
}

template<typename Stream, typename Pred>
auto offset_for(const Stream& stream, Pred&& pred) {
    if constexpr (requires { stream.offset_for(std::forward<Pred>(pred)); }) {
        return stream.offset_for(std::forward<Pred>(pred));
    } else if constexpr (std::is_convertible_v<Stream, std::string_view>) {
        return offset_for(as_bytes(std::string_view(stream)), std::forward<Pred>(pred));
    } else if constexpr (std::is_convertible_v<Stream, std::span<const uint8_t>>) {
        return offset_for(std::span<const uint8_t>(stream), std::forward<Pred>(pred));
    } else {
        static_assert(
            detail::always_false_v<Stream>,
            "rusty::offset_for: unsupported stream type"
        );
    }
}

template<typename Stream, typename Pattern>
auto find_slice(const Stream& stream, Pattern&& pattern) {
    if constexpr (requires { stream.find_slice(std::forward<Pattern>(pattern)); }) {
        return stream.find_slice(std::forward<Pattern>(pattern));
    } else if constexpr (std::is_convertible_v<Stream, std::string_view>) {
        return ::winnow::stream::detail::find_slice_impl(
            as_bytes(std::string_view(stream)),
            std::forward<Pattern>(pattern)
        );
    } else if constexpr (std::is_convertible_v<Stream, std::span<const uint8_t>>) {
        return ::winnow::stream::detail::find_slice_impl(
            std::span<const uint8_t>(stream),
            std::forward<Pattern>(pattern)
        );
    } else {
        static_assert(detail::always_false_v<Stream>, "rusty::find_slice: unsupported stream type");
    }
}

template<typename Pattern, typename Token>
bool contains_token(const Pattern& pattern, Token&& token) {
    if constexpr (requires { pattern.contains_token(std::forward<Token>(token)); }) {
        return pattern.contains_token(std::forward<Token>(token));
    } else if constexpr (requires { pattern.contains(std::forward<Token>(token)); }) {
        return pattern.contains(std::forward<Token>(token));
    } else if constexpr (::winnow::stream::detail::is_tuple_v<Pattern>) {
        bool matched = false;
        std::apply(
            [&](const auto&... part) {
                (
                    [&] {
                        if (!matched && contains_token(part, token)) {
                            matched = true;
                        }
                    }(),
                    ...
                );
            },
            pattern
        );
        return matched;
    } else if constexpr (requires { pattern.begin(); pattern.end(); }) {
        for (const auto& item : pattern) {
            if (contains_token(item, token)) {
                return true;
            }
        }
        return false;
    } else if constexpr (requires { pattern == token; }) {
        return pattern == token;
    } else if constexpr (requires { token == pattern; }) {
        return token == pattern;
    } else {
        return false;
    }
}

inline std::string_view checkpoint(const std::string_view& stream) {
    return stream;
}

template<typename Stream>
auto checkpoint(const Stream& stream) {
    if constexpr (requires { stream.checkpoint(); }) {
        return stream.checkpoint();
    } else if constexpr (std::is_convertible_v<Stream, std::string_view>) {
        return checkpoint(std::string_view(stream));
    } else {
        static_assert(detail::always_false_v<Stream>, "rusty::checkpoint: unsupported stream type");
    }
}

inline void reset(std::string_view& stream, std::string_view cp) {
    stream = cp;
}

template<typename Stream, typename Checkpoint>
void reset(Stream& stream, const Checkpoint& cp) {
    if constexpr (requires { stream.reset(cp); }) {
        stream.reset(cp);
    } else if constexpr (
        std::is_same_v<std::remove_cvref_t<Stream>, std::string_view>
        && std::is_convertible_v<Checkpoint, std::string_view>
    ) {
        reset(stream, std::string_view(cp));
    } else {
        static_assert(detail::always_false_v<Stream>, "rusty::reset: unsupported stream type");
    }
}

inline std::string_view next_slice(std::string_view& stream, std::size_t offset) {
    const auto take = std::min(offset, stream.size());
    const auto out = stream.substr(0, take);
    stream.remove_prefix(take);
    return out;
}

template<typename Stream>
auto next_slice(Stream& stream, std::size_t offset) {
    if constexpr (requires { stream.next_slice(offset); }) {
        return stream.next_slice(offset);
    } else if constexpr (std::is_same_v<std::remove_cvref_t<Stream>, std::string_view>) {
        return next_slice(stream, offset);
    } else {
        static_assert(detail::always_false_v<Stream>, "rusty::next_slice: unsupported stream type");
    }
}

template<typename Stream>
std::size_t eof_offset(const Stream& stream) {
    if constexpr (requires { stream.eof_offset(); }) {
        return stream.eof_offset();
    } else if constexpr (requires { stream.size(); }) {
        return stream.size();
    } else {
        return 0;
    }
}

} // namespace rusty

#endif // RUSTY_WINNOW_STREAM_HPP
