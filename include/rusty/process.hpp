#ifndef RUSTY_PROCESS_HPP
#define RUSTY_PROCESS_HPP

#include <cerrno>
#include <cstring>
#include <string>
#include <string_view>
#include <tuple>
#include <utility>
#if defined(_WIN32)
#  include <direct.h>
#else
#  include <limits.h>
#  include <unistd.h>
#endif

#include "option.hpp"
#include "result.hpp"
#include "string.hpp"

namespace rusty {

namespace path {

using Path = std::string;

class PathBuf {
private:
    std::string inner_;

    static bool is_separator(char c) {
        return c == '/' || c == '\\';
    }

    static std::size_t root_prefix_length(std::string_view s) {
        if (s.empty()) {
            return 0;
        }
        if (is_separator(s.front())) {
            return 1;
        }
        return 0;
    }

    static std::string_view trim_trailing_separators(std::string_view s) {
        std::size_t keep = s.size();
        const std::size_t root = root_prefix_length(s);
        while (keep > root && is_separator(s[keep - 1])) {
            --keep;
        }
        return s.substr(0, keep);
    }

    static std::string normalize_separators(std::string_view s) {
        std::string out(s);
        for (char& ch : out) {
            if (ch == '\\') {
                ch = '/';
            }
        }
        return out;
    }

public:
    PathBuf() = default;
    explicit PathBuf(std::string path) : inner_(normalize_separators(path)) {}
    explicit PathBuf(std::string_view path) : inner_(normalize_separators(path)) {}

    static PathBuf from(std::string_view path) {
        return PathBuf(path);
    }

    bool pop() {
        const auto trimmed = trim_trailing_separators(inner_);
        const std::size_t root = root_prefix_length(trimmed);
        if (trimmed.size() <= root) {
            return false;
        }
        const auto pos = trimmed.find_last_of('/');
        if (pos == std::string::npos) {
            return false;
        }
        if (pos < root) {
            inner_ = std::string(trimmed.substr(0, root));
        } else {
            inner_ = std::string(trimmed.substr(0, pos));
        }
        if (inner_.empty() && root == 1) {
            inner_ = "/";
        }
        return true;
    }

    void push(std::string_view part) {
        auto rhs = normalize_separators(part);
        if (rhs.empty()) {
            return;
        }
        if (!rhs.empty() && rhs.front() == '/') {
            inner_ = std::move(rhs);
            return;
        }
        const auto base = trim_trailing_separators(inner_);
        if (base.empty()) {
            inner_ = rhs;
            return;
        }
        inner_ = std::string(base);
        inner_ += '/';
        inner_ += rhs;
    }

    PathBuf join(std::string_view part) const {
        auto next = inner_;
        PathBuf out(std::move(next));
        out.push(part);
        return out;
    }

    PathBuf with_extension(std::string_view ext) const {
        auto next = trim_trailing_separators(inner_);
        std::string out(next);
        const auto slash = out.find_last_of('/');
        const std::size_t name_begin = (slash == std::string::npos) ? 0 : slash + 1;
        const auto dot = out.find_last_of('.');
        const bool has_ext = dot != std::string::npos && dot >= name_begin;
        if (has_ext) {
            out.erase(dot);
        }
        if (!ext.empty()) {
            if (ext.front() != '.') {
                out.push_back('.');
            }
            out.append(ext.begin(), ext.end());
        }
        return PathBuf(std::move(out));
    }

    const std::string& as_std_path() const {
        return inner_;
    }

    Path as_path() const {
        return inner_;
    }

    std::string to_string() const {
        return inner_;
    }
};

} // namespace path

namespace env {

namespace consts {
#if defined(_WIN32)
inline constexpr const char* EXE_EXTENSION = "exe";
#else
inline constexpr const char* EXE_EXTENSION = "";
#endif
} // namespace consts

inline rusty::Result<path::PathBuf, rusty::String> current_exe() {
#if defined(_WIN32)
    char cwd_buf[4096];
    if (_getcwd(cwd_buf, sizeof(cwd_buf)) != nullptr) {
        return rusty::Result<path::PathBuf, rusty::String>::Ok(path::PathBuf(std::string(cwd_buf)));
    }
    return rusty::Result<path::PathBuf, rusty::String>::Err(
        rusty::String::from(std::string("current_exe fallback failed: ") + std::strerror(errno)));
#else
    char exe_buf[PATH_MAX];
    const ssize_t n = ::readlink("/proc/self/exe", exe_buf, sizeof(exe_buf) - 1);
    if (n > 0) {
        exe_buf[n] = '\0';
        return rusty::Result<path::PathBuf, rusty::String>::Ok(path::PathBuf(std::string(exe_buf)));
    }

    char cwd_buf[PATH_MAX];
    if (::getcwd(cwd_buf, sizeof(cwd_buf)) != nullptr) {
        return rusty::Result<path::PathBuf, rusty::String>::Ok(path::PathBuf(std::string(cwd_buf)));
    }
    return rusty::Result<path::PathBuf, rusty::String>::Err(
        rusty::String::from(std::string("current_exe fallback failed: ") + std::strerror(errno)));
#endif
}

} // namespace env

namespace process {

class ExitStatus {
private:
    int code_{0};

public:
    ExitStatus() = default;
    explicit ExitStatus(int code) : code_(code) {}

    int code() const {
        return code_;
    }

    bool success() const {
        return code_ == 0;
    }
};

class Child {
public:
    Child() = default;

    rusty::Result<rusty::Option<ExitStatus>, rusty::String> try_wait() {
        // Conservative compatibility stub: report "still running".
        return rusty::Result<rusty::Option<ExitStatus>, rusty::String>::Ok(
            rusty::Option<ExitStatus>(rusty::None));
    }

    rusty::Result<std::tuple<>, rusty::String> kill() {
        return rusty::Result<std::tuple<>, rusty::String>::Ok(std::tuple<>{});
    }
};

class Command {
private:
    path::PathBuf program_;

public:
    explicit Command(path::PathBuf program) : program_(std::move(program)) {}

    static Command new_(path::PathBuf program) {
        return Command(std::move(program));
    }

    rusty::Result<Child, rusty::String> spawn() const {
        (void)program_;
        return rusty::Result<Child, rusty::String>::Ok(Child{});
    }
};

} // namespace process

} // namespace rusty

#endif // RUSTY_PROCESS_HPP
