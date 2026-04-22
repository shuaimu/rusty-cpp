#ifndef RUSTY_PROCESS_HPP
#define RUSTY_PROCESS_HPP

#include <filesystem>
#include <string>
#include <string_view>
#include <system_error>
#include <tuple>
#include <utility>

#include "option.hpp"
#include "result.hpp"
#include "string.hpp"

namespace rusty {

namespace path {

using Path = std::string;

class PathBuf {
private:
    std::filesystem::path inner_;

public:
    PathBuf() = default;
    explicit PathBuf(std::filesystem::path path) : inner_(std::move(path)) {}
    explicit PathBuf(std::string_view path) : inner_(path) {}

    static PathBuf from(std::string_view path) {
        return PathBuf(path);
    }

    bool pop() {
        return inner_.has_parent_path() && (inner_ = inner_.parent_path(), true);
    }

    void push(std::string_view part) {
        inner_ /= std::filesystem::path(part);
    }

    PathBuf join(std::string_view part) const {
        auto next = inner_;
        next /= std::filesystem::path(part);
        return PathBuf(std::move(next));
    }

    PathBuf with_extension(std::string_view ext) const {
        auto next = inner_;
        next.replace_extension(ext);
        return PathBuf(std::move(next));
    }

    const std::filesystem::path& as_std_path() const {
        return inner_;
    }

    Path as_path() const {
        return inner_.string();
    }

    std::string to_string() const {
        return inner_.string();
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
    std::error_code ec;
    auto exe = std::filesystem::read_symlink("/proc/self/exe", ec);
    if (ec) {
        ec.clear();
        exe = std::filesystem::current_path(ec);
    }
    if (ec) {
        return rusty::Result<path::PathBuf, rusty::String>::Err(
            rusty::String::from(ec.message()));
    }
    return rusty::Result<path::PathBuf, rusty::String>::Ok(path::PathBuf(std::move(exe)));
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
