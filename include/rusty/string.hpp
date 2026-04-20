#ifndef RUSTY_STRING_HPP
#define RUSTY_STRING_HPP

#include <cstring>
#include <cstdlib>
#include <algorithm>
#include <iterator>
#include <ostream>
#include <cstdint>
#include <string>
#include <string_view>
#include <tuple>
#include <vector>
#include <cctype>
#include <span>
#include <limits>
#include <stdexcept>
#include <utility>
#include "rusty/box.hpp"
#include "rusty/fmt.hpp"

// @safe
namespace rusty {

// Forward declaration for str type (borrowed string slice)
class str;

// @safe
// Rust-like owned String type
// Manages a heap-allocated, growable UTF-8 string
class String {
private:
    static bool is_valid_utf8_bytes(const unsigned char* data, size_t len) {
        size_t i = 0;
        while (i < len) {
            const auto byte = data[i];
            if (byte <= 0x7F) {
                ++i;
                continue;
            }
            if ((byte >> 5) == 0x6) {
                if (i + 1 >= len) return false;
                const auto b1 = data[i + 1];
                if ((b1 & 0xC0) != 0x80 || byte < 0xC2) return false;
                i += 2;
                continue;
            }
            if ((byte >> 4) == 0xE) {
                if (i + 2 >= len) return false;
                const auto b1 = data[i + 1];
                const auto b2 = data[i + 2];
                if ((b1 & 0xC0) != 0x80 || (b2 & 0xC0) != 0x80) return false;
                if (byte == 0xE0 && b1 < 0xA0) return false;
                if (byte == 0xED && b1 >= 0xA0) return false;
                i += 3;
                continue;
            }
            if ((byte >> 3) == 0x1E) {
                if (i + 3 >= len) return false;
                const auto b1 = data[i + 1];
                const auto b2 = data[i + 2];
                const auto b3 = data[i + 3];
                if ((b1 & 0xC0) != 0x80 || (b2 & 0xC0) != 0x80 || (b3 & 0xC0) != 0x80) return false;
                if (byte == 0xF0 && b1 < 0x90) return false;
                if (byte == 0xF4 && b1 >= 0x90) return false;
                if (byte > 0xF4) return false;
                i += 4;
                continue;
            }
            return false;
        }
        return true;
    }

    char* data_;
    size_t len_;      // Current length (excluding null terminator)
    size_t capacity_; // Allocated capacity (including space for null terminator)

    // Private helper to ensure null termination
    void ensure_null_terminated() {
        if (data_ && len_ < capacity_) {
            data_[len_] = '\0';
        }
    }

    // Grow capacity to at least new_cap
    void grow(size_t new_cap) {
        if (new_cap <= capacity_) return;
        
        // Round up to next power of 2 for better performance
        size_t actual_cap = 16;
        while (actual_cap < new_cap) {
            actual_cap *= 2;
        }
        
        char* new_data = static_cast<char*>(std::malloc(actual_cap));
        if (!new_data) {
            throw std::bad_alloc();
        }
        
        if (data_) {
            std::memcpy(new_data, data_, len_);
            std::free(data_);
        }
        
        data_ = new_data;
        capacity_ = actual_cap;
        ensure_null_terminated();
    }

public:
    // Constructors
    String() : data_(nullptr), len_(0), capacity_(0) {}

    // Converting constructors for parity with transpiled call sites that
    // materialize std::string/std::string_view intermediates.
    String(const char* cstr) : String(String::from(cstr)) {}
    String(const std::string& str) : String(String::from(str)) {}
    String(std::string_view sv) : String(String::from(sv)) {}
    
    // @lifetime: owned
    static String new_() {
        return String();
    }

    // Alias for backward compatibility
    static auto make() {
        return String();
    }
    
    // @lifetime: owned
    static String with_capacity(size_t cap) {
        String s;
        s.grow(cap + 1); // +1 for null terminator
        return s;
    }
    
    // @lifetime: owned
    static String from(const char* cstr) {
        if (!cstr) return String();
        
        size_t len = std::strlen(cstr);
        String s;
        s.grow(len + 1);
        std::memcpy(s.data_, cstr, len);
        s.len_ = len;
        s.ensure_null_terminated();
        return s;
    }
    
    // @lifetime: owned
    static String from(const std::string& str) {
        return String::from(str.c_str());
    }
    
    // @lifetime: owned
    static String from(std::string_view sv) {
        String s;
        s.grow(sv.length() + 1);
        std::memcpy(s.data_, sv.data(), sv.length());
        s.len_ = sv.length();
        s.ensure_null_terminated();
        return s;
    }

    // Rust-like lossy UTF-8 decode helper used by expanded crates.
    // Current runtime keeps a byte-preserving fallback to maintain compile parity.
    static String from_utf8_lossy(std::span<const uint8_t> bytes) {
        String s;
        if (bytes.empty()) {
            return s;
        }
        s.grow(bytes.size() + 1);
        for (size_t i = 0; i < bytes.size(); i++) {
            s.data_[i] = static_cast<char>(bytes[i]);
        }
        s.len_ = bytes.size();
        s.ensure_null_terminated();
        return s;
    }

    template<typename Bytes>
    static rusty::Result<String, String> from_utf8(Bytes&& bytes) {
        if constexpr (requires { bytes.data(); bytes.size(); }) {
            const auto* raw = bytes.data();
            const size_t len = static_cast<size_t>(bytes.size());
            const auto* data = reinterpret_cast<const unsigned char*>(raw);
            if (!is_valid_utf8_bytes(data, len)) {
                return rusty::Result<String, String>::Err(String::from("invalid utf-8"));
            }
            String s;
            if (len > 0) {
                s.grow(len + 1);
                for (size_t i = 0; i < len; i++) {
                    s.data_[i] = static_cast<char>(data[i]);
                }
                s.len_ = len;
                s.ensure_null_terminated();
            }
            return rusty::Result<String, String>::Ok(std::move(s));
        }
        return rusty::Result<String, String>::Err(String::from("unsupported from_utf8 input"));
    }
    
    // Move constructor (String is move-only)
    String(String&& other) noexcept 
        : data_(other.data_), len_(other.len_), capacity_(other.capacity_) {
        other.data_ = nullptr;
        other.len_ = 0;
        other.capacity_ = 0;
    }
    
    // Move assignment
    String& operator=(String&& other) noexcept {
        if (this != &other) {
            if (data_) {
                std::free(data_);
            }
            data_ = other.data_;
            len_ = other.len_;
            capacity_ = other.capacity_;
            
            other.data_ = nullptr;
            other.len_ = 0;
            other.capacity_ = 0;
        }
        return *this;
    }
    
    // Delete copy constructor and copy assignment
    String(const String&) = delete;
    String& operator=(const String&) = delete;
    
    // Destructor
    ~String() {
        if (data_) {
            std::free(data_);
        }
    }
    
    // Clone method for explicit copying
    // @lifetime: owned
    String clone() const {
        if (!data_) return String();
        
        String s;
        s.grow(capacity_);
        std::memcpy(s.data_, data_, len_);
        s.len_ = len_;
        s.ensure_null_terminated();
        return s;
    }
    
    // Capacity and length
    size_t len() const { return len_; }
    size_t capacity() const { return capacity_ > 0 ? capacity_ - 1 : 0; } // Exclude null terminator
    bool is_empty() const { return len_ == 0; }
    
    // Reserve capacity
    void reserve(size_t additional) {
        grow(len_ + additional + 1);
    }
    
    // Clear the string
    void clear() {
        len_ = 0;
        ensure_null_terminated();
    }
    
    // Push a single character
    void push(char ch) {
        if (len_ + 1 >= capacity_) {
            grow(len_ + 2); // +1 for char, +1 for null
        }
        data_[len_++] = ch;
        ensure_null_terminated();
    }
    
    // Push a string slice
    void push_str(const char* str) {
        if (!str) return;
        
        size_t str_len = std::strlen(str);
        if (len_ + str_len >= capacity_) {
            grow(len_ + str_len + 1);
        }
        std::memcpy(data_ + len_, str, str_len);
        len_ += str_len;
        ensure_null_terminated();
    }
    
    void push_str(const String& other) {
        if (!other.data_) return;
        
        if (len_ + other.len_ >= capacity_) {
            grow(len_ + other.len_ + 1);
        }
        std::memcpy(data_ + len_, other.data_, other.len_);
        len_ += other.len_;
        ensure_null_terminated();
    }
    
    // fmt::Write trait — write string/char into this String.
    fmt::Result write_str(std::string_view s) {
        push_str(s.data());
        return fmt::Result::Ok({});
    }
    fmt::Result write_char(char32_t ch) {
        push(ch);
        return fmt::Result::Ok({});
    }

    // Pop character from end
    char pop() {
        if (len_ == 0) {
            throw std::out_of_range("pop from empty String");
        }
        char ch = data_[--len_];
        ensure_null_terminated();
        return ch;
    }
    
    // Truncate to new length
    void truncate(size_t new_len) {
        if (new_len < len_) {
            len_ = new_len;
            ensure_null_terminated();
        }
    }
    
    // Insert string at position
    void insert(size_t idx, const char* str) {
        if (!str) return;
        if (idx > len_) {
            throw std::out_of_range("insert index out of bounds");
        }
        
        size_t str_len = std::strlen(str);
        if (len_ + str_len >= capacity_) {
            grow(len_ + str_len + 1);
        }
        
        // Move existing data
        if (idx < len_) {
            std::memmove(data_ + idx + str_len, data_ + idx, len_ - idx);
        }
        
        // Insert new data
        std::memcpy(data_ + idx, str, str_len);
        len_ += str_len;
        ensure_null_terminated();
    }
    
    // Remove range [start, end)
    void drain(size_t start, size_t end) {
        if (start > end || end > len_) {
            throw std::out_of_range("drain range out of bounds");
        }
        
        size_t remove_len = end - start;
        if (remove_len == 0) return;
        
        // Move data after the range
        if (end < len_) {
            std::memmove(data_ + start, data_ + end, len_ - end);
        }
        
        len_ -= remove_len;
        ensure_null_terminated();
    }
    
    // Get C string (null-terminated)
    // @lifetime: (&'a) -> &'a
    const char* as_ptr() const {
        return data_ ? data_ : "";
    }
    
    // @lifetime: (&'a) -> &'a
    const char* c_str() const {
        return as_ptr();
    }
    
    // Get as string_view
    // @lifetime: (&'a) -> &'a
    std::string_view as_str() const {
        return data_ ? std::string_view(data_, len_) : std::string_view();
    }

    // Implicit conversion to string_view — enables passing String where
    // std::string_view is expected (mirrors Rust's Deref<Target=str>).
    operator std::string_view() const {
        return as_str();
    }

    // Convert to std::string (copies data)
    // @lifetime: owned
    std::string to_string() const {
        return data_ ? std::string(data_, len_) : std::string();
    }

    std::vector<uint8_t> into_bytes() && {
        std::vector<uint8_t> out;
        out.reserve(len_);
        for (size_t i = 0; i < len_; i++) {
            out.push_back(static_cast<uint8_t>(data_[i]));
        }
        return out;
    }

    std::vector<uint8_t> into_bytes() const& {
        std::vector<uint8_t> out;
        out.reserve(len_);
        for (size_t i = 0; i < len_; i++) {
            out.push_back(static_cast<uint8_t>(data_[i]));
        }
        return out;
    }

    // Rust `String::into_boxed_str()`.
    // Keep owned storage by boxing the String itself.
    Box<String> into_boxed_str() && {
        return Box<String>::new_(std::move(*this));
    }

    Box<String> into_boxed_str() const& {
        return Box<String>::new_(this->clone());
    }
    
    // Character access
    // @lifetime: (&'a) -> &'a
    const char& operator[](size_t idx) const {
        if (idx >= len_) {
            throw std::out_of_range("index out of bounds");
        }
        return data_[idx];
    }
    
    // @lifetime: (&'a mut) -> &'a mut
    char& operator[](size_t idx) {
        if (idx >= len_) {
            throw std::out_of_range("index out of bounds");
        }
        return data_[idx];
    }
    
    // Get slice of string
    // @lifetime: (&'a) -> &'a
    std::string_view slice(size_t start, size_t end) const {
        if (start > end || end > len_) {
            throw std::out_of_range("slice range out of bounds");
        }
        return std::string_view(data_ + start, end - start);
    }
    
    // Iterators
    // @lifetime: (&'a) -> &'a
    const char* begin() const { return data_ ? data_ : ""; }
    // @lifetime: (&'a) -> &'a
    const char* end() const { return data_ ? data_ + len_ : ""; }
    
    // @lifetime: (&'a mut) -> &'a mut
    char* begin() { return data_; }
    // @lifetime: (&'a mut) -> &'a mut  
    char* end() { return data_ ? data_ + len_ : nullptr; }
    
    // Comparison operators
    bool operator==(const String& other) const {
        if (len_ != other.len_) return false;
        if (!data_ && !other.data_) return true;
        if (!data_ || !other.data_) return false;
        return std::memcmp(data_, other.data_, len_) == 0;
    }
    
    bool operator==(const char* cstr) const {
        if (!cstr) return !data_ || len_ == 0;
        if (!data_) return std::strlen(cstr) == 0;  // Empty string equals empty cstring

        size_t cstr_len = std::strlen(cstr);
        if (len_ != cstr_len) return false;
        if (len_ == 0) return true;  // Both empty
        return std::memcmp(data_, cstr, len_) == 0;
    }
    bool operator==(const std::string& other) const {
        return as_str() == std::string_view(other);
    }
    
    bool operator!=(const String& other) const { return !(*this == other); }
    bool operator!=(const char* cstr) const { return !(*this == cstr); }
    
    bool operator<(const String& other) const {
        if (!data_ && !other.data_) return false;
        if (!data_) return true;
        if (!other.data_) return false;
        
        int cmp = std::memcmp(data_, other.data_, std::min(len_, other.len_));
        if (cmp != 0) return cmp < 0;
        return len_ < other.len_;
    }
    
    // String concatenation
    // @lifetime: owned
    String operator+(const String& other) const {
        String result;
        result.grow(len_ + other.len_ + 1);
        if (data_) {
            std::memcpy(result.data_, data_, len_);
        }
        if (other.data_) {
            std::memcpy(result.data_ + len_, other.data_, other.len_);
        }
        result.len_ = len_ + other.len_;
        result.ensure_null_terminated();
        return result;
    }
    
    // Append operator
    String& operator+=(const String& other) {
        push_str(other);
        return *this;
    }
    
    String& operator+=(const char* cstr) {
        push_str(cstr);
        return *this;
    }
    
    String& operator+=(char ch) {
        push(ch);
        return *this;
    }

    // Repeat the string count times (Rust String/str repeat semantics).
    // @lifetime: owned
    String repeat(size_t count) const {
        if (count == 0 || len_ == 0 || !data_) {
            return String();
        }

        // Guard size computation before allocation/copy.
        if (count > (std::numeric_limits<size_t>::max() - 1) / len_) {
            throw std::length_error("String::repeat overflow");
        }

        const size_t total_len = len_ * count;
        String result;
        result.grow(total_len + 1);

        char* dst = result.data_;
        for (size_t i = 0; i < count; i++) {
            std::memcpy(dst, data_, len_);
            dst += len_;
        }

        result.len_ = total_len;
        result.ensure_null_terminated();
        return result;
    }
    
    // Check if string contains substring
    bool contains(const char* needle) const {
        if (!needle || !data_) return false;
        return std::strstr(data_, needle) != nullptr;
    }
    
    bool starts_with(const char* prefix) const {
        if (!prefix || !data_) return !prefix;
        size_t prefix_len = std::strlen(prefix);
        if (prefix_len > len_) return false;
        return std::memcmp(data_, prefix, prefix_len) == 0;
    }
    
    bool ends_with(const char* suffix) const {
        if (!suffix || !data_) return !suffix;
        size_t suffix_len = std::strlen(suffix);
        if (suffix_len > len_) return false;
        return std::memcmp(data_ + len_ - suffix_len, suffix, suffix_len) == 0;
    }
    
    // Find substring
    size_t find(const char* needle) const {
        if (!needle || !data_) return static_cast<size_t>(-1);
        const char* pos = std::strstr(data_, needle);
        if (!pos) return static_cast<size_t>(-1);
        return pos - data_;
    }
    
    // Replace all occurrences
    // @lifetime: owned
    String replace(const char* from, const char* to) const {
        if (!from || !to || !data_) return clone();
        
        size_t from_len = std::strlen(from);
        size_t to_len = std::strlen(to);
        if (from_len == 0) return clone();
        
        // Count occurrences
        size_t count = 0;
        const char* pos = data_;
        while ((pos = std::strstr(pos, from)) != nullptr) {
            count++;
            pos += from_len;
        }
        
        if (count == 0) return clone();
        
        // Calculate new length
        size_t new_len = len_ + count * (to_len - from_len);
        
        String result;
        result.grow(new_len + 1);
        
        const char* src = data_;
        char* dst = result.data_;
        
        while ((pos = std::strstr(src, from)) != nullptr) {
            size_t prefix_len = pos - src;
            std::memcpy(dst, src, prefix_len);
            dst += prefix_len;
            std::memcpy(dst, to, to_len);
            dst += to_len;
            src = pos + from_len;
        }
        
        // Copy remainder
        size_t remainder = data_ + len_ - src;
        std::memcpy(dst, src, remainder);
        
        result.len_ = new_len;
        result.ensure_null_terminated();
        return result;
    }
    
    // Trim whitespace
    // @lifetime: owned
    String trim() const {
        if (!data_ || len_ == 0) return String();
        
        size_t start = 0;
        while (start < len_ && std::isspace(data_[start])) {
            start++;
        }
        
        if (start == len_) return String();
        
        size_t end = len_;
        while (end > start && std::isspace(data_[end - 1])) {
            end--;
        }
        
        String result;
        size_t new_len = end - start;
        if (new_len > 0) {  // Check for valid length
            result.grow(new_len + 1);
            std::memcpy(result.data_, data_ + start, new_len);
            result.len_ = new_len;
            result.ensure_null_terminated();
        }
        return result;
    }
    
    // Split string by delimiter
    // @lifetime: owned
    std::vector<String> split(char delim) const {
        std::vector<String> result;
        if (!data_ || len_ == 0) return result;
        
        size_t start = 0;
        for (size_t i = 0; i < len_; i++) {
            if (data_[i] == delim) {
                String part;
                size_t part_len = i - start;
                if (part_len > 0) {
                    part.grow(part_len + 1);
                    std::memcpy(part.data_, data_ + start, part_len);
                    part.len_ = part_len;
                    part.ensure_null_terminated();
                }
                result.push_back(std::move(part));
                start = i + 1;
            }
        }
        
        // Add last part
        if (start < len_) {
            String part;
            size_t part_len = len_ - start;
            if (part_len > 0) {  // Check for valid length
                part.grow(part_len + 1);
                std::memcpy(part.data_, data_ + start, part_len);
                part.len_ = part_len;
                part.ensure_null_terminated();
            }
            result.push_back(std::move(part));
        } else if (start == len_ && len_ > 0 && data_[len_ - 1] == delim) {
            // If string ends with delimiter, add empty part
            result.push_back(String());
        }
        
        return result;
    }
    
    // Convert to uppercase
    // @lifetime: owned
    String to_uppercase() const {
        String result = clone();
        for (size_t i = 0; i < result.len_; i++) {
            result.data_[i] = std::toupper(result.data_[i]);
        }
        return result;
    }
    
    // Convert to lowercase
    // @lifetime: owned
    String to_lowercase() const {
        String result = clone();
        for (size_t i = 0; i < result.len_; i++) {
            result.data_[i] = std::tolower(result.data_[i]);
        }
        return result;
    }
    
    // Friend function for stream output
    friend std::ostream& operator<<(std::ostream& os, const String& s) {
        if (s.data_) {
            os.write(s.data_, s.len_);
        }
        return os;
    }
};

inline Box<String> into_boxed_str(String value) {
    return Box<String>::new_(std::move(value));
}

inline Box<String> into_boxed_str(std::string value) {
    return Box<String>::new_(String::from(value));
}

inline Box<String> into_boxed_str(std::string_view value) {
    return Box<String>::new_(String::from(value));
}

inline Box<String> into_boxed_str(const char* value) {
    return Box<String>::new_(String::from(value));
}

// str - borrowed string slice (similar to Rust's &str)
// This is a non-owning view into a string
class str {
private:
    const char* data_;
    size_t len_;
    
public:
    // Constructors
    str() : data_(nullptr), len_(0) {}
    str(const char* s) : data_(s), len_(s ? std::strlen(s) : 0) {}
    str(const char* s, size_t len) : data_(s), len_(len) {}
    str(const String& s) : data_(s.as_ptr()), len_(s.len()) {}
    str(std::string_view sv) : data_(sv.data()), len_(sv.length()) {}
    
    // Length and emptiness
    size_t len() const { return len_; }
    bool is_empty() const { return len_ == 0; }
    
    // Get as C string (may not be null-terminated!)
    // @lifetime: (&'a) -> &'a
    const char* as_ptr() const { return data_; }
    
    // Get as string_view
    // @lifetime: (&'a) -> &'a
    std::string_view as_str() const {
        return data_ ? std::string_view(data_, len_) : std::string_view();
    }
    
    // Convert to owned String
    // @lifetime: owned
    String to_string() const {
        String s;
        if (data_ && len_ > 0) {
            s = String::with_capacity(len_);
            s.push_str(std::string_view(data_, len_).data());
        }
        return s;
    }
    
    // Character access
    // @lifetime: (&'a) -> &'a
    const char& operator[](size_t idx) const {
        if (idx >= len_) {
            throw std::out_of_range("index out of bounds");
        }
        return data_[idx];
    }
    
    // Iterators
    // @lifetime: (&'a) -> &'a
    const char* begin() const { return data_; }
    // @lifetime: (&'a) -> &'a
    const char* end() const { return data_ ? data_ + len_ : nullptr; }
    
    // Comparison
    bool operator==(const str& other) const {
        if (len_ != other.len_) return false;
        if (!data_ && !other.data_) return true;
        if (!data_ || !other.data_) return false;
        return std::memcmp(data_, other.data_, len_) == 0;
    }
    
    bool operator!=(const str& other) const { return !(*this == other); }
};

// Helper function: Rust str::as_bytes() equivalent for std::string_view.
// Returns a std::span<const uint8_t> representing the raw bytes of the string.
// @lifetime: (&'a) -> &'a
inline std::span<const uint8_t> as_bytes(std::string_view sv) {
    return std::span<const uint8_t>(reinterpret_cast<const uint8_t*>(sv.data()), sv.size());
}

// Helper function: Rust str::split_at() equivalent.
// Splits a string_view into (left, right) at the provided byte offset.
// Rust requires UTF-8 character boundaries for `str::split_at`; enforce the
// same boundary check for borrowed UTF-8 string views.
inline std::tuple<std::string_view, std::string_view> split_at(std::string_view sv, size_t mid) {
    if (mid > sv.size()) {
        throw std::out_of_range("split_at index out of bounds");
    }
    if (mid < sv.size()) {
        const auto byte = static_cast<unsigned char>(sv[mid]);
        if ((byte & 0xC0u) == 0x80u) {
            throw std::out_of_range("split_at index is not a UTF-8 character boundary");
        }
    }
    return std::make_tuple(sv.substr(0, mid), sv.substr(mid));
}

// Symmetric comparison: allow `"str" == rusty::String` and `"str" == rusty::str`
inline bool operator==(const char* lhs, const String& rhs) { return rhs == lhs; }
inline bool operator!=(const char* lhs, const String& rhs) { return !(rhs == lhs); }
inline bool operator==(const char* lhs, const str& rhs) {
    return std::string_view(lhs ? lhs : "") == rhs.as_str();
}
inline bool operator!=(const char* lhs, const str& rhs) { return !(lhs == rhs); }
// Also support string_view comparisons
inline bool operator==(std::string_view lhs, const String& rhs) { return lhs == rhs.as_str(); }
inline bool operator==(const String& lhs, std::string_view rhs) { return lhs.as_str() == rhs; }

// Factory functions
// @lifetime: owned
inline String string(const char* s) {
    return String::from(s);
}

// @lifetime: owned
inline String string(const std::string& s) {
    return String::from(s);
}

// @lifetime: owned
inline String string(std::string_view sv) {
    return String::from(sv);
}

} // namespace rusty

// Specialization of std::hash for rusty::String
namespace std {
    template<>
    struct hash<rusty::String> {
        size_t operator()(const rusty::String& s) const noexcept {
            // Simple hash using djb2 algorithm
            size_t hash = 5381;
            std::string_view sv = s.as_str();
            for (char c : sv) {
                hash = ((hash << 5) + hash) + c; // hash * 33 + c
            }
            return hash;
        }
    };
}

#endif // RUSTY_STRING_HPP
