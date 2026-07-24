#include <algorithm>
#include <chrono>
#include <cctype>
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <iterator>
#include <map>
#include <optional>
#include <regex>
#include <sstream>
#include <string>
#include <string_view>
#include <vector>

namespace {

struct OpenDocument {
    std::string uri;
    std::filesystem::path path;
    std::string text;
    std::optional<long long> version;
};

struct ServerConfig {
    std::string checker_path;
    std::vector<std::filesystem::path> include_paths;
    std::vector<std::string> defines;
    std::optional<std::filesystem::path> compile_commands;
};

struct CheckerDiagnostic {
    std::size_t line = 0;
    std::string message;
};

std::map<std::string, OpenDocument> g_documents;
ServerConfig g_config;

std::string trim_cr(std::string line) {
    if (!line.empty() && line.back() == '\r') {
        line.pop_back();
    }
    return line;
}

std::optional<std::string> read_lsp_message() {
    std::string line;
    std::optional<std::size_t> content_length;

    while (std::getline(std::cin, line)) {
        line = trim_cr(line);
        if (line.empty()) {
            break;
        }

        auto colon = line.find(':');
        if (colon == std::string::npos) {
            continue;
        }

        std::string name = line.substr(0, colon);
        std::string value = line.substr(colon + 1);
        value.erase(value.begin(), std::find_if(value.begin(), value.end(), [](unsigned char c) {
                        return !std::isspace(c);
                    }));

        std::transform(name.begin(), name.end(), name.begin(), [](unsigned char c) {
            return static_cast<char>(std::tolower(c));
        });
        if (name == "content-length") {
            content_length = static_cast<std::size_t>(std::stoull(value));
        }
    }

    if (!content_length) {
        return std::nullopt;
    }

    std::string body(*content_length, '\0');
    std::cin.read(body.data(), static_cast<std::streamsize>(*content_length));
    if (std::cin.gcount() != static_cast<std::streamsize>(*content_length)) {
        return std::nullopt;
    }
    return body;
}

void write_lsp_message(const std::string& body) {
    std::cout << "Content-Length: " << body.size() << "\r\n\r\n" << body;
    std::cout.flush();
}

std::string json_escape(std::string_view text) {
    std::string out;
    out.reserve(text.size() + 8);
    for (char c : text) {
        switch (c) {
            case '\\':
                out += "\\\\";
                break;
            case '"':
                out += "\\\"";
                break;
            case '\n':
                out += "\\n";
                break;
            case '\r':
                out += "\\r";
                break;
            case '\t':
                out += "\\t";
                break;
            default:
                if (static_cast<unsigned char>(c) < 0x20) {
                    out += ' ';
                } else {
                    out += c;
                }
                break;
        }
    }
    return out;
}

std::optional<std::pair<std::string, std::size_t>> parse_json_string_at(const std::string& json,
                                                                        std::size_t quote_pos) {
    if (quote_pos >= json.size() || json[quote_pos] != '"') {
        return std::nullopt;
    }

    std::string out;
    for (std::size_t i = quote_pos + 1; i < json.size(); ++i) {
        char c = json[i];
        if (c == '"') {
            return std::make_pair(out, i + 1);
        }
        if (c != '\\') {
            out += c;
            continue;
        }
        if (++i >= json.size()) {
            return std::nullopt;
        }
        switch (json[i]) {
            case '"':
            case '\\':
            case '/':
                out += json[i];
                break;
            case 'b':
                out += '\b';
                break;
            case 'f':
                out += '\f';
                break;
            case 'n':
                out += '\n';
                break;
            case 'r':
                out += '\r';
                break;
            case 't':
                out += '\t';
                break;
            case 'u':
                out += '?';
                i += std::min<std::size_t>(4, json.size() - i - 1);
                break;
            default:
                out += json[i];
                break;
        }
    }

    return std::nullopt;
}

std::optional<std::size_t> find_key(const std::string& json,
                                    std::string_view key,
                                    std::size_t start = 0) {
    std::string needle = "\"" + std::string(key) + "\"";
    auto pos = json.find(needle, start);
    if (pos == std::string::npos) {
        return std::nullopt;
    }
    auto colon = json.find(':', pos + needle.size());
    if (colon == std::string::npos) {
        return std::nullopt;
    }
    return colon + 1;
}
std::optional<std::string> find_string_field(const std::string& json,
                                             std::string_view key,
                                             std::size_t start = 0) {
    auto value_pos = find_key(json, key, start);
    if (!value_pos) {
        return std::nullopt;
    }
    auto quote = json.find('"', *value_pos);
    if (quote == std::string::npos) {
        return std::nullopt;
    }
    auto parsed = parse_json_string_at(json, quote);
    if (!parsed) {
        return std::nullopt;
    }
    return parsed->first;
}

std::optional<std::string> find_raw_field(const std::string& json, std::string_view key) {
    auto value_pos = find_key(json, key);
    if (!value_pos) {
        return std::nullopt;
    }
    std::size_t pos = *value_pos;
    while (pos < json.size() && std::isspace(static_cast<unsigned char>(json[pos]))) {
        ++pos;
    }
    if (pos >= json.size()) {
        return std::nullopt;
    }
    if (json[pos] == '"') {
        auto parsed = parse_json_string_at(json, pos);
        if (!parsed) {
            return std::nullopt;
        }
        return "\"" + json_escape(parsed->first) + "\"";
    }
    auto end = pos;
    while (end < json.size() && json[end] != ',' && json[end] != '}' && json[end] != '\n' &&
           json[end] != '\r') {
        ++end;
    }
    return json.substr(pos, end - pos);
}

std::vector<std::string> find_string_array_field(const std::string& json, std::string_view key) {
    std::vector<std::string> values;
    auto value_pos = find_key(json, key);
    if (!value_pos) {
        return values;
    }
    auto open = json.find('[', *value_pos);
    auto close = json.find(']', open);
    if (open == std::string::npos || close == std::string::npos) {
        return values;
    }

    std::size_t pos = open + 1;
    while (pos < close) {
        auto quote = json.find('"', pos);
        if (quote == std::string::npos || quote > close) {
            break;
        }
        auto parsed = parse_json_string_at(json, quote);
        if (!parsed) {
            break;
        }
        values.push_back(parsed->first);
        pos = parsed->second;
    }

    return values;
}

int hex_value(char c) {
    if (c >= '0' && c <= '9') {
        return c - '0';
    }
    if (c >= 'a' && c <= 'f') {
        return c - 'a' + 10;
    }
    if (c >= 'A' && c <= 'F') {
        return c - 'A' + 10;
    }
    return -1;
}

std::string percent_decode(std::string_view input) {
    std::string out;
    for (std::size_t i = 0; i < input.size(); ++i) {
        if (input[i] == '%' && i + 2 < input.size()) {
            int high = hex_value(input[i + 1]);
            int low = hex_value(input[i + 2]);
            if (high >= 0 && low >= 0) {
                out += static_cast<char>((high << 4) | low);
                i += 2;
                continue;
            }
        }
        out += input[i];
    }
    return out;
}

std::optional<std::filesystem::path> uri_to_path(const std::string& uri) {
    constexpr std::string_view prefix = "file://";
    if (!uri.starts_with(prefix)) {
        return std::nullopt;
    }
    return std::filesystem::path(percent_decode(std::string_view(uri).substr(prefix.size())));
}

std::string shell_quote(const std::filesystem::path& path) {
    std::string value = path.string();
    std::string out = "'";
    for (char c : value) {
        if (c == '\'') {
            out += "'\\''";
        } else {
            out += c;
        }
    }
    out += "'";
    return out;
}

std::string checker_path() {
    if (!g_config.checker_path.empty()) {
        return g_config.checker_path;
    }
    if (const char* env_path = std::getenv("RUSTY_CPP_CHECKER")) {
        return env_path;
    }
    return "rusty-cpp-checker";
}

std::filesystem::path temp_path_for_document(const OpenDocument& document) {
    auto parent = document.path.parent_path();
    if (parent.empty()) {
        parent = ".";
    }
    std::string stem = document.path.stem().string();
    if (stem.empty()) {
        stem = "document";
    }
    std::string extension = document.path.extension().string();
    if (extension.empty()) {
        extension = ".cpp";
    }
    auto stamp = std::chrono::steady_clock::now().time_since_epoch().count();
    return parent / ("." + stem + ".rusty-lsp-" + std::to_string(stamp) + extension);
}

std::optional<std::filesystem::path> write_temp_document(const OpenDocument& document) {
    auto path = temp_path_for_document(document);
    std::ofstream out(path);
    if (!out) {
        return std::nullopt;
    }
    out << document.text;
    return path;
}

std::string run_checker(const std::filesystem::path& path) {
    std::ostringstream command;
    command << shell_quote(checker_path()) << " " << shell_quote(path);
    for (const auto& include_path : g_config.include_paths) {
        command << " -I " << shell_quote(include_path);
    }
    for (const auto& define : g_config.defines) {
        command << " -D " << shell_quote(define);
    }
    if (g_config.compile_commands) {
        command << " --compile-commands " << shell_quote(*g_config.compile_commands);
    }
    command << " 2>&1";

    std::string output;
    FILE* pipe = popen(command.str().c_str(), "r");
    if (!pipe) {
        return "rusty-cpp-lsp: failed to run checker";
    }

    char buffer[4096];
    while (fgets(buffer, sizeof(buffer), pipe)) {
        output += buffer;
    }
    pclose(pipe);
    return output;
}

std::vector<std::string> split_lines(std::string_view text) {
    std::vector<std::string> lines;
    std::stringstream stream{std::string(text)};
    std::string line;
    while (std::getline(stream, line)) {
        lines.push_back(trim_cr(line));
    }
    return lines;
}

bool looks_like_function_start(const std::vector<std::string>& lines, std::size_t line) {
    std::string trimmed = lines[line];
    trimmed.erase(trimmed.begin(), std::find_if(trimmed.begin(), trimmed.end(), [](unsigned char c) {
                      return !std::isspace(c);
                  }));
    if (trimmed.empty() || trimmed.starts_with("//") || trimmed.starts_with("#")) {
        return false;
    }
    if (trimmed.find('(') == std::string::npos || trimmed.find(')') == std::string::npos) {
        return false;
    }
    if (trimmed.find('{') == std::string::npos) {
        auto next = std::find_if(std::next(lines.begin(), static_cast<long>(line + 1)), lines.end(),
                                 [](const std::string& candidate) {
                                     return candidate.find_first_not_of(" \t\r\n") !=
                                            std::string::npos;
                                 });
        if (next == lines.end() || next->find('{') == std::string::npos) {
            return false;
        }
    }

    static const std::regex control_re(R"(^\s*(if|for|while|switch|catch|else|do|try)\b)");
    return !std::regex_search(trimmed, control_re);
}

std::string leading_whitespace(std::string_view line) {
    std::string out;
    for (char c : line) {
        if (c != ' ' && c != '\t') {
            break;
        }
        out += c;
    }
    return out;
}

std::string trim_left(std::string_view line) {
    auto first = std::find_if(line.begin(), line.end(), [](unsigned char c) {
        return !std::isspace(c);
    });
    return std::string(first, line.end());
}

bool has_safety_annotation_before(const std::vector<std::string>& lines, std::size_t function_line) {
    if (function_line == 0) {
        return false;
    }

    for (std::size_t line = function_line; line > 0; --line) {
        std::string trimmed = trim_left(lines[line - 1]);
        if (trimmed.empty()) {
            continue;
        }
        return trimmed.starts_with("// @safe") || trimmed.starts_with("// @unsafe");
    }

    return false;
}

std::optional<std::size_t> find_enclosing_function_line(const std::vector<std::string>& lines,
                                                        std::size_t request_line) {
    if (lines.empty()) {
        return std::nullopt;
    }

    std::size_t line = std::min(request_line, lines.size() - 1);
    while (true) {
        if (looks_like_function_start(lines, line)) {
            return line;
        }
        if (line == 0) {
            break;
        }
        --line;
    }

    return std::nullopt;
}

std::size_t fallback_function_line(std::string_view text) {
    auto lines = split_lines(text);
    for (std::size_t i = 0; i < lines.size(); ++i) {
        if (looks_like_function_start(lines, i)) {
            return i;
        }
    }
    return 0;
}

std::vector<CheckerDiagnostic> parse_checker_diagnostics(const std::string& output,
                                                         std::string_view document_text) {
    std::vector<CheckerDiagnostic> diagnostics;
    std::regex diagnostic_re(R"(^In function '([^']+)': (.*)$)");
    std::regex line_re(R"(\bat line ([0-9]+)\b)");
    std::size_t fallback_line = fallback_function_line(document_text);

    std::stringstream stream(output);
    std::string line;
    while (std::getline(stream, line)) {
        line = trim_cr(line);
        std::smatch match;
        if (!std::regex_match(line, match, diagnostic_re)) {
            continue;
        }

        std::size_t diagnostic_line = fallback_line;
        std::smatch line_match;
        std::string message = match[2].str();
        if (std::regex_search(message, line_match, line_re)) {
            auto one_based = static_cast<std::size_t>(std::stoull(line_match[1].str()));
            diagnostic_line = one_based > 0 ? one_based - 1 : 0;
        }

        diagnostics.push_back({diagnostic_line, message});
    }

    return diagnostics;
}

std::string diagnostic_json(const CheckerDiagnostic& diagnostic, std::string_view document_text) {
    auto lines = split_lines(document_text);
    std::size_t end_character = 1;
    if (diagnostic.line < lines.size()) {
        end_character = lines[diagnostic.line].size();
    }

    std::ostringstream out;
    out << R"({"range":{"start":{"line":)" << diagnostic.line
        << R"(,"character":0},"end":{"line":)" << diagnostic.line
        << R"(,"character":)" << end_character
        << R"(}},"severity":1,"source":"rusty-cpp","message":")" << json_escape(diagnostic.message)
        << R"("})";
    return out.str();
}

std::string diagnostics_array_json(const std::vector<CheckerDiagnostic>& diagnostics,
                                   std::string_view document_text) {
    std::ostringstream out;
    out << "[";
    for (std::size_t i = 0; i < diagnostics.size(); ++i) {
        if (i > 0) {
            out << ",";
        }
        out << diagnostic_json(diagnostics[i], document_text);
    }
    out << "]";
    return out.str();
}

void publish_diagnostics(const OpenDocument& document) {
    auto temp_path = write_temp_document(document);
    std::vector<CheckerDiagnostic> diagnostics;
    if (temp_path) {
        std::string checker_output = run_checker(*temp_path);
        std::filesystem::remove(*temp_path);
        diagnostics = parse_checker_diagnostics(checker_output, document.text);
    }

    std::ostringstream body;
    body << R"({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":")"
         << json_escape(document.uri) << R"(","diagnostics":)"
         << diagnostics_array_json(diagnostics, document.text) << "}}";
    write_lsp_message(body.str());
}

void write_response(const std::string& id, const std::string& result_json) {
    write_lsp_message(R"({"jsonrpc":"2.0","id":)" + id + R"(,"result":)" + result_json + "}");
}

std::optional<std::size_t> parse_size_at(const std::string& text, std::size_t pos) {
    while (pos < text.size() && std::isspace(static_cast<unsigned char>(text[pos]))) {
        ++pos;
    }
    std::size_t start = pos;
    while (pos < text.size() && std::isdigit(static_cast<unsigned char>(text[pos]))) {
        ++pos;
    }
    if (start == pos) {
        return std::nullopt;
    }
    return static_cast<std::size_t>(std::stoull(text.substr(start, pos - start)));
}

std::optional<std::size_t> find_code_action_request_line(const std::string& message) {
    auto range_pos = message.find("\"range\"");
    if (range_pos == std::string::npos) {
        return std::nullopt;
    }
    auto start_pos = message.find("\"start\"", range_pos);
    if (start_pos == std::string::npos) {
        return std::nullopt;
    }
    auto line_pos = find_key(message, "line", start_pos);
    if (!line_pos) {
        return std::nullopt;
    }
    return parse_size_at(message, *line_pos);
}

std::string safety_annotation_action_json(std::string_view title,
                                          std::string_view annotation,
                                          const std::string& uri,
                                          std::size_t function_line,
                                          const std::string& indent) {
    std::ostringstream out;
    out << R"({"title":")" << json_escape(title)
        << R"(","kind":"quickfix","edit":{"changes":{")" << json_escape(uri)
        << R"(":[{"range":{"start":{"line":)" << function_line
        << R"(,"character":0},"end":{"line":)" << function_line
        << R"(,"character":0}},"newText":")" << json_escape(indent) << json_escape(annotation)
        << R"(\n"}]}}})";
    return out.str();
}

std::string code_actions_for_document(const OpenDocument& document, std::size_t request_line) {
    auto lines = split_lines(document.text);
    auto function_line = find_enclosing_function_line(lines, request_line);
    if (!function_line || has_safety_annotation_before(lines, *function_line)) {
        return "[]";
    }

    std::string indent = leading_whitespace(lines[*function_line]);
    std::ostringstream out;
    out << "["
        << safety_annotation_action_json("Mark function as @safe", "// @safe", document.uri,
                                         *function_line, indent)
        << ","
        << safety_annotation_action_json("Mark function as @unsafe", "// @unsafe", document.uri,
                                         *function_line, indent)
        << "]";
    return out.str();
}

std::string initialize_result_json() {
    return R"({"capabilities":{"textDocumentSync":{"openClose":true,"change":1,"save":true},"codeActionProvider":true},"serverInfo":{"name":"rusty-cpp-lsp","version":"0.1.1"}})";
}

void parse_initialization_options(const std::string& message) {
    if (auto checker = find_string_field(message, "checkerPath")) {
        g_config.checker_path = *checker;
    }
    for (const auto& include_path : find_string_array_field(message, "includePaths")) {
        g_config.include_paths.emplace_back(include_path);
    }
    for (const auto& define : find_string_array_field(message, "defines")) {
        g_config.defines.push_back(define);
    }
    if (auto compile_commands = find_string_field(message, "compileCommands")) {
        g_config.compile_commands = std::filesystem::path(*compile_commands);
    }
}

std::optional<OpenDocument> document_from_did_open(const std::string& message) {
    auto uri = find_string_field(message, "uri");
    auto text = find_string_field(message, "text");
    if (!uri || !text) {
        return std::nullopt;
    }
    auto path = uri_to_path(*uri);
    if (!path) {
        return std::nullopt;
    }
    return OpenDocument{*uri, *path, *text, std::nullopt};
}

std::optional<OpenDocument> document_from_did_change(const std::string& message) {
    auto uri = find_string_field(message, "uri");
    if (!uri) {
        return std::nullopt;
    }
    auto existing = g_documents.find(*uri);
    if (existing == g_documents.end()) {
        return std::nullopt;
    }

    OpenDocument document = existing->second;
    auto changes_pos = message.find("\"contentChanges\"");
    auto text = find_string_field(message, "text", changes_pos == std::string::npos ? 0 : changes_pos);
    if (text) {
        document.text = *text;
    }
    return document;
}

void handle_message(const std::string& message) {
    auto method = find_string_field(message, "method");
    if (!method) {
        return;
    }
    auto id = find_raw_field(message, "id");

    if (*method == "initialize") {
        parse_initialization_options(message);
        if (id) {
            write_response(*id, initialize_result_json());
        }
    } else if (*method == "initialized") {
        return;
    } else if (*method == "shutdown") {
        if (id) {
            write_response(*id, "null");
        }
    } else if (*method == "textDocument/didOpen") {
        if (auto document = document_from_did_open(message)) {
            g_documents[document->uri] = *document;
            publish_diagnostics(*document);
        }
    } else if (*method == "textDocument/didChange") {
        if (auto document = document_from_did_change(message)) {
            g_documents[document->uri] = *document;
            publish_diagnostics(*document);
        }
    } else if (*method == "textDocument/didSave") {
        auto uri = find_string_field(message, "uri");
        if (uri) {
            auto existing = g_documents.find(*uri);
            if (existing != g_documents.end()) {
                publish_diagnostics(existing->second);
            }
        }
    } else if (*method == "textDocument/codeAction") {
        auto uri = find_string_field(message, "uri");
        auto request_line = find_code_action_request_line(message);
        if (id && uri && request_line) {
            auto existing = g_documents.find(*uri);
            if (existing != g_documents.end()) {
                write_response(*id, code_actions_for_document(existing->second, *request_line));
            } else {
                write_response(*id, "[]");
            }
        } else if (id) {
            write_response(*id, "[]");
        }
    } else if (id) {
        write_response(*id, "null");
    }
}

}  // namespace

int main() {
    while (auto message = read_lsp_message()) {
        if (auto method = find_string_field(*message, "method"); method && *method == "exit") {
            break;
        }
        handle_message(*message);
    }
    return 0;
}
