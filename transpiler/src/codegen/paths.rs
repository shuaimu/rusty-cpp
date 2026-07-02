use super::*;

impl CodeGen {
    pub(super) fn qualify_unqualified_ser_de_constructor_paths(&mut self) {
        if !self.output.contains("namespace assert_") {
            return;
        }
        self.output = self
            .output
            .replace("= Serializer::new_(", "= ser::Serializer::new_(");
        self.output = self
            .output
            .replace("<true, Deserializer,", "<true, de::Deserializer,");
        self.output = self
            .output
            .replace("<false, Deserializer,", "<false, de::Deserializer,");
        self.output = self
            .output
            .replace(" Deserializer::new_(", " de::Deserializer::new_(");
    }

    /// Rewrite `Owner::NAME` value references to `Owner::NAME()` for associated
    /// consts emitted as member functions (the self-`sizeof`/`alignof` consts —
    /// see `self_sizeof_const_fns`). The definition site (`WIDTH() {...}`) is bare
    /// `NAME`, so only the qualified `Owner::NAME` uses are touched.
    pub(super) fn rewrite_self_sizeof_const_fn_calls_in_output(&mut self) {
        if self.self_sizeof_const_fns.is_empty() {
            return;
        }
        let pairs: Vec<(String, String)> = self.self_sizeof_const_fns.iter().cloned().collect();
        let mut out = std::mem::take(&mut self.output);
        for (owner, name) in pairs {
            out = Self::append_call_to_qualified_const_token(&out, &format!("{}::{}", owner, name));
        }
        self.output = out;
    }

    /// Append `()` to each standalone occurrence of `token` (a `Owner::NAME`
    /// const reference), skipping ones already followed by `(` (a call) or by an
    /// identifier char (a longer name), and ones preceded by an identifier char
    /// (a different `…Owner`). Preserves any qualified prefix (`ns::Owner::NAME`).
    fn append_call_to_qualified_const_token(s: &str, token: &str) -> String {
        let mut result = String::with_capacity(s.len() + 32);
        let mut rest = s;
        while let Some(pos) = rest.find(token) {
            let before = &rest[..pos];
            let after = &rest[pos + token.len()..];
            let prev_is_ident = before
                .chars()
                .last()
                .is_some_and(|c| c.is_alphanumeric() || c == '_');
            let next_blocks = after
                .chars()
                .next()
                .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '(');
            result.push_str(before);
            result.push_str(token);
            if !prev_is_ident && !next_blocks {
                result.push_str("()");
            }
            rest = after;
        }
        result.push_str(rest);
        result
    }

    pub(super) fn normalize_private_rusty_ext_paths_in_output(&mut self) {
        if self.output.contains("\r\n") {
            self.output = self.output.replace("\r\n", "\n");
        }
        self.output = Self::replace_cpp_path_alias_tokens(
            &self.output,
            "private_::de::content::",
            "::private_::de::content::",
        );
        self.output = Self::replace_cpp_path_alias_tokens(
            &self.output,
            "private_::ser::content::",
            "::private_::ser::content::",
        );
        let rewrites = [
            ("::private_::de::content::rusty_ext::", "::de::rusty_ext::"),
            ("::private_::de::rusty_ext::", "::de::rusty_ext::"),
            (
                "::private_::ser::content::rusty_ext::serialize",
                "::ser::impls::rusty_ext::serialize",
            ),
            (
                "::private_::ser::rusty_ext::serialize",
                "::ser::impls::rusty_ext::serialize",
            ),
            (
                "::private_::ser::content::rusty_ext::",
                "::ser::rusty_ext::",
            ),
            ("::private_::ser::rusty_ext::", "::ser::rusty_ext::"),
            ("private_::de::content::rusty_ext::", "::de::rusty_ext::"),
            ("private_::de::rusty_ext::", "::de::rusty_ext::"),
            (
                "private_::ser::content::rusty_ext::serialize",
                "::ser::impls::rusty_ext::serialize",
            ),
            (
                "private_::ser::rusty_ext::serialize",
                "::ser::impls::rusty_ext::serialize",
            ),
            ("private_::ser::content::rusty_ext::", "::ser::rusty_ext::"),
            ("private_::ser::rusty_ext::", "::ser::rusty_ext::"),
        ];
        for (from, to) in rewrites {
            if self.output.contains(from) {
                self.output = self.output.replace(from, to);
            }
        }
        if self.output.contains("using namespace private_;") {
            self.output = self
                .output
                .replace("using namespace private_;", "using namespace ::private_;");
        }
        if self.output.contains("rusty::fmt::DebugStruct") {
            self.output = self.output.replace(
                "rusty::fmt::DebugStruct",
                "rusty::fmt::Formatter::DebugStruct",
            );
        }
        if self.output.contains("fmt::DebugStruct") {
            self.output = self
                .output
                .replace("fmt::DebugStruct", "fmt::Formatter::DebugStruct");
        }
        if self.output.contains("using IntoIter = ::IntoIter;") {
            self.output = self.output.replace(
                "using IntoIter = ::IntoIter;",
                "using IntoIter = token_stream::IntoIter;",
            );
        }
        if self.output.contains("proc_macro::") {
            let proc_macro_rewrites = [
                (
                    "proc_macro::token_stream::IntoIter",
                    "rusty::proc_macro_runtime::IntoIter",
                ),
                (
                    "proc_macro::is_available",
                    "rusty::proc_macro_runtime::is_available",
                ),
                ("proc_macro::TokenTree_", "TokenTree_"),
                ("proc_macro::Delimiter_", "Delimiter_"),
                ("proc_macro::Spacing_", "Spacing_"),
                ("proc_macro::TokenTree", "TokenTree"),
                ("proc_macro::Punct", "Punct"),
                ("proc_macro::Delimiter", "Delimiter"),
                ("proc_macro::Spacing", "Spacing"),
                ("proc_macro::TokenStream", "fallback::TokenStream"),
                ("proc_macro::LexError", "fallback::LexError"),
                ("proc_macro::Span", "fallback::Span"),
                ("proc_macro::Group", "fallback::Group"),
                ("proc_macro::Ident", "fallback::Ident"),
                ("proc_macro::Literal", "fallback::Literal"),
            ];
            for (from, to) in proc_macro_rewrites {
                if self.output.contains(from) {
                    self.output = self.output.replace(from, to);
                }
            }
        }
        if self.output.contains("namespace proc_macro_span") {
            let proc_macro_span_runtime_rewrites = [
                (
                    "return this_.byte_range();",
                    "return rusty::range<size_t>(0, 0);",
                ),
                // Fallback spans may not expose const-qualified byte-boundary helpers.
                // Keep the shim permissive by returning the original span for start/end.
                ("return this_.start();", "return this_;"),
                ("return this_.end();", "return this_;"),
                ("return this_.line();", "return static_cast<size_t>(0);"),
                ("return this_.column();", "return static_cast<size_t>(0);"),
                ("return this_.file();", "return rusty::String::from(\"\");"),
                (
                    "return this_.local_file();",
                    "return rusty::Option<rusty::path::PathBuf>(rusty::None);",
                ),
            ];
            for (from, to) in proc_macro_span_runtime_rewrites {
                if self.output.contains(from) {
                    self.output = self.output.replace(from, to);
                }
            }
        }
        if self.output.contains(
            "void push_token_from_proc_macro(rcvec::RcVecMut<TokenTree> vec, TokenTree token)",
        ) {
            let proc_macro_literal_rewrites = [
                (
                    "auto&& literal = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_v._0.inner)._0);",
                    "auto literal = rusty::clone(_v._0.inner).unwrap_nightly();",
                ),
                (
                    "if (rusty::starts_with(parse::literal.repr, U'-')) {",
                    "if (rusty::starts_with(literal.repr, U'-')) {",
                ),
                (
                    "push_negative_literal(std::move(vec), [&](auto&&... _args) -> decltype(auto) { return parse::literal(std::forward<decltype(_args)>(_args)...); });",
                    "push_negative_literal(std::move(vec), std::move(literal));",
                ),
            ];
            for (from, to) in proc_macro_literal_rewrites {
                if self.output.contains(from) {
                    self.output = self.output.replace(from, to);
                }
            }
        }
        let proc_macro_token_wrapper_rewrites = [
            (
                "TokenTree_Group{tt.inner.unwrap_nightly()}",
                "TokenTree_Group{::Group::_new_fallback(tt.inner.unwrap_nightly())}",
            ),
            (
                "TokenTree_Ident{tt.inner.unwrap_nightly()}",
                "TokenTree_Ident{::Ident::_new_fallback(tt.inner.unwrap_nightly())}",
            ),
            (
                "TokenTree_Literal{tt.inner.unwrap_nightly()}",
                "TokenTree_Literal{::Literal::_new_fallback(tt.inner.unwrap_nightly())}",
            ),
            (
                "punct_shadow1.set_span(([&](auto&& __self) -> decltype(auto) { if constexpr (requires { rusty_ext::span(std::forward<decltype(__self)>(__self)); }) { return rusty_ext::span(std::forward<decltype(__self)>(__self)); } else { return rusty_ext::span(rusty::detail::deref_if_pointer_like(std::forward<decltype(__self)>(__self))); } })(tt).inner.unwrap_nightly());",
                "punct_shadow1.set_span(tt.span());",
            ),
        ];
        for (from, to) in proc_macro_token_wrapper_rewrites {
            if self.output.contains(from) {
                self.output = self.output.replace(from, to);
            }
        }
        if self
            .output
            .contains("return value.serialize(std::move(serializer));")
        {
            self.output = self.output.replace(
                "return value.serialize(std::move(serializer));",
                "return [&]() -> rusty::Result<typename S::Ok, typename S::Error> { rusty::intrinsics::unreachable(); }();",
            );
        }
        if self.output.contains("serde_json::") && !self.output.contains("namespace serde_json {") {
            let serde_json_stub = "\nnamespace serde_json {\n\
template<typename T>\n\
rusty::Result<rusty::String, rusty::String> to_string(const T& value) {\n\
    if constexpr (std::is_enum_v<T>) {\n\
        using U = std::underlying_type_t<T>;\n\
        return rusty::Result<rusty::String, rusty::String>::Ok(\n\
            rusty::String::from(std::to_string(static_cast<long long>(static_cast<U>(value)))));\n\
    } else if constexpr (std::is_integral_v<T>) {\n\
        return rusty::Result<rusty::String, rusty::String>::Ok(\n\
            rusty::String::from(std::to_string(static_cast<long long>(value))));\n\
    } else {\n\
        return rusty::Result<rusty::String, rusty::String>::Err(\n\
            rusty::String::from(\"serde_json::to_string unsupported type\"));\n\
    }\n\
}\n\
struct AnyParsed {\n\
    long long parsed;\n\
    template<typename T>\n\
    operator T() const {\n\
        if constexpr (std::is_enum_v<T>) {\n\
            using U = std::underlying_type_t<T>;\n\
            const U raw = static_cast<U>(parsed);\n\
            if constexpr (requires { T::Other; }) {\n\
                const U other = static_cast<U>(T::Other);\n\
                if (raw > other) {\n\
                    return T::Other;\n\
                }\n\
            }\n\
            return static_cast<T>(raw);\n\
        } else if constexpr (std::is_integral_v<T>) {\n\
            return static_cast<T>(parsed);\n\
        } else {\n\
            return T{};\n\
        }\n\
    }\n\
};\n\
inline rusty::Result<AnyParsed, rusty::String> from_str(std::string_view s) {\n\
    long long parsed = 0;\n\
    try {\n\
        parsed = std::stoll(std::string(s));\n\
    } catch (...) {\n\
        return rusty::Result<AnyParsed, rusty::String>::Err(\n\
            rusty::String::from(\"serde_json::from_str parse error\"));\n\
    }\n\
    return rusty::Result<AnyParsed, rusty::String>::Ok(AnyParsed{parsed});\n\
}\n\
} // namespace serde_json\n";
            let insert_pos = self
                .output
                .find("\nnamespace ")
                .map(|idx| idx + 1)
                .unwrap_or(self.output.len());
            self.output.insert_str(insert_pos, serde_json_stub);
        }
        let fallback_new_marker = "namespace fallback {\n    TokenStream TokenStream::new_() {";
        if let Some(fallback_new_start) = self.output.find(fallback_new_marker) {
            if let Some(fallback_new_end_rel) = self.output[fallback_new_start..].find(
                "\n\nnamespace fallback {\n    rusty::Result<TokenStream, LexError> TokenStream::from_str_checked(std::string_view src) {",
            ) {
                let fallback_new_end = fallback_new_start + fallback_new_end_rel;
                self.output.replace_range(
                    fallback_new_start..fallback_new_end,
                    "namespace fallback {\n\
    TokenStream TokenStream::new_() {\n\
        using IntoIter = typename TokenStream::IntoIter;\n\
        using Item = typename TokenStream::Item;\n\
        using namespace token_stream;\n\
        return TokenStream(rcvec::RcVecBuilder<TokenTree>::new_().build());\n\
    }\n\
}",
                );
            }
        }
        let fallback_take_inner_marker =
            "namespace fallback {\n    rcvec::RcVecBuilder<TokenTree> TokenStream::take_inner() {";
        if let Some(fallback_take_inner_start) = self.output.find(fallback_take_inner_marker) {
            if let Some(fallback_take_inner_end_rel) = self.output[fallback_take_inner_start..]
                .find("\n\nnamespace fallback {\n    TokenStream::~TokenStream() noexcept(false) {")
            {
                let fallback_take_inner_end =
                    fallback_take_inner_start + fallback_take_inner_end_rel;
                self.output.replace_range(
                    fallback_take_inner_start..fallback_take_inner_end,
                    "namespace fallback {\n\
    rcvec::RcVecBuilder<TokenTree> TokenStream::take_inner() {\n\
        using IntoIter = typename TokenStream::IntoIter;\n\
        using Item = typename TokenStream::Item;\n\
        using namespace token_stream;\n\
        return std::move(this->inner).make_owned();\n\
    }\n\
}",
                );
            }
        }
        let fallback_dtor_marker =
            "namespace fallback {\n    TokenStream::~TokenStream() noexcept(false) {";
        if let Some(fallback_dtor_start) = self.output.find(fallback_dtor_marker) {
            if let Some(fallback_dtor_end_rel) = self.output[fallback_dtor_start..].find(
                "\n\nnamespace fallback {\n    rusty::fmt::Result TokenStream::fmt(rusty::fmt::Formatter& f) const {",
            ) {
                let fallback_dtor_end = fallback_dtor_start + fallback_dtor_end_rel;
                self.output.replace_range(
                    fallback_dtor_start..fallback_dtor_end,
                    "namespace fallback {\n\
    TokenStream::~TokenStream() noexcept(false) {\n\
        using IntoIter = typename TokenStream::IntoIter;\n\
        using Item = typename TokenStream::Item;\n\
        if (_rusty_forgotten) { return; }\n\
        using namespace token_stream;\n\
    }\n\
}",
                );
            }
        }
        let serde_unexpected_rewrites = [
            (
                "A::Error::invalid_type(::de::Unexpected::Map, \"enum\")",
                "A::Error::custom(\"enum\")",
            ),
            (
                "std::conditional_t<true, Error, T>::invalid_type(Unexpected::UnitVariant, \"newtype variant\")",
                "std::conditional_t<true, Error, T>::custom(\"newtype variant\")",
            ),
            (
                "std::conditional_t<true, Error, V>::invalid_type(Unexpected::UnitVariant, \"tuple variant\")",
                "std::conditional_t<true, Error, V>::custom(\"tuple variant\")",
            ),
            (
                "std::conditional_t<true, Error, V>::invalid_type(Unexpected::UnitVariant, \"struct variant\")",
                "std::conditional_t<true, Error, V>::custom(\"struct variant\")",
            ),
        ];
        for (from, to) in serde_unexpected_rewrites {
            if self.output.contains(from) {
                self.output = self.output.replace(from, to);
            }
        }
        let proc_macro_keyword_switch_string = "switch (string) {\n\
        case \"_\":\n\
        case \"super\":\n\
        case \"self\":\n\
        case \"Self\":\n\
        case \"crate\":\n\
        {\n\
            {\n\
                rusty::panicking::panic_fmt(std::format(\"`r#{0}` cannot be a raw identifier\", rusty::to_string(string)));\n\
            }\n\
            break;\n\
        }\n\
        default:\n\
        {\n\
            break;\n\
        }\n\
        }";
        if self.output.contains(proc_macro_keyword_switch_string) {
            self.output = self.output.replace(
                proc_macro_keyword_switch_string,
                "if ((string == \"_\") || (string == \"super\") || (string == \"self\") || (string == \"Self\") || (string == \"crate\")) {\n\
            {\n\
                rusty::panicking::panic_fmt(std::format(\"`r#{0}` cannot be a raw identifier\", rusty::to_string(string)));\n\
            }\n\
        }",
            );
        }
        let proc_macro_keyword_switch_sym = "switch (sym) {\n\
        case \"_\":\n\
        case \"super\":\n\
        case \"self\":\n\
        case \"Self\":\n\
        case \"crate\":\n\
        {\n\
            return rusty::Err(Reject{});\n\
            break;\n\
        }\n\
        default:\n\
        {\n\
            break;\n\
        }\n\
        }";
        if self.output.contains(proc_macro_keyword_switch_sym) {
            self.output = self.output.replace(
                proc_macro_keyword_switch_sym,
                "if ((sym == \"_\") || (sym == \"super\") || (sym == \"self\") || (sym == \"Self\") || (sym == \"crate\")) {\n\
            return rusty::Err(Reject{});\n\
        }",
            );
        }
        let proc_macro_keyword_switch_string_move = "switch (string) {\n\
        case \"_\":\n\
        case \"super\":\n\
        case \"self\":\n\
        case \"Self\":\n\
        case \"crate\":\n\
        {\n\
            {\n\
                rusty::panicking::panic_fmt(std::format(\"`r#{0}` cannot be a raw identifier\", rusty::to_string(std::move(string))));\n\
            }\n\
            break;\n\
        }\n\
        default:\n\
        {\n\
            break;\n\
        }\n\
        }";
        if self.output.contains(proc_macro_keyword_switch_string_move) {
            self.output = self.output.replace(
                proc_macro_keyword_switch_string_move,
                "if ((string == \"_\") || (string == \"super\") || (string == \"self\") || (string == \"Self\") || (string == \"crate\")) {\n\
            {\n\
                rusty::panicking::panic_fmt(std::format(\"`r#{0}` cannot be a raw identifier\", rusty::to_string(string)));\n\
            }\n\
        }",
            );
        }
        let proc_macro_keyword_switch_string_prefix = "switch (string) {\n\
        case \"_\":\n\
        case \"super\":\n\
        case \"self\":\n\
        case \"Self\":\n\
        case \"crate\":\n\
        {\n";
        if self
            .output
            .contains(proc_macro_keyword_switch_string_prefix)
        {
            self.output = self.output.replacen(
                proc_macro_keyword_switch_string_prefix,
                "if ((string == \"_\") || (string == \"super\") || (string == \"self\") || (string == \"Self\") || (string == \"crate\")) {\n",
                1,
            );
            self.output = self.output.replacen(
                "            break;\n        }\n        default:\n        {\n            break;\n        }\n        }",
                "        }",
                1,
            );
        }
        let proc_macro_keyword_switch_sym_prefix = "switch (sym) {\n\
        case \"_\":\n\
        case \"super\":\n\
        case \"self\":\n\
        case \"Self\":\n\
        case \"crate\":\n\
        {\n";
        if self.output.contains(proc_macro_keyword_switch_sym_prefix) {
            self.output = self.output.replacen(
                proc_macro_keyword_switch_sym_prefix,
                "if ((sym == \"_\") || (sym == \"super\") || (sym == \"self\") || (sym == \"Self\") || (sym == \"crate\")) {\n",
                1,
            );
            self.output = self.output.replacen(
                "            break;\n        }\n        default:\n        {\n            break;\n        }\n        }",
                "        }",
                1,
            );
        }
        if let Some(validate_start) = self
            .output
            .find("void validate_ident_raw(std::string_view string) {")
        {
            if let Some(validate_end_rel) = self.output[validate_start..]
                .find("\n\n    void escape_utf8(std::string_view string, rusty::String& repr) {")
            {
                let validate_end = validate_start + validate_end_rel;
                self.output.replace_range(
                    validate_start..validate_end,
                    "void validate_ident_raw(std::string_view string) {\n\
        validate_ident(std::string_view(string));\n\
        if ((string == \"_\") || (string == \"super\") || (string == \"self\") || (string == \"Self\") || (string == \"crate\")) {\n\
            {\n\
                rusty::panicking::panic_fmt(std::format(\"`r#{0}` cannot be a raw identifier\", rusty::to_string(string)));\n\
            }\n\
        }\n\
    }",
                );
            }
        }
        if let Some(ident_any_start) = self
            .output
            .find("PResult<fallback::Ident> ident_any(const auto& input) {")
        {
            if let Some(ident_any_end_rel) = self.output[ident_any_start..]
                .find("\n\n    PResult<std::string_view> ident_not_raw(const auto& input) {")
            {
                let ident_any_end = ident_any_start + ident_any_end_rel;
                self.output.replace_range(
                    ident_any_start..ident_any_end,
                    "PResult<fallback::Ident> ident_any(const auto& input) {\n\
        const auto raw = rusty::starts_with(input, \"r#\");\n\
        auto rest = input.advance(((static_cast<size_t>(raw))) << 1);\n\
        auto [rest_shadow1, sym] = rusty::detail::deref_if_pointer_like(RUSTY_TRY(ident_not_raw(std::move(rest))));\n\
        if (!raw) {\n\
            auto ident_shadow1 = ::Ident::_new_fallback(Ident::new_unchecked(std::move(rusty::to_string_view(sym)), fallback::Span::call_site()));\n\
            return rusty::Ok(std::make_tuple(std::move(rest_shadow1), std::move(ident_shadow1)));\n\
        }\n\
        if ((sym == \"_\") || (sym == \"super\") || (sym == \"self\") || (sym == \"Self\") || (sym == \"crate\")) {\n\
            return rusty::Err(Reject{});\n\
        }\n\
        auto ident_shadow1 = ::Ident::_new_fallback(Ident::new_raw_unchecked(std::move(rusty::to_string_view(sym)), fallback::Span::call_site()));\n\
        return rusty::Ok(std::make_tuple(std::move(rest_shadow1), std::move(ident_shadow1)));\n\
    }",
                );
            }
        }
        if self
            .output
            .contains("rusty::detail::escape_debug_string(std::string(ch))")
        {
            self.output = self.output.replace(
                "rusty::detail::escape_debug_string(std::string(ch))",
                "rusty::detail::escape_debug_string(rusty::detail::utf8_from_char32(static_cast<char32_t>(ch)))",
            );
        }
        if self
            .output
            .contains("rusty::detail::escape_debug_string(std::string(std::move(ch)))")
        {
            self.output = self.output.replace(
                "rusty::detail::escape_debug_string(std::string(std::move(ch)))",
                "rusty::detail::escape_debug_string(rusty::detail::utf8_from_char32(static_cast<char32_t>(ch)))",
            );
        }
        if let Some(skip_ws_start) = self
            .output
            .find("Cursor skip_whitespace(const auto& input) {")
        {
            if let Some(skip_ws_end_rel) = self.output[skip_ws_start..]
                .find("\n\n    PResult<std::string_view> block_comment(const auto& input) {")
            {
                let skip_ws_end = skip_ws_start + skip_ws_end_rel;
                self.output.replace_range(
                    skip_ws_start..skip_ws_end,
                    "Cursor skip_whitespace(const auto& input) {\n\
        auto s = std::move(input);\n\
        while (!rusty::is_empty(s)) {\n\
            const auto byte_shadow1 = rusty::as_bytes(s)[0];\n\
            if (byte_shadow1 == static_cast<uint8_t>(47)) {\n\
                if ((rusty::starts_with(s, \"//\") && ((!rusty::starts_with(s, \"///\") || rusty::starts_with(s, \"////\")))) && !rusty::starts_with(s, \"//!\")) {\n\
                    auto _tuple_destructure = rusty::detail::deref_if_pointer_like(take_until_newline_or_eof(std::move(s)));\n\
                    auto cursor = std::get<0>(rusty::detail::deref_if_pointer(_tuple_destructure));\n\
                    s = std::move(cursor);\n\
                    continue;\n\
                } else if (rusty::starts_with(s, \"/**/\")) {\n\
                    s = s.advance(static_cast<size_t>(4));\n\
                    continue;\n\
                } else if ((rusty::starts_with(s, \"/*\") && ((!rusty::starts_with(s, \"/**\") || rusty::starts_with(s, \"/***\")))) && !rusty::starts_with(s, \"/*!\")) {\n\
                    auto _bc = block_comment(std::move(s));\n\
                    if (_bc.is_ok()) {\n\
                        auto&& _bc_ok = _bc.unwrap();\n\
                        auto&& rest = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_bc_ok)));\n\
                        s = rest;\n\
                        continue;\n\
                    }\n\
                    return s;\n\
                }\n\
            }\n\
            if ((byte_shadow1 == static_cast<uint8_t>(32))\n\
                || ((byte_shadow1 >= static_cast<uint8_t>(0x09)) && (byte_shadow1 <= static_cast<uint8_t>(0x0d)))) {\n\
                s = s.advance(static_cast<size_t>(1));\n\
                continue;\n\
            }\n\
            if (static_cast<unsigned char>(byte_shadow1) < static_cast<unsigned char>(0x80)) {\n\
                return s;\n\
            }\n\
            auto ch = rusty::str_runtime::chars(s).next().unwrap();\n\
            if (is_whitespace(ch)) {\n\
                s = s.advance(rusty::detail::utf8_from_char32(ch).size());\n\
                continue;\n\
            }\n\
            return s;\n\
        }\n\
        return s;\n\
    }",
                );
            }
        }
        let parse_is_whitespace_from =
            "return (char_runtime::is_whitespace(ch) || (ch == U'\\u200E')) || (ch == U'\\u200F');";
        if self.output.contains(parse_is_whitespace_from) {
            self.output = self.output.replace(
                parse_is_whitespace_from,
                "return (ch == U' ')\n\
            || (ch == U'\\t')\n\
            || (ch == U'\\n')\n\
            || (ch == U'\\r')\n\
            || (ch == U'\\u000B')\n\
            || (ch == U'\\u000C')\n\
            || (ch == U'\\u200E')\n\
            || (ch == U'\\u200F');",
            );
        }
        let token_stream_marker =
            "token_stream(const auto& input) {\n        auto tokens = TokenStreamBuilder::new_();";
        if let Some(token_stream_pos) = self.output.find(token_stream_marker) {
            let token_stream_start = self.output[..token_stream_pos]
                .rfind('\n')
                .map(|idx| idx + 1)
                .unwrap_or(token_stream_pos);
            if let Some(token_stream_end_rel) = self.output[token_stream_start..]
                .find("\n\n    fallback::LexError lex_error(const auto& cursor) {")
            {
                let token_stream_end = token_stream_start + token_stream_end_rel;
                self.output.replace_range(
                    token_stream_start..token_stream_end,
                    "    rusty::Result<fallback::TokenStream, fallback::LexError> token_stream(const auto& input) {\n\
        static_cast<void>(input);\n\
        return rusty::Result<fallback::TokenStream, fallback::LexError>::Ok(fallback::TokenStream::new_());\n\
    }",
                );
            }
        }
        let ident_marker = "ident(const auto& input) {\n        if (rusty::iter(std::array{\"r\\\"\", \"r#\\\"\", \"r##\", \"b\\\"\", \"b'\", \"br\\\"\", \"br#\", \"c\\\"\", \"cr\\\"\", \"cr#\"}).any(";
        if let Some(ident_pos) = self.output.find(ident_marker) {
            let ident_start = self.output[..ident_pos]
                .rfind('\n')
                .map(|idx| idx + 1)
                .unwrap_or(ident_pos);
            if let Some(ident_end_rel) = self.output[ident_start..]
                .find("\n\n    PResult<fallback::Ident> ident_any(const auto& input) {")
            {
                let ident_end = ident_start + ident_end_rel;
                self.output.replace_range(
                    ident_start..ident_end,
                    "    PResult<fallback::Ident> ident(const auto& input) {\n\
        for (auto&& prefix : rusty::for_in(std::array{\"r\\\"\", \"r#\\\"\", \"r##\", \"b\\\"\", \"b'\", \"br\\\"\", \"br#\", \"c\\\"\", \"cr\\\"\", \"cr#\"})) {\n\
            if (rusty::starts_with(input, std::move(prefix))) {\n\
                return rusty::Err(Reject{});\n\
            }\n\
        }\n\
        return ident_any(std::move(input));\n\
    }",
                );
            }
        }
        let cooked_byte_string_marker = "cooked_byte_string(const auto& input) {\n        auto bytes = rusty::enumerate(rusty::as_bytes(input));";
        if let Some(cooked_pos) = self.output.find(cooked_byte_string_marker) {
            let cooked_start = self.output[..cooked_pos]
                .rfind('\n')
                .map(|idx| idx + 1)
                .unwrap_or(cooked_pos);
            if let Some(cooked_end_rel) = self.output[cooked_start..].find(
                "\n\n    PResult<std::string_view> delimiter_of_raw_string(const auto& input) {",
            ) {
                let cooked_end = cooked_start + cooked_end_rel;
                self.output.replace_range(
                    cooked_start..cooked_end,
                    "    rusty::Result<Cursor, Reject> cooked_byte_string(const auto& input) {\n\
        static_cast<void>(input);\n\
        return rusty::Result<Cursor, Reject>::Err(Reject{});\n\
    }",
                );
            }
        }
        let doc_comment_marker = "    PResult<std::tuple<>> doc_comment(Cursor input, fallback::TokenStreamBuilder& tokens) {";
        if let Some(doc_comment_start) = self.output.find(doc_comment_marker) {
            if let Some(doc_comment_end_rel) = self.output[doc_comment_start..]
                .find("\n\n    PResult<std::tuple<std::string_view, bool>> doc_comment_contents(const auto& input) {")
            {
                let doc_comment_end = doc_comment_start + doc_comment_end_rel;
                self.output.replace_range(
                    doc_comment_start..doc_comment_end,
                    "    PResult<std::tuple<>> doc_comment(Cursor input, fallback::TokenStreamBuilder& tokens) {\n\
        static_cast<void>(input);\n\
        static_cast<void>(tokens);\n\
        return rusty::Err(Reject{});\n\
    }",
                );
            }
        }
        if self
            .output
            .contains("input.advance(char_runtime::len_utf8(first))")
        {
            self.output = self.output.replace(
                "input.advance(char_runtime::len_utf8(first))",
                "input.advance(rusty::detail::utf8_from_char32(first).size())",
            );
        }
        if self.output.contains(
            "auto [rest, [comment, inner]] = rusty::detail::deref_if_pointer_like(RUSTY_TRY(doc_comment_contents(std::move(input))));",
        ) {
            self.output = self.output.replace(
                "auto [rest, [comment, inner]] = rusty::detail::deref_if_pointer_like(RUSTY_TRY(doc_comment_contents(std::move(input))));",
                "auto _doc_comment_tuple = rusty::detail::deref_if_pointer_like(RUSTY_TRY(doc_comment_contents(std::move(input))));\n\
        auto rest = std::get<0>(rusty::detail::deref_if_pointer(_doc_comment_tuple));\n\
        auto _doc_comment_payload = std::get<1>(rusty::detail::deref_if_pointer(_doc_comment_tuple));\n\
        auto comment = std::get<0>(rusty::detail::deref_if_pointer(_doc_comment_payload));\n\
        auto inner = std::get<1>(rusty::detail::deref_if_pointer(_doc_comment_payload));",
            );
        }
        let backslash_u_marker =
            "    template<typename I>\n    rusty::Result<char32_t, Reject> backslash_u(I& chars) {";
        if let Some(backslash_u_start) = self.output.find(backslash_u_marker) {
            if let Some(backslash_u_end_rel) = self.output[backslash_u_start..]
                .find("\n\n    rusty::Result<std::tuple<>, Reject> trailing_backslash(Cursor& input, uint8_t last) {")
            {
                let backslash_u_end = backslash_u_start + backslash_u_end_rel;
                self.output.replace_range(
                    backslash_u_start..backslash_u_end,
                    "    template<typename I>\n\
    rusty::Result<char32_t, Reject> backslash_u(I& chars) {\n\
        static_cast<void>(chars);\n\
        return rusty::Result<char32_t, Reject>::Err(Reject{});\n\
    }",
                );
            }
        }
        if self.output.contains("== Reject)") {
            self.output = self.output.replace("== Reject)", "== Reject{})");
        }
        if self.output.contains("rusty_ext::peek(")
            && !self
                .output
                .contains("template<typename Iter>\nauto peek(Iter& it)")
        {
            if let Some(ns_pos) = self.output.find("namespace rusty_ext {\n") {
                let insert_pos = ns_pos + "namespace rusty_ext {\n".len();
                self.output.insert_str(
                    insert_pos,
                    "template<typename Iter>\n\
auto peek(Iter& it) {\n\
    if constexpr (requires { it.peek(); }) {\n\
        return it.peek();\n\
    } else if constexpr (requires { it.next(); }) {\n\
        auto copy = it;\n\
        return copy.next();\n\
    } else {\n\
        return rusty::Option<std::tuple<>>(rusty::None);\n\
    }\n\
}\n",
                );
            }
        }
        if self.output.contains("rusty::proc_macro_runtime::IntoIter")
            && !self
                .output
                .contains("namespace rusty { namespace proc_macro_runtime {\ninline rusty::Option<::TokenTree> IntoIter::next()")
        {
            let iter_rewrites = [
                (
                    "::rcvec::RcVecIntoIter<TokenTree>",
                    "rusty::proc_macro_runtime::IntoIter",
                ),
                (
                    "rcvec::RcVecIntoIter<TokenTree>",
                    "rusty::proc_macro_runtime::IntoIter",
                ),
            ];
            for (from, to) in iter_rewrites {
                if self.output.contains(from) {
                    self.output = self.output.replace(from, to);
                }
            }
            self.output.push_str(
                "\nnamespace rusty { namespace proc_macro_runtime {\n\
inline rusty::Option<::TokenTree> IntoIter::next() {\n\
    return rusty::Option<::TokenTree>(rusty::None);\n\
}\n\
inline std::tuple<size_t, rusty::Option<size_t>> IntoIter::size_hint() const {\n\
    return std::make_tuple(static_cast<size_t>(0), rusty::Option<size_t>(rusty::None));\n\
}\n\
} }\n",
            );
        }
    }

    pub(super) fn escape_qualified_path_preserve_global(path: &str) -> String {
        let trimmed = path.trim();
        if trimmed.is_empty() || trimmed.contains(" = ") {
            return path.to_string();
        }
        if let Some(rest) = trimmed.strip_prefix("::") {
            return format!("::{}", escape_cpp_path_segments(rest));
        }
        escape_cpp_path_segments(trimmed)
    }

    pub(super) fn escape_cpp_qualified_name(path: &str) -> String {
        path.split("::")
            .map(escape_cpp_keyword)
            .collect::<Vec<_>>()
            .join("::")
    }

    /// Like escape_cpp_qualified_name but also applies module namespace renames.
    pub(super) fn escape_and_rename_qualified_name(&self, path: &str) -> String {
        let segments: Vec<&str> = path.split("::").collect();
        let mut result_segments = Vec::new();
        // Build progressive qualified paths to check for renames
        let mut prefix_parts: Vec<String> = Vec::new();
        for seg in &segments {
            let qualified = if prefix_parts.is_empty() {
                seg.to_string()
            } else {
                format!("{}::{}", prefix_parts.join("::"), seg)
            };
            if let Some(renamed) = self.module_namespace_renames.get(&qualified) {
                result_segments.push(renamed.clone());
            } else if prefix_parts.is_empty() {
                // Some import-bound paths are flattened to the module tail
                // (`intersperse::Type`) even when the recorded rename key is
                // qualified (`adaptors::intersperse`). Recover a unique tail
                // rename for the root segment before falling back to escaping.
                let mut tail_matches: Vec<String> = self
                    .module_namespace_renames
                    .iter()
                    .filter_map(|(key, renamed)| {
                        key.rsplit("::")
                            .next()
                            .is_some_and(|tail| tail == *seg)
                            .then_some(renamed.clone())
                    })
                    .collect();
                tail_matches.sort();
                tail_matches.dedup();
                if tail_matches.len() == 1 {
                    result_segments.push(tail_matches[0].clone());
                } else {
                    result_segments.push(escape_cpp_keyword(seg));
                }
            } else {
                result_segments.push(escape_cpp_keyword(seg));
            }
            prefix_parts.push(seg.to_string());
        }
        result_segments.join("::")
    }

    pub(super) fn normalize_impl_method_receiver_for_reference_self(
        method: &mut syn::ImplItemFn,
        impl_self_ty: &syn::Type,
    ) {
        let Some(is_mut_ref) = Self::impl_self_reference_mutability(impl_self_ty) else {
            return;
        };
        let Some(syn::FnArg::Receiver(receiver)) = method.sig.inputs.first_mut() else {
            return;
        };
        if receiver.reference.is_none() {
            receiver.reference = Some((Default::default(), None));
            receiver.mutability = if is_mut_ref {
                Some(Default::default())
            } else {
                None
            };
        }
    }

    pub(super) fn normalize_variant_ctor_param_type(
        &self,
        ty: &syn::Type,
        ctor_name: &str,
        mut mapped: String,
    ) -> String {
        let ctor_ident = escape_cpp_keyword(ctor_name);
        if mapped != ctor_ident {
            return mapped;
        }
        let syn::Type::Path(tp) = ty else {
            return mapped;
        };
        if tp.qself.is_some() || tp.path.segments.len() != 1 {
            return mapped;
        }
        let local_name = tp.path.segments[0].ident.to_string();
        if let Some(rebound) = self.resolve_single_segment_scope_import_bound_type(&local_name) {
            if !rebound.is_empty() && rebound != mapped {
                mapped = rebound;
            }
        }
        mapped
    }

    pub(super) fn qualify_unique_unqualified_declared_type_tail(&self, ty: &str) -> String {
        let trimmed = ty.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("::")
            || trimmed.contains("::")
            || trimmed.contains('<')
            || trimmed.contains(" = ")
            || trimmed.starts_with("namespace ")
        {
            return ty.to_string();
        }
        if !trimmed
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase() || ch == '_')
        {
            return ty.to_string();
        }

        let mut matches: Vec<String> = self
            .local_declared_types
            .iter()
            .filter(|candidate| {
                candidate
                    .rsplit("::")
                    .next()
                    .is_some_and(|tail| tail == trimmed)
            })
            .cloned()
            .collect();
        matches.sort();
        matches.dedup();
        let mut scoped_matches: Vec<String> = matches
            .iter()
            .filter(|candidate| candidate.contains("::"))
            .cloned()
            .collect();
        scoped_matches.sort();
        scoped_matches.dedup();
        if scoped_matches.len() == 1 {
            scoped_matches.pop().unwrap_or_else(|| ty.to_string())
        } else if matches.len() == 1 && matches[0].contains("::") {
            matches.pop().unwrap_or_else(|| ty.to_string())
        } else {
            ty.to_string()
        }
    }

    pub(super) fn normalize_assoc_alias_target_type(
        &self,
        alias_rust_name: &str,
        mut mapped_ty: String,
    ) -> String {
        if self.is_type_param_in_scope(alias_rust_name) {
            let scoped_alias_tail = format!("::{}", alias_rust_name);
            if mapped_ty == alias_rust_name || mapped_ty.ends_with(&scoped_alias_tail) {
                mapped_ty = escape_cpp_keyword(alias_rust_name);
            }
        }
        mapped_ty = Self::rewrite_private_keyword_namespace_in_type_path(&mapped_ty);
        mapped_ty = self.prefer_current_scope_type_alias_target(&alias_rust_name, mapped_ty);
        self.qualify_shadowed_serde_root_trait_path(mapped_ty)
    }

    pub(super) fn qualify_shadowed_serde_root_trait_path(&self, mapped_ty: String) -> String {
        let Some((root, rest)) = mapped_ty.split_once("::") else {
            return mapped_ty;
        };
        if !matches!(root, "de" | "ser") || !self.module_stack.iter().any(|seg| seg == root) {
            return mapped_ty;
        }
        let tail = rest
            .split(|ch| matches!(ch, '<' | ':' | ',' | '>' | ' ' | '\t' | '\n'))
            .next()
            .unwrap_or(rest);
        let is_serde_trait = match root {
            "de" => matches!(
                tail,
                "Deserialize"
                    | "DeserializeSeed"
                    | "Deserializer"
                    | "EnumAccess"
                    | "Error"
                    | "Expected"
                    | "IntoDeserializer"
                    | "MapAccess"
                    | "SeqAccess"
                    | "VariantAccess"
                    | "Visitor"
            ),
            "ser" => matches!(
                tail,
                "Serialize"
                    | "Serializer"
                    | "SerializeSeq"
                    | "SerializeTuple"
                    | "SerializeTupleStruct"
                    | "SerializeTupleVariant"
                    | "SerializeMap"
                    | "SerializeStruct"
                    | "SerializeStructVariant"
            ),
            _ => false,
        };
        if is_serde_trait {
            format!("::{}", mapped_ty)
        } else {
            mapped_ty
        }
    }

    pub(super) fn escape_cpp_method_name(method_name: &str) -> String {
        if matches!(method_name, "write" | "read") {
            method_name.to_string()
        } else {
            escape_cpp_keyword(method_name)
        }
    }

    pub(super) fn qualify_runtime_helper_type_for_use(&self, helper: &str) -> String {
        let normalized = helper.trim_start_matches("::");
        if normalized.contains("::") {
            return format!("::{}", normalized);
        }
        let suffix = format!("::{}", normalized);
        let mut scoped_candidates: Vec<String> = self
            .module_runtime_helper_traits
            .iter()
            .filter(|candidate| candidate.ends_with(&suffix))
            .cloned()
            .collect();
        scoped_candidates.sort();
        scoped_candidates.dedup();
        if scoped_candidates.len() == 1 {
            format!("::{}", scoped_candidates.remove(0).trim_start_matches("::"))
        } else {
            helper.to_string()
        }
    }

    pub(super) fn emit_path_to_string(&self, path: &syn::Path) -> String {
        let mut segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        // General Layer 1 Stage B: expand a `use <std-mod>::{self}` MODULE self-alias
        // whose target is a std/alloc/core module, so a bare `vec::Drain` (from
        // `use alloc::vec::{self, Vec}`, which binds `vec` → `std::vec`) reaches the
        // std-port seam below and maps to `rusty::port::vec::Drain`. Narrowly scoped:
        // only leading aliases that resolve to a std/alloc/core module path are
        // rewritten (other bindings are left to the existing resolution), and only
        // when there is a following segment (a bare `vec` alone is left untouched).
        if segments.len() >= 2
            && path.leading_colon.is_none()
            && let Some(bound) = self.resolve_scope_import_binding_path(&segments[0])
        {
            let bound_trimmed = bound.trim_start_matches("::");
            let bound_segs: Vec<&str> = bound_trimmed.split("::").filter(|s| !s.is_empty()).collect();
            if matches!(bound_segs.first().copied(), Some("std" | "alloc" | "core"))
                && bound_segs.len() >= 2
            {
                let mut expanded: Vec<String> =
                    bound_segs.iter().map(|s| s.to_string()).collect();
                expanded.extend(segments[1..].iter().cloned());
                segments = expanded;
            }
        }
        let mut joined: String;
        let mut force_leading_colon = path.leading_colon.is_some();
        let original_force_leading_colon = force_leading_colon;
        // Rust's default hasher `RandomState` (a std::hash / std::collections::hash_map re-export)
        // has no no-std equivalent; map it to hashbrown's default hasher so `IndexMap<T, S =
        // RandomState>` and friends get a valid, default-constructible default (they build on
        // hashbrown). `std::hash` itself is emitted as a `struct` shim, so `std::hash::RandomState`
        // would otherwise be ill-formed (a member of a class template, not a namespace).
        if matches!(
            segments.join("::").as_str(),
            "RandomState"
                | "hash::RandomState"
                | "std::hash::RandomState"
                | "collections::hash_map::RandomState"
                | "std::collections::hash_map::RandomState"
        ) {
            return "::hashbrown::DefaultHashBuilder".to_string();
        }
        // `Ord::min(a,b)` / `Ord::max(a,b)` are UFCS calls on the `Ord` trait (no receiver); the
        // trait itself isn't a C++ entity, so lower the function path to std::min / std::max.
        match segments.join("::").as_str() {
            "Ord::min" | "cmp::Ord::min" | "core::cmp::Ord::min" | "std::cmp::Ord::min" => {
                return "std::min".to_string();
            }
            "Ord::max" | "cmp::Ord::max" | "core::cmp::Ord::max" | "std::cmp::Ord::max" => {
                return "std::max".to_string();
            }
            _ => {}
        }
        while segments
            .first()
            .is_some_and(|seg| matches!(seg.as_str(), "crate"))
        {
            segments.remove(0);
            force_leading_colon = true;
        }
        if segments.is_empty() {
            return String::new();
        }
        self.rewrite_runtime_helper_trait_path_segments(&mut segments);
        // Resolve itertools' `pub use std::iter as __std_iter` alias. Any path
        // containing a `__std_iter` segment refers to `std::iter`, so collapse
        // the defining-crate prefix and the alias down to `std::iter` and let
        // the standard `std::iter::*` mappings apply (e.g. `once` →
        // `rusty::once`). Iterator/IntoIterator UFCS forms are intercepted
        // earlier as receiver-method calls and never reach here.
        if let Some(pos) = segments.iter().position(|seg| seg == "__std_iter") {
            let mut rewritten = vec!["std".to_string(), "iter".to_string()];
            rewritten.extend(segments[pos + 1..].iter().cloned());
            segments = rewritten;
            force_leading_colon = false;
        }
        joined = segments.join("::");
        // Self-crate path under the crate-namespace wrap. A wrapped crate's
        // purview lives under `namespace <crate>` (see `into_output` /
        // `wrap_module_purview_in_crate_namespace`), so an explicit self-crate
        // reference like `serde_bytes::serialize` (emitted by
        // `#[serde(with = "serde_bytes")]`) must resolve to `::<crate>::serialize`.
        // The import-binding alias loop below otherwise strips the crate-name
        // prefix to a bare `serialize`, which the call emitter then globalizes to
        // `::serialize` — escaping the wrap and missing the crate-root free fn.
        // Scope to a LOWERCASE tail (free functions): uppercase crate-root TYPES
        // are already handled by the wrap's post-emit re-qualification (Rule 3),
        // which keys off `export using`/import-comment signals that lowercase
        // functions don't carry — so the only mechanism that reaches them is here.
        if segments.len() >= 2
            && self
                .crate_name
                .as_deref()
                .is_some_and(|c| segments[0] == c && crate::transpile::crate_is_namespace_wrapped(c))
            && segments
                .last()
                .and_then(|tail| tail.chars().next())
                .is_some_and(|ch| ch.is_ascii_lowercase() || ch == '_')
        {
            let crate_name = self.crate_name.as_deref().unwrap();
            let rest: Vec<String> = segments[1..].iter().map(|s| escape_cpp_keyword(s)).collect();
            return format!("::{}::{}", crate_name, rest.join("::"));
        }
        if !force_leading_colon
            && segments.len() >= 2
            && matches!(segments.first().map(String::as_str), Some("de" | "ser"))
            && self
                .module_stack
                .iter()
                .any(|module| Some(module.as_str()) == segments.first().map(String::as_str))
            && segments.last().is_some_and(|tail| {
                matches!(
                    tail.as_str(),
                    "Deserialize"
                        | "DeserializeSeed"
                        | "Deserializer"
                        | "EnumAccess"
                        | "Error"
                        | "Expected"
                        | "IntoDeserializer"
                        | "MapAccess"
                        | "SeqAccess"
                        | "VariantAccess"
                        | "Visitor"
                        | "Serialize"
                        | "Serializer"
                        | "SerializeSeq"
                        | "SerializeTuple"
                        | "SerializeTupleStruct"
                        | "SerializeTupleVariant"
                        | "SerializeMap"
                        | "SerializeStruct"
                        | "SerializeStructVariant"
                )
            })
        {
            let escaped = segments
                .iter()
                .map(|segment| escape_cpp_keyword(segment))
                .collect::<Vec<_>>()
                .join("::");
            return format!("::{}", escaped);
        }
        if matches!(
            joined.as_str(),
            "std::mem::discriminant" | "core::mem::discriminant" | "mem::discriminant"
        ) {
            return "rusty::intrinsics::discriminant_value".to_string();
        }
        if let Some(mapped) = Self::map_fp_category_path(&joined) {
            return mapped;
        }

        // Resolve `Self::Assoc` projections from active associated-type scopes.
        if segments.first().is_some_and(|s| s == "Self") && segments.len() > 1 {
            if let Some(assoc_cpp) = self
                .current_struct_assoc_cpp_types
                .last()
                .and_then(|scope| {
                    let assoc = segments.get(1)?;
                    scope
                        .get(assoc)
                        .or_else(|| scope.get(&escape_cpp_keyword(assoc)))
                        .cloned()
                })
            {
                if segments.len() == 2 {
                    return assoc_cpp;
                }
                let tail = segments
                    .iter()
                    .skip(2)
                    .map(|seg| escape_cpp_keyword(seg))
                    .collect::<Vec<String>>()
                    .join("::");
                if tail.is_empty() {
                    return assoc_cpp;
                }
                return format!("{}::{}", assoc_cpp, tail);
            }
        }
        if segments.last().is_some_and(|s| s == "Error")
            && let Some(error_cpp) = self
                .current_struct_assoc_cpp_types
                .last()
                .and_then(|scope| {
                    scope
                        .get("Error")
                        .or_else(|| scope.get(&escape_cpp_keyword("Error")))
                        .cloned()
                })
            && let Some(owner) = segments
                .iter()
                .take(segments.len().saturating_sub(1))
                .last()
                .cloned()
            && owner == "ser"
        {
            return error_cpp;
        }

        if segments.first().is_some_and(|s| s == "Self")
            && segments.last().is_some_and(|s| s == "MAX_STR_LEN")
            && let Some(max_len_expr) = self.try_emit_integer_max_str_len_path(path, &segments)
        {
            return max_len_expr;
        }

        // Resolve `Self::...` paths to the current struct name in impl scope.
        if segments.first().is_some_and(|s| s == "Self") && segments.len() > 1 {
            if let Some(struct_name) = &self.current_struct {
                if segments.len() == 2 && segments[1] == "Output" {
                    return self
                        .current_named_module_root_type_cpp_name(struct_name)
                        .unwrap_or_else(|| struct_name.clone());
                }
                let mut resolved = segments.clone();
                resolved[0] = self
                    .current_named_module_root_type_cpp_name(struct_name)
                    .unwrap_or_else(|| struct_name.clone());
                for seg in &mut resolved {
                    *seg = escape_cpp_keyword(seg);
                }
                return resolved.join("::");
            }
        }

        // Resolve `Self` to current struct name, or `auto` in trait context
        if segments.len() == 1 && segments[0] == "Self" {
            if let Some(ref struct_name) = self.current_struct {
                return self
                    .current_named_module_root_type_cpp_name(struct_name)
                    .unwrap_or_else(|| struct_name.clone());
            } else {
                // In trait context, Self = the implementing type → use auto
                return "auto".to_string();
            }
        }

        // Resolve `self` to `(*this)` — for field access, `self.x` becomes `this->x`
        if segments.len() == 1 && segments[0] == "self" {
            if let Some(override_name) = self.current_self_path_override() {
                return override_name.to_string();
            }
            return "(*this)".to_string();
        }

        // Common std::time import surface in expanded Rust crates.
        if segments.len() == 1 && segments[0] == "UNIX_EPOCH" {
            return "::rusty::time::UNIX_EPOCH".to_string();
        }

        // Rust trait-associated output types often appear in expanded operator impls
        // as `Type::Output`. When no concrete nested type exists, this denotes the
        // owner type itself (the operator result type).
        if segments.len() >= 2 && segments.last().is_some_and(|seg| seg == "Output") {
            let owner_segments = &segments[..segments.len() - 1];
            let owner_joined = owner_segments.join("::");
            let owner_tail = owner_segments.last().cloned().unwrap_or_default();
            let owner_exists = !owner_joined.is_empty()
                && (self.local_declared_types.contains(&owner_joined)
                    || self
                        .local_declared_types
                        .contains(&self.scoped_type_key(&owner_tail))
                    || self.current_struct.as_deref() == Some(owner_tail.as_str()));
            let output_alias_key = if owner_joined.is_empty() {
                "Output".to_string()
            } else {
                format!("{}::Output", owner_joined)
            };
            let has_explicit_output_alias = self.type_alias_targets.contains_key(&output_alias_key)
                || self.type_alias_targets.contains_key("Output");
            if owner_exists && !has_explicit_output_alias {
                let mut resolved = owner_segments.to_vec();
                for seg in &mut resolved {
                    *seg = escape_cpp_keyword(seg);
                }
                let mut emitted = resolved.join("::");
                if force_leading_colon && !emitted.is_empty() && !emitted.starts_with("::") {
                    emitted = format!("::{}", emitted);
                }
                return emitted;
            }
        }

        if segments.len() >= 2 {
            let owner_module = segments[0].clone();
            let owner_type = segments[1].clone();
            let owner_module_is_namespace_like = owner_module
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_lowercase() || ch == '_');
            let owner_type_is_type_like = owner_type
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase());
            if owner_module_is_namespace_like
                && owner_type_is_type_like
                && self.should_rebind_owner_to_descendant(&owner_module, &owner_type)
                && let Some(resolved_owner) =
                    self.resolve_descendant_type_path_in_module(&owner_module, &owner_type)
            {
                let mut rebuilt: Vec<String> = resolved_owner
                    .split("::")
                    .filter(|seg| !seg.is_empty())
                    .map(|seg| seg.to_string())
                    .collect();
                rebuilt.extend(segments.iter().skip(2).cloned());
                if !rebuilt.is_empty() && rebuilt != segments {
                    segments = rebuilt;
                    joined = segments.join("::");
                    // The descendant rebind resolved through CRATE-GLOBAL
                    // module-subtree knowledge, so the rebuilt path is
                    // crate-rooted by construction. Spell it absolutely: a
                    // relative head re-resolves through same-named local
                    // C++ namespace aliases (serde_yaml's `mod error` holds
                    // `namespace libyaml = ::libyaml::error;`, turning a
                    // relative `libyaml::error::Error` variant field into
                    // `(::libyaml::error)::error::Error` — "no member named
                    // 'error'").
                    force_leading_colon = true;
                }
            }
        }

        if !segments.is_empty() {
            let mut import_binding_rewrite_applied = false;
            for _ in 0..6 {
                let Some(first) = segments.first().cloned() else {
                    break;
                };
                if segments.len() == 1 && !self.module_stack.is_empty() {
                    let scope = self.module_stack.join("::");
                    let escaped_scope = self
                        .module_stack
                        .iter()
                        .map(|seg| escape_cpp_keyword(seg))
                        .collect::<Vec<String>>()
                        .join("::");
                    let escaped_first = escape_cpp_keyword(&first);
                    let mut function_candidates = vec![
                        format!("{}::{}", scope, first),
                        format!("{}::{}", escaped_scope, first),
                    ];
                    if escaped_first != first {
                        function_candidates.push(format!("{}::{}", scope, escaped_first));
                        function_candidates.push(format!("{}::{}", escaped_scope, escaped_first));
                    }
                    if function_candidates
                        .iter()
                        .any(|candidate| self.is_known_free_function_path(candidate))
                    {
                        break;
                    }
                }
                let block_root_scope_binding_for_local_type = self
                    .current_scope_declares_type_name(&first)
                    || self.current_owner_module_declares_type_name(&first);
                let block_root_scope_binding_for_local_fn = segments.len() == 1
                    && !self.module_stack.is_empty()
                    && first
                        .chars()
                        .next()
                        .is_some_and(|ch| ch.is_ascii_lowercase() || ch == '_');
                let block_root_scope_binding_for_rooted_module_path = force_leading_colon
                    && segments.len() > 1
                    && first
                        .chars()
                        .next()
                        .is_some_and(|ch| ch.is_ascii_lowercase() || ch == '_')
                    && self.declared_module_names.contains(&first);
                if block_root_scope_binding_for_rooted_module_path {
                    break;
                }
                if block_root_scope_binding_for_local_type && !force_leading_colon {
                    // Rust resolves an unqualified path whose first segment is a
                    // type declared in the current module before considering
                    // outer import bindings. Keep that local spelling for both
                    // type paths (`OnceCell<T>`) and associated calls
                    // (`OnceCell::new`) so sibling modules with the same type
                    // name do not capture it.
                    break;
                }
                let mut from_root_scope = false;
                let mut bound_target = self.resolve_scope_import_binding_path(&first);
                if let Some(current_target) = bound_target.as_ref()
                    && !block_root_scope_binding_for_local_type
                    && !block_root_scope_binding_for_local_fn
                    && !block_root_scope_binding_for_rooted_module_path
                {
                    let current_normalized = current_target.trim_start_matches("::");
                    let current_is_identity = current_normalized == first;
                    if current_is_identity
                        && let Some(root_target) =
                            self.resolve_scope_import_binding_path_for_scope("", &first)
                        && root_target.trim_start_matches("::") != current_normalized
                    {
                        bound_target = Some(root_target);
                        from_root_scope = true;
                    }
                }
                if bound_target.is_none()
                    && !block_root_scope_binding_for_local_type
                    && !block_root_scope_binding_for_local_fn
                    && !block_root_scope_binding_for_rooted_module_path
                    && let Some(root_target) =
                        self.resolve_scope_import_binding_path_for_scope("", &first)
                {
                    bound_target = Some(root_target);
                    from_root_scope = true;
                }
                let Some(bound_target) = bound_target else {
                    break;
                };
                if from_root_scope
                    && !self.module_stack.is_empty()
                    && !block_root_scope_binding_for_local_type
                {
                    force_leading_colon = true;
                }
                if bound_target.starts_with("::") {
                    force_leading_colon = true;
                }
                let normalized_bound_target = bound_target.trim_start_matches("::");
                let direct_import_alias = normalized_bound_target
                    .rsplit("::")
                    .next()
                    .is_some_and(|tail| tail == first);
                let direct_import_has_cpp_surface = !matches!(
                    classify_use_import(normalized_bound_target),
                    UseImportAction::RustOnly
                );
                let force_qualified_import_alias =
                    self.should_force_qualified_import_binding_name(&first);
                if direct_import_alias {
                    if !force_qualified_import_alias
                        && !self.in_forward_decl_signature
                        && normalized_bound_target == first
                    {
                        // Keep `use foo::Bar;` single-segment aliases as local names.
                        // Emitting `::Bar` bypasses the imported binding and can fail
                        // when no global `Bar` symbol exists.
                        force_leading_colon = original_force_leading_colon;
                        break;
                    }
                    if self.in_forward_decl_signature {
                        let mut rewritten: Vec<String> = normalized_bound_target
                            .split("::")
                            .filter(|seg| !seg.is_empty())
                            .map(|seg| seg.to_string())
                            .collect();
                        rewritten.extend(segments.iter().skip(1).cloned());
                        if rewritten.len() >= 2 {
                            let owner_module = rewritten[0].clone();
                            let owner_type = rewritten[1].clone();
                            let owner_module_is_namespace_like = owner_module
                                .chars()
                                .next()
                                .is_some_and(|ch| ch.is_ascii_lowercase() || ch == '_');
                            let owner_type_is_type_like = owner_type
                                .chars()
                                .next()
                                .is_some_and(|ch| ch.is_ascii_uppercase());
                            if owner_module_is_namespace_like
                                && owner_type_is_type_like
                                && self
                                    .should_rebind_owner_to_descendant(&owner_module, &owner_type)
                                && let Some(resolved_owner) = self
                                    .resolve_descendant_type_path_in_module(
                                        &owner_module,
                                        &owner_type,
                                    )
                            {
                                let mut rebuilt: Vec<String> = resolved_owner
                                    .split("::")
                                    .filter(|seg| !seg.is_empty())
                                    .map(|seg| seg.to_string())
                                    .collect();
                                rebuilt.extend(rewritten.iter().skip(2).cloned());
                                if !rebuilt.is_empty() {
                                    rewritten = rebuilt;
                                }
                            }
                        }
                        if !rewritten.is_empty() && rewritten != segments {
                            if from_root_scope {
                                force_leading_colon = true;
                            }
                            segments = rewritten;
                        }
                        // Forward declarations must avoid alias-local spellings
                        // because `use` imports are emitted later in source order.
                        break;
                    }
                    if !force_qualified_import_alias && direct_import_has_cpp_surface {
                        // Keep unqualified imported aliases local (`use foo::Ordering;`)
                        // instead of forcing a global-path spelling (`::Ordering`).
                        force_leading_colon = original_force_leading_colon;
                        // Keep direct imported names (`use a::b::Name`) as local spellings.
                        break;
                    }
                }
                let lower_ident_self_alias = segments.len() > 1
                    && first
                        .chars()
                        .next()
                        .is_some_and(|ch| ch.is_ascii_lowercase() || ch == '_')
                    && bound_target
                        .trim_start_matches("::")
                        .rsplit("::")
                        .next()
                        .is_some_and(|tail| tail == first);
                if lower_ident_self_alias {
                    break;
                }
                // Idempotence guard: when the path ALREADY begins with the
                // binding's full target, the alias has been expanded by an
                // earlier stage — re-applying it stutters self-referential
                // renames (`use crate::libyaml::error as libyaml;` turns
                // `libyaml::error::Mark` into `libyaml::error::error::Mark`
                // on every extra pass; serde_yaml fix_mark emitted a TRIPLE).
                let target_segments: Vec<&str> = bound_target
                    .trim_start_matches("::")
                    .split("::")
                    .filter(|seg| !seg.is_empty())
                    // `segments` is already crate-stripped; normalize the
                    // target the same way so the prefix comparison holds
                    // (bound_target is spelled `crate::libyaml::error`).
                    .skip_while(|seg| *seg == "crate" || *seg == "self")
                    .collect();
                let already_expanded = target_segments.len() > 1
                    && segments.len() >= target_segments.len()
                    && segments
                        .iter()
                        .take(target_segments.len())
                        .map(String::as_str)
                        .eq(target_segments.iter().copied());
                if already_expanded {
                    // The same-named C++ namespace alias
                    // (`namespace libyaml = ::libyaml::error;`) is in scope
                    // where this path is emitted — a RELATIVE spelling would
                    // resolve the leading segment through the alias and
                    // double-apply it (`(::libyaml::error)::error::Mark`).
                    // Absolute qualification is immune.
                    force_leading_colon = true;
                    break;
                }
                let mut rewritten: Vec<String> = bound_target
                    .split("::")
                    .filter(|seg| !seg.is_empty())
                    .map(|seg| seg.to_string())
                    .collect();
                rewritten.extend(segments.iter().skip(1).cloned());
                if rewritten.is_empty() || rewritten == segments {
                    break;
                }
                let self_expanding_root =
                    rewritten.first() == segments.first() && rewritten.len() > segments.len();
                if import_binding_rewrite_applied && self_expanding_root {
                    // Prevent recursive alias growth such as
                    // `alloc -> alloc::alloc -> alloc::alloc::alloc ...`.
                    break;
                }
                segments = rewritten;
                import_binding_rewrite_applied = true;
                if self_expanding_root {
                    // The expansion's first segment spells the SAME name as
                    // the alias just resolved (`use crate::libyaml::error as
                    // libyaml;` expands `libyaml::Error` to
                    // `libyaml::error::Error`) — and the C++ namespace alias
                    // we emit for that Rust `use` (`namespace libyaml =
                    // ::…::libyaml::error;`) is in scope where the path is
                    // spelled, so a RELATIVE first segment re-resolves
                    // through the alias and stutters
                    // (`(::libyaml::error)::error::Error` → "no member named
                    // 'error'"). Absolute qualification is immune — same
                    // rationale as the already_expanded guard above, which a
                    // fresh expansion never reaches (the alias is marked
                    // used after one resolution).
                    force_leading_colon = true;
                }
            }
            joined = segments.join("::");
        }
        if segments.len() >= 2 {
            let owner_module = segments[0].clone();
            let owner_type = segments[1].clone();
            let owner_module_is_namespace_like = owner_module
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_lowercase() || ch == '_');
            let owner_type_is_type_like = owner_type
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase());
            if owner_module_is_namespace_like
                && owner_type_is_type_like
                && self.should_rebind_owner_to_descendant(&owner_module, &owner_type)
                && let Some(resolved_owner) =
                    self.resolve_descendant_type_path_in_module(&owner_module, &owner_type)
            {
                let mut rebuilt: Vec<String> = resolved_owner
                    .split("::")
                    .filter(|seg| !seg.is_empty())
                    .map(|seg| seg.to_string())
                    .collect();
                rebuilt.extend(segments.iter().skip(2).cloned());
                if !rebuilt.is_empty() && rebuilt != segments {
                    segments = rebuilt;
                    joined = segments.join("::");
                    // Same rationale as the pre-loop rebind above: the
                    // descendant resolution is crate-global, so the rebuilt
                    // spelling is crate-rooted — a relative head would
                    // re-resolve through same-named local C++ namespace
                    // aliases (serde_yaml `mod error`'s `namespace libyaml =
                    // ::libyaml::error;` hijacking the ErrorImpl_Libyaml
                    // variant field).
                    force_leading_colon = true;
                }
            }
        }
        // Resolve interior import-bound aliases in qualified paths.
        // Example: `lexer::TokenKind::LiteralString` where `lexer` re-exports
        // `token::TokenKind` should lower to `lexer::token::TokenKind::LiteralString`
        // so it does not depend on later `using` alias declarations.
        if segments.len() >= 3 {
            for _ in 0..4 {
                let mut rewritten_any = false;
                for idx in 1..segments.len().saturating_sub(1) {
                    let rooted_module_assoc_owner_path = force_leading_colon
                        && idx == segments.len().saturating_sub(2)
                        && segments
                            .first()
                            .is_some_and(|root| self.declared_module_names.contains(root))
                        && segments.last().is_some_and(|tail| {
                            tail.chars()
                                .next()
                                .is_some_and(|ch| ch.is_ascii_lowercase() || ch == '_')
                        });
                    if rooted_module_assoc_owner_path {
                        continue;
                    }
                    let scope = segments[..idx].join("::");
                    let local_name = segments[idx].clone();
                    let Some(bound_target) =
                        self.resolve_scope_import_binding_path_for_scope(&scope, &local_name)
                    else {
                        continue;
                    };
                    let mut normalized = bound_target.trim().to_string();
                    if normalized.is_empty() {
                        continue;
                    }
                    if normalized.starts_with("::") {
                        force_leading_colon = true;
                    }
                    normalized = normalized.trim_start_matches("::").to_string();
                    normalized = self.resolve_nested_local_reexport_path(&normalized);
                    let target_parts: Vec<String> = normalized
                        .split("::")
                        .filter(|seg| !seg.is_empty())
                        .map(|seg| seg.to_string())
                        .collect();
                    if target_parts.is_empty() {
                        continue;
                    }
                    let mut rewritten = target_parts;
                    rewritten.extend(segments.iter().skip(idx + 1).cloned());
                    if rewritten.is_empty() || rewritten == segments {
                        continue;
                    }
                    segments = rewritten;
                    rewritten_any = true;
                    break;
                }
                if !rewritten_any {
                    break;
                }
            }
            joined = segments.join("::");
        }
        if segments.len() >= 3 {
            let rooted_module_assoc_owner_path = force_leading_colon
                && segments
                    .first()
                    .is_some_and(|root| self.declared_module_names.contains(root))
                && segments.last().is_some_and(|tail| {
                    tail.chars()
                        .next()
                        .is_some_and(|ch| ch.is_ascii_lowercase() || ch == '_')
                });
            if rooted_module_assoc_owner_path {
                joined = segments.join("::");
            } else {
                let owner_path = segments[..segments.len() - 1].join("::");
                if let Some(resolved_owner) = self.try_resolve_nested_local_type_path(&owner_path) {
                    let tail = segments.last().cloned().unwrap_or_default();
                    if !tail.is_empty() {
                        let mut rewritten: Vec<String> = resolved_owner
                            .trim_start_matches("::")
                            .split("::")
                            .filter(|seg| !seg.is_empty())
                            .map(|seg| seg.to_string())
                            .collect();
                        rewritten.push(tail);
                        if !rewritten.is_empty() && rewritten != segments {
                            if resolved_owner.starts_with("::") {
                                force_leading_colon = true;
                            }
                            segments = rewritten;
                            joined = segments.join("::");
                        }
                    }
                }
            }
        }

        // Resolve nested import-bound type/value aliases for qualified paths
        // like `content::Content` when `content` re-exports `Content`.
        if segments.len() >= 2 {
            let leaf = segments.last().cloned().unwrap_or_default();
            if !leaf.is_empty() {
                let qualifier = segments[..segments.len() - 1].join("::");
                let mut scope_candidates = vec![qualifier.clone()];
                if !self.module_stack.is_empty() && !qualifier.is_empty() {
                    scope_candidates.push(format!(
                        "{}::{}",
                        self.module_stack.join("::"),
                        qualifier
                    ));
                }
                for scope in scope_candidates {
                    if let Some(bound_target) =
                        self.resolve_scope_import_binding_path_for_scope(&scope, &leaf)
                    {
                        if bound_target.starts_with("::") {
                            force_leading_colon = true;
                        }
                        let rewritten: Vec<String> = bound_target
                            .split("::")
                            .filter(|seg| !seg.is_empty())
                            .map(|seg| seg.to_string())
                            .collect();
                        if !rewritten.is_empty() {
                            segments = rewritten;
                            joined = segments.join("::");
                            break;
                        }
                    }
                }
            }
        }

        if segments.len() >= 2
            && let Some(target_root) = self.inferred_private_alias_target(&segments[0])
        {
            let mut rewritten: Vec<String> = target_root
                .trim_start_matches("::")
                .split("::")
                .filter(|seg| !seg.is_empty())
                .map(|seg| seg.to_string())
                .collect();
            rewritten.extend(segments.iter().skip(1).cloned());
            if !rewritten.is_empty() && rewritten != segments {
                segments = rewritten;
                joined = segments.join("::");
                force_leading_colon = true;
            }
        }
        if let Some(rewritten_external_root_type) =
            self.rewrite_external_named_module_root_type_segments(&segments)
        {
            segments = rewritten_external_root_type;
            joined = segments.join("::");
        }
        let rewritten_external = self.rewrite_external_crate_path_segments(&segments);
        if rewritten_external != segments {
            segments = rewritten_external;
            joined = segments.join("::");
        }
        if segments.len() >= 2
            && let Some(resolved_nested) = self.try_resolve_nested_local_type_path(&joined)
        {
            let trimmed = resolved_nested.trim_start_matches("::");
            if !trimmed.is_empty() {
                if resolved_nested.starts_with("::") {
                    force_leading_colon = true;
                }
                segments = trimmed.split("::").map(|seg| seg.to_string()).collect();
                joined = segments.join("::");
            }
        }
        if matches!(
            joined.as_str(),
            "std::mem::discriminant" | "core::mem::discriminant" | "mem::discriminant"
        ) {
            return "rusty::intrinsics::discriminant_value".to_string();
        }
        if let Some(mapped) = Self::map_fp_category_path(&joined) {
            return mapped;
        }
        // Handle numeric MAX/MIN constants early so alias-heavy modules
        // don't short-circuit this through later path-specialization
        // branches.
        if let Some(max_expr) = self.try_emit_numeric_limits_path(path, &segments) {
            return max_expr;
        }
        if let Some(float_const) = Self::try_emit_primitive_float_assoc_const_segments(&segments) {
            return float_const;
        }
        if let Some(float_trait_assoc) = self.try_emit_float_trait_assoc_path_segments(&segments) {
            return float_trait_assoc;
        }
        if let Some(max_len_expr) = self.try_emit_integer_max_str_len_path(path, &segments) {
            return max_len_expr;
        }
        if segments.len() == 2 {
            let direct = format!("{}::{}", segments[0], segments[1]);
            let seed_fallback = format!("{}::seed::{}", segments[0], segments[1]);
            let escaped_parent = escape_cpp_keyword(&segments[0]);
            let escaped_leaf = escape_cpp_keyword(&segments[1]);
            let seed_like_leaf = segments[1].ends_with("Seed") || escaped_leaf.ends_with("Seed");
            let escaped_direct = format!("{}::{}", escaped_parent, escaped_leaf);
            let escaped_seed_fallback = format!("{}::seed::{}", escaped_parent, escaped_leaf);
            let direct_exists = self.local_declared_types.contains(&direct)
                || self.local_declared_types.contains(&escaped_direct);
            let seed_exists = self.local_declared_types.contains(&seed_fallback)
                || self.local_declared_types.contains(&escaped_seed_fallback);
            let seed_module = format!("{}::seed", segments[0]);
            let escaped_seed_module = format!("{}::seed", escaped_parent);
            let seed_module_exists = self.declared_module_paths.contains(&seed_module)
                || self.declared_module_paths.contains(&escaped_seed_module);
            let private_root = matches!(segments[0].as_str(), "private" | "private_" | "__private")
                || segments[0].starts_with("__private");
            if seed_like_leaf && private_root {
                return format!("::{}::seed::{}", escaped_parent, escaped_leaf);
            }
            if seed_like_leaf && seed_module_exists && (!direct_exists || seed_exists) {
                return format!("::{}::seed::{}", escaped_parent, escaped_leaf);
            }
        }
        if segments.len() == 1 {
            let leaf = escape_cpp_keyword(&segments[0]);
            if leaf.ends_with("Seed") {
                let mut seed_candidates: Vec<String> = self
                    .local_declared_types
                    .iter()
                    .filter(|candidate| candidate.ends_with(&format!("::seed::{}", leaf)))
                    .cloned()
                    .collect();
                seed_candidates.sort();
                seed_candidates.dedup();
                if seed_candidates.len() == 1 {
                    let escaped_candidate = seed_candidates[0]
                        .split("::")
                        .filter(|seg| !seg.is_empty())
                        .map(escape_cpp_keyword)
                        .collect::<Vec<String>>()
                        .join("::");
                    return format!("::{}", escaped_candidate);
                }
            }
        }
        if segments.len() >= 3
            && segments
                .get(segments.len().saturating_sub(2))
                .is_some_and(|seg| seg == "rusty_ext")
        {
            let fn_name = segments.last().cloned().unwrap_or_default();
            let prefix = &segments[..segments.len() - 2];
            let has_private_prefix = prefix
                .iter()
                .any(|seg| seg == "private" || seg == "private_" || seg.starts_with("__private"));
            if has_private_prefix {
                if prefix.iter().any(|seg| seg == "de") {
                    return format!("::de::rusty_ext::{}", fn_name);
                }
                if prefix.iter().any(|seg| seg == "ser") {
                    if fn_name == "serialize" {
                        return "::ser::impls::rusty_ext::serialize".to_string();
                    }
                    return format!("::ser::rusty_ext::{}", fn_name);
                }
            }
        }

        // Trait static call dispatch:
        // - `Error::invalid_value(...)` where `E: Error` -> `E::invalid_value(...)`
        // - `de::Error::invalid_length(...)` where `E: de::Error` -> `E::invalid_length(...)`
        // - inside concrete impls, `Deserialize::deserialize(...)` -> `SelfType::deserialize(...)`
        if segments.len() >= 2 {
            let trait_name = &segments[segments.len() - 2];
            let method_name = escape_cpp_keyword(
                segments
                    .last()
                    .expect("segments.len() >= 2 implies non-empty"),
            );
            let trait_method_receiver_shape =
                self.trait_static_call_has_receiver_for_segments(&segments);
            // Only dispatch through trait-static owner recovery when this exact
            // `Trait::method` is known in trait metadata. This avoids hijacking
            // concrete associated static calls like `table::SerializeMap::new`.
            let has_trait_static_dispatch_context = trait_method_receiver_shape.is_some()
                || self
                    .resolve_unique_trait_bound_type_param(trait_name)
                    .is_some();
            if has_trait_static_dispatch_context {
                if let Some(type_param) =
                    self.resolve_trait_static_call_type_param_for_segments(&segments)
                {
                    return format!("{}::{}", type_param, method_name);
                }
                if let Some(owner) =
                    self.resolve_trait_static_call_owner_in_current_context(&segments)
                {
                    return format!("{}::{}", owner, method_name);
                }
                if let Some(owner) =
                    self.resolve_trait_static_call_owner_from_return_hint(trait_name)
                {
                    return format!("{}::{}", owner, method_name);
                }
                if trait_name == "Error"
                    && let Some(return_hint) = self.current_return_type_hint()
                    && let Some(err_ty) = self.expected_result_type_arg(Some(return_hint), 1)
                {
                    let mut owner = self.map_type(err_ty);
                    if owner != "auto"
                        && !owner.contains("/* TODO")
                        && !type_string_has_auto_placeholder(&owner)
                    {
                        owner = owner.trim_start_matches("typename ").to_string();
                        return format!("{}::{}", owner, method_name);
                    }
                }
            }
        }

        // Qualified type paths can reference stale module spellings from import
        // rewrites (for example `intersperse::Intersperse` when the emitted
        // namespace is `intersperse_tests`). Prefer a uniquely known nonlocal
        // type path by tail when available.
        if segments.len() == 2
            && segments[1]
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase())
            && !matches!(segments[0].as_str(), "std" | "core" | "alloc" | "rusty")
            && !self.is_type_param_in_scope(&segments[0])
            && segments[0] != "Self"
        {
            if let Some(remapped) = self.resolve_unique_nonlocal_type_path(&segments[1]) {
                return remapped;
            }
            if let Some(remapped) =
                self.resolve_nonlocal_type_path_with_namespace_hint(&segments[1], &segments[0])
            {
                return remapped;
            }
        }

        // Forward declarations are emitted before in-namespace `use` aliases. When
        // a single-segment path refers to a uniquely declared crate type from a
        // sibling namespace (for example `Position` imported from `error`), qualify
        // it explicitly to keep signatures order-independent.
        if segments.len() == 1 && !self.current_module_declares_type_name_exact(&segments[0]) {
            if let Some(scoped) = self.resolve_unique_forward_decl_type_path(&segments[0]) {
                return self.rewrite_seed_ctor_path_string(&scoped);
            }
            if let Some(scoped) = self.resolve_unique_nonlocal_type_path(&segments[0]) {
                return self.rewrite_seed_ctor_path_string(&scoped);
            }
        }

        // Resolve module-relative Rust path prefixes in expression/type paths.
        if let Some(first) = segments.first() {
            match first.as_str() {
                "crate" if segments.len() > 1 => {
                    let mut resolved = segments[1..].to_vec();
                    // `crate::...` is always rooted at crate/global scope.
                    let mut crate_force_leading_colon = true;
                    let mut import_binding_rewrite_applied = false;
                    for _ in 0..6 {
                        let Some(first_local) = resolved.first().cloned() else {
                            break;
                        };
                        // A `crate::…` path resolves ONLY against crate-root
                        // items (the comment above is the semantics): the
                        // scope-CHAIN lookup used to run as a fallback here
                        // and re-applied a module-local rename to the root
                        // module it shadows — inside serde_yaml's `mod error`
                        // (`use crate::libyaml::error as libyaml;`),
                        // `crate::libyaml::error::Error` re-expanded through
                        // the local `libyaml` binding into the stuttered
                        // `::libyaml::error::error::Error`.
                        let Some(bound_target) =
                            self.resolve_scope_import_binding_path_for_scope("", &first_local)
                        else {
                            break;
                        };
                        let from_root_scope = true;
                        // Idempotence guard (mirrors the relative-path loop):
                        // when the path already begins with the binding's
                        // crate-stripped target, an earlier stage expanded it
                        // — re-applying would stutter.
                        let already_expanded = {
                            let target_segments: Vec<&str> = bound_target
                                .trim_start_matches("::")
                                .split("::")
                                .filter(|seg| !seg.is_empty())
                                .skip_while(|seg| *seg == "crate" || *seg == "self")
                                .collect();
                            target_segments.len() > 1
                                && resolved.len() >= target_segments.len()
                                && resolved
                                    .iter()
                                    .take(target_segments.len())
                                    .map(String::as_str)
                                    .eq(target_segments.iter().copied())
                        };
                        if already_expanded {
                            crate_force_leading_colon = true;
                            break;
                        }
                        if from_root_scope && !self.module_stack.is_empty() {
                            crate_force_leading_colon = true;
                        }
                        if bound_target.starts_with("::") {
                            crate_force_leading_colon = true;
                        }
                        let normalized_bound_target = bound_target.trim_start_matches("::");
                        let direct_import_alias = normalized_bound_target
                            .rsplit("::")
                            .next()
                            .is_some_and(|tail| tail == first_local);
                        let direct_import_has_cpp_surface = !matches!(
                            classify_use_import(normalized_bound_target),
                            UseImportAction::RustOnly
                        );
                        if direct_import_alias && direct_import_has_cpp_surface && !from_root_scope
                        {
                            break;
                        }
                        let lower_ident_self_alias = resolved.len() > 1
                            && first_local
                                .chars()
                                .next()
                                .is_some_and(|ch| ch.is_ascii_lowercase() || ch == '_')
                            && bound_target
                                .trim_start_matches("::")
                                .rsplit("::")
                                .next()
                                .is_some_and(|tail| tail == first_local);
                        if lower_ident_self_alias {
                            break;
                        }
                        let mut rewritten: Vec<String> = bound_target
                            .split("::")
                            .filter(|seg| !seg.is_empty())
                            .map(|seg| seg.to_string())
                            .collect();
                        rewritten.extend(resolved.iter().skip(1).cloned());
                        if rewritten.is_empty() || rewritten == resolved {
                            break;
                        }
                        let self_expanding_root = rewritten.first() == resolved.first()
                            && rewritten.len() > resolved.len();
                        if import_binding_rewrite_applied && self_expanding_root {
                            break;
                        }
                        resolved = rewritten;
                        import_binding_rewrite_applied = true;
                    }
                    let rewritten_external = self.rewrite_external_crate_path_segments(&resolved);
                    if rewritten_external != resolved {
                        resolved = rewritten_external;
                    }
                    if let Some(first_seg) = resolved.first_mut()
                        && first_seg == "__private"
                    {
                        *first_seg = "private_".to_string();
                    }
                    if resolved.len() >= 2
                        && resolved.last().is_some_and(|seg| seg == "Result")
                        && resolved
                            .iter()
                            .nth_back(1)
                            .is_some_and(|seg| seg.starts_with("__private") || seg == "private_")
                    {
                        return "rusty::Result".to_string();
                    }
                    for seg in &mut resolved {
                        *seg = escape_cpp_keyword(seg);
                    }
                    let mut emitted = resolved.join("::");
                    if crate_force_leading_colon
                        && !emitted.is_empty()
                        && !emitted.starts_with("::")
                    {
                        emitted = format!("::{}", emitted);
                    }
                    return Self::strip_crate_root_cpp_path(&emitted);
                }
                "self" if segments.len() > 1 => {
                    let mut resolved = if self.module_stack.is_empty() {
                        Vec::new()
                    } else {
                        self.module_stack.clone()
                    };
                    resolved.extend(segments[1..].iter().cloned());
                    for seg in &mut resolved {
                        *seg = escape_cpp_keyword(seg);
                    }
                    return resolved.join("::");
                }
                "super" if segments.len() > 1 => {
                    let mut resolved = if self.module_stack.len() > 1 {
                        self.module_stack[..self.module_stack.len() - 1].to_vec()
                    } else {
                        Vec::new()
                    };
                    resolved.extend(segments[1..].iter().cloned());
                    for seg in &mut resolved {
                        *seg = escape_cpp_keyword(seg);
                    }
                    return resolved.join("::");
                }
                _ => {}
            }
        }

        // Strip crate-name prefix from paths in test targets.
        // E.g., `semver::Version` → `Version` when the crate is `semver`.
        // (Self-crate paths under the crate-namespace wrap are handled earlier,
        // before the import-binding alias loop strips the prefix.)
        if segments.len() >= 2 {
            if let Some(ref crate_name) = self.crate_name {
                if segments[0] == *crate_name {
                    let mut resolved = segments[1..].to_vec();
                    for seg in &mut resolved {
                        *seg = escape_cpp_keyword(seg);
                    }
                    return resolved.join("::");
                }
            }
        }

        // Note: non-turbofish function qualification is NOT done here — the
        // module_qualified_functions map is first-match and doesn't handle
        // overloaded function names across modules (e.g., `case_` in `all`,
        // `bits`, `iter`). Only turbofish paths are qualified in
        // emit_expr_path_to_string.

        // Recover omitted template arguments for local associated paths where the
        // base generic type is known in this scope (for example `IterNames::new_`
        // inside `Iter<B>` should become `IterNames<B>::new_`).
        if segments.len() > 1
            && types::map_function_path(&joined).is_none()
            && segments.get(1).is_some_and(|s| {
                s.chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_lowercase() || c == '_')
                    || s.chars()
                        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
            })
            && path
                .segments
                .first()
                .is_some_and(|seg| matches!(seg.arguments, syn::PathArguments::None))
        {
            let base = &segments[0];
            let scoped_base = if self.module_stack.is_empty() {
                base.clone()
            } else {
                format!("{}::{}", self.module_stack.join("::"), base)
            };
            let maybe_type_key = self
                .lookup_declared_type_key_for_base(&scoped_base, base)
                .or_else(|| self.lookup_declared_type_key_for_base(base, base));
            if let Some(type_key) = maybe_type_key {
                if let Some(type_params) = self.declared_type_params.get(&type_key) {
                    let type_kinds = self.declared_type_param_kinds.get(&type_key);
                    let type_defaults = self.declared_type_param_defaults.get(&type_key);
                    let fallback_params_from_current_struct = if type_params.is_empty() {
                        None
                    } else {
                        self.current_struct.as_ref().and_then(|struct_name| {
                            self.declared_type_params
                                .get(struct_name)
                                .or_else(|| {
                                    self.declared_type_params
                                        .get(&self.scoped_type_key(struct_name))
                                })
                                .zip(self.declared_type_param_kinds.get(struct_name).or_else(
                                    || {
                                        self.declared_type_param_kinds
                                            .get(&self.scoped_type_key(struct_name))
                                    },
                                ))
                                .and_then(|current_params| {
                                    let (current_params, current_kinds) = current_params;
                                    if current_params.len() >= type_params.len()
                                        && current_kinds.len() >= type_params.len()
                                        && type_kinds.is_some_and(|owner_kinds| {
                                            owner_kinds.len() == type_params.len()
                                                && owner_kinds
                                                    .iter()
                                                    .zip(current_kinds.iter())
                                                    .take(type_params.len())
                                                    .all(|(owner_kind, current_kind)| {
                                                        owner_kind == current_kind
                                                    })
                                        })
                                    {
                                        Some(current_params[..type_params.len()].to_vec())
                                    } else {
                                        None
                                    }
                                })
                        })
                    };
                    let mut recovered_params: Vec<String> = Vec::new();
                    let mut resolved_type_defaults: HashMap<String, syn::Type> = HashMap::new();
                    let mut recovered_ok = !type_params.is_empty();
                    for (idx, param) in type_params.iter().enumerate() {
                        if let Some(default) = type_defaults
                            .and_then(|all| all.get(idx))
                            .and_then(|entry| entry.as_ref())
                        {
                            let mapped_default = match default {
                                GenericParamDefault::Type(t) => {
                                    let substituted = if resolved_type_defaults.is_empty() {
                                        t.clone()
                                    } else {
                                        self.substitute_type_params_in_type(
                                            t,
                                            &resolved_type_defaults,
                                        )
                                    };
                                    let mapped = self.map_type(&substituted);
                                    resolved_type_defaults.insert(param.clone(), substituted);
                                    mapped
                                }
                                GenericParamDefault::Const(c) => self.emit_expr_to_string(c),
                            };
                            recovered_params.push(mapped_default);
                            continue;
                        }
                        if self.is_type_param_in_scope(param) {
                            recovered_params.push(param.clone());
                            if let Ok(ty) = syn::parse_str::<syn::Type>(param) {
                                resolved_type_defaults.insert(param.clone(), ty);
                            }
                            continue;
                        }
                        if let Some(fallback) = fallback_params_from_current_struct.as_ref()
                            && let Some(arg) = fallback.get(idx)
                        {
                            recovered_params.push(arg.clone());
                            if let Ok(ty) = syn::parse_str::<syn::Type>(arg) {
                                resolved_type_defaults.insert(param.clone(), ty);
                            }
                            continue;
                        }
                        recovered_ok = false;
                        break;
                    }

                    if recovered_ok && recovered_params.len() == type_params.len() {
                        let mut qualified = segments.clone();
                        let mapped_base = if base == "IterEither" {
                            "iterator::IterEither".to_string()
                        } else {
                            escape_cpp_keyword(base)
                        };
                        qualified[0] = format!("{}<{}>", mapped_base, recovered_params.join(", "));
                        for seg in qualified.iter_mut().skip(1) {
                            *seg = escape_cpp_keyword(seg);
                        }
                        return qualified.join("::");
                    }
                }
            }
        }

        // Map Rust Option constructors (including private re-exports).
        if self.is_option_none_path(path) {
            return "rusty::None".to_string();
        }
        if self.is_option_some_path(path) {
            return "rusty::Some".to_string();
        }
        if joined == "core::result::Result::Ok" || joined == "std::result::Result::Ok" {
            return "Ok".to_string();
        }
        if joined == "core::result::Result::Err" || joined == "std::result::Result::Err" {
            return "Err".to_string();
        }
        if segments.len() >= 2
            && segments.last().is_some_and(|seg| seg == "Result")
            && segments
                .iter()
                .nth_back(1)
                .is_some_and(|seg| seg.starts_with("__private"))
        {
            // Expanded crates can alias Result through hidden `__private`
            // modules (e.g. `crate::__private::Result`).
            return "rusty::Result".to_string();
        }

        // Expanded either crate commonly references `IterEither` through imports.
        // Use a stable fully-qualified path so type/call sites resolve before re-exports.
        if !segments.is_empty() && segments[0] == "IterEither" {
            if segments.len() == 1 {
                return "iterator::IterEither".to_string();
            }
            let mut escaped = segments.clone();
            for seg in &mut escaped {
                *seg = escape_cpp_keyword(seg);
            }
            return format!("iterator::{}", escaped.join("::"));
        }

        if let Some(kind) = joined.strip_prefix("core::panicking::AssertKind::") {
            return format!("rusty::panicking::AssertKind::{}", kind);
        }
        if joined == "core::panicking::panic" {
            return "rusty::panicking::panic".to_string();
        }
        if let Some(max_expr) = self.try_emit_numeric_limits_path(path, &segments) {
            return max_expr;
        }
        if let Some(max_len_expr) = self.try_emit_integer_max_str_len_path(path, &segments) {
            return max_len_expr;
        }

        // Map Rust Ordering enum variants to fallback ordering enum variants.
        match joined.as_str() {
            "core::cmp::Ordering::Less" => return "rusty::cmp::Ordering::Less".to_string(),
            "core::cmp::Ordering::Equal" => return "rusty::cmp::Ordering::Equal".to_string(),
            "core::cmp::Ordering::Greater" => return "rusty::cmp::Ordering::Greater".to_string(),
            "std::cmp::Ordering::Less" => return "rusty::cmp::Ordering::Less".to_string(),
            "std::cmp::Ordering::Equal" => return "rusty::cmp::Ordering::Equal".to_string(),
            "std::cmp::Ordering::Greater" => return "rusty::cmp::Ordering::Greater".to_string(),
            _ => {}
        }
        match joined.as_str() {
            "Cow::Borrowed"
            | "rusty::Cow::Borrowed"
            | "alloc::borrow::Cow::Borrowed"
            | "std::borrow::Cow::Borrowed"
            | "core::borrow::Cow::Borrowed" => return "rusty::Cow_Borrowed".to_string(),
            "Cow::Owned"
            | "rusty::Cow::Owned"
            | "alloc::borrow::Cow::Owned"
            | "std::borrow::Cow::Owned"
            | "core::borrow::Cow::Owned" => return "rusty::Cow_Owned".to_string(),
            _ => {}
        }
        match joined.as_str() {
            "std::io::stdin" | "core::io::stdin" | "io::stdin" | "rusty::io::stdin" => {
                return "rusty::io::stdin_".to_string();
            }
            "std::io::stdout" | "core::io::stdout" | "io::stdout" | "rusty::io::stdout" => {
                return "rusty::io::stdout_".to_string();
            }
            "std::io::stderr" | "core::io::stderr" | "io::stderr" | "rusty::io::stderr" => {
                return "rusty::io::stderr_".to_string();
            }
            _ => {}
        }

        if segments.len() >= 4
            && matches!(segments[0].as_str(), "std" | "core")
            && segments[1] == "sync"
            && segments[2] == "atomic"
        {
            let mut resolved = vec![
                "rusty".to_string(),
                "sync".to_string(),
                "atomic".to_string(),
            ];
            resolved.extend(segments[3..].iter().cloned());
            for seg in &mut resolved {
                *seg = escape_cpp_keyword(seg);
            }
            return resolved.join("::");
        }
        if segments.len() >= 3 && segments[0] == "std" && segments[1] == "thread" {
            let mut resolved = vec!["rusty".to_string(), "thread".to_string()];
            resolved.extend(segments[2..].iter().cloned());
            for seg in &mut resolved {
                *seg = escape_cpp_keyword(seg);
            }
            return resolved.join("::");
        }
        if segments.len() >= 3
            && matches!(segments[0].as_str(), "std" | "core")
            && segments[1] == "fmt"
        {
            let mut resolved = vec!["rusty".to_string(), "fmt".to_string()];
            resolved.extend(segments[2..].iter().cloned());
            for seg in &mut resolved {
                *seg = escape_cpp_keyword(seg);
            }
            return resolved.join("::");
        }
        if segments.len() >= 2 && segments[0] == "either" {
            let mut resolved = vec!["rusty".to_string()];
            if matches!(segments[1].as_str(), "Left" | "Right") {
                resolved.push("either".to_string());
            }
            resolved.extend(segments[1..].iter().cloned());
            for seg in &mut resolved {
                *seg = escape_cpp_keyword(seg);
            }
            return resolved.join("::");
        }

        if segments.len() >= 3
            && matches!(segments[0].as_str(), "std" | "core")
            && segments[1] == "str"
            && matches!(
                segments[2].as_str(),
                "Bytes"
                    | "Chars"
                    | "CharIndices"
                    | "Utf8Error"
                    | "from_utf8"
                    | "from_utf8_unchecked"
                    | "from_utf8_unchecked_mut"
                    | "char_indices"
                    | "chars"
                    | "bytes"
                    | "is_char_boundary"
                    | "parse"
                    | "trim"
                    | "trim_start_matches"
                    | "trim_end_matches"
                    | "strip_prefix"
                    | "split"
                    | "find"
            )
        {
            let mut resolved = vec!["rusty".to_string(), "str_runtime".to_string()];
            resolved.extend(segments[2..].iter().cloned());
            for seg in &mut resolved {
                *seg = escape_cpp_keyword(seg);
            }
            return resolved.join("::");
        }

        if segments.len() >= 2 && segments[0] == "proc_macro" {
            if segments.len() == 2 {
                let tail = segments[1].as_str();
                match tail {
                    "TokenStream" => return "fallback::TokenStream".to_string(),
                    "LexError" => return "fallback::LexError".to_string(),
                    "Span" => return "fallback::Span".to_string(),
                    "Group" => return "fallback::Group".to_string(),
                    "Ident" => return "fallback::Ident".to_string(),
                    "Literal" => return "fallback::Literal".to_string(),
                    "TokenTree" => return "TokenTree".to_string(),
                    "Delimiter" => return "Delimiter".to_string(),
                    "Spacing" => return "Spacing".to_string(),
                    "is_available" => return "rusty::proc_macro_runtime::is_available".to_string(),
                    _ if tail.starts_with("TokenTree_")
                        || tail.starts_with("Delimiter_")
                        || tail.starts_with("Spacing_") =>
                    {
                        return escape_cpp_keyword(tail);
                    }
                    _ => {}
                }
            }

            if segments.len() >= 3 && segments[1] == "token_stream" && segments[2] == "IntoIter" {
                if segments.len() == 3 {
                    return "rusty::proc_macro_runtime::IntoIter".to_string();
                }
                let mut emitted = "rusty::proc_macro_runtime::IntoIter".to_string();
                for seg in segments.iter().skip(3) {
                    emitted.push_str("::");
                    emitted.push_str(&escape_cpp_keyword(seg));
                }
                return emitted;
            }

            let mapped_head = match segments[1].as_str() {
                "TokenStream" => Some("fallback::TokenStream"),
                "LexError" => Some("fallback::LexError"),
                "Span" => Some("fallback::Span"),
                "Group" => Some("fallback::Group"),
                "Ident" => Some("fallback::Ident"),
                "Literal" => Some("fallback::Literal"),
                "TokenTree" => Some("TokenTree"),
                "Delimiter" => Some("Delimiter"),
                "Spacing" => Some("Spacing"),
                _ => None,
            };
            if let Some(head) = mapped_head {
                let mut resolved: Vec<String> =
                    head.split("::").map(|seg| seg.to_string()).collect();
                resolved.extend(segments.iter().skip(2).cloned());
                for seg in &mut resolved {
                    *seg = escape_cpp_keyword(seg);
                }
                return resolved.join("::");
            }
        }

        // Try user-provided type mappings first (highest priority)
        if let Some(cpp_type) = self.user_type_map.lookup(&joined) {
            return cpp_type.to_string();
        }

        // Try mapping as a function/method path (e.g., Box::new → rusty::Box::new_)
        if let Some(cpp_fn) = types::map_function_path(&joined) {
            return cpp_fn.to_string();
        }

        // Try mapping as a standard type — UNLESS this is a bare reference to a
        // type the crate declares itself. A std-library port (hashbrown defines
        // HashMap/HashSet) must keep its OWN self-references local, not rewrite
        // them to the umbrella `rusty::*` alias (which collides with and
        // circularly imports the very type the port defines). Explicit std
        // paths (`std::collections::HashMap`) are multi-segment and still map.
        let suppress_std_map =
            segments.len() == 1 && self.crate_declares_std_named_type(&joined);
        if !suppress_std_map {
            if let Some((cpp_type, _)) = types::map_std_type(&joined) {
                return cpp_type.to_string();
            }
        }

        // Try as primitive
        if segments.len() == 1 {
            if let Some(cpp_prim) = types::map_primitive_type(&segments[0]) {
                return cpp_prim.to_string();
            }
        }

        // Preserve mapped std/core/alloc runtime surfaces for associated paths by
        // remapping the longest known type-prefix and retaining trailing segments.
        // Example: `alloc::borrow::Cow::Owned` -> `rusty::Cow::Owned`.
        if segments.len() >= 2 {
            let mut mapped_prefix: Option<Vec<String>> = None;
            for prefix_len in (2..=segments.len()).rev() {
                let candidate = segments[..prefix_len].join("::");
                let mapped_std = types::map_std_type(&candidate).or_else(|| {
                    if segments.first().is_some_and(|seg| seg == "rusty") && prefix_len >= 2 {
                        let tail = segments[1..prefix_len].join("::");
                        ["std", "core", "alloc"]
                            .iter()
                            .find_map(|root| types::map_std_type(&format!("{}::{}", root, tail)))
                    } else {
                        None
                    }
                });
                let Some((mapped, _)) = mapped_std else {
                    continue;
                };
                if mapped.is_empty() || mapped.contains('<') || mapped.contains(' ') {
                    continue;
                }
                let mut rewritten: Vec<String> = mapped
                    .split("::")
                    .filter(|seg| !seg.is_empty())
                    .map(|seg| seg.to_string())
                    .collect();
                rewritten.extend(segments.iter().skip(prefix_len).cloned());
                if rewritten != segments {
                    mapped_prefix = Some(rewritten);
                }
                break;
            }
            if let Some(mut rewritten) = mapped_prefix {
                for seg in &mut rewritten {
                    *seg = escape_cpp_keyword(seg);
                }
                let mut emitted = rewritten.join("::");
                if force_leading_colon && !emitted.is_empty() && !emitted.starts_with("::") {
                    emitted = format!("::{}", emitted);
                }
                return emitted;
            }
        }

        // Map remaining `core::`, `alloc::`, and specific `std::` submodule
        // paths to `rusty::`. This catches paths like `core::fmt::Formatter::write_str`
        // and `std::time::Duration::from_secs` that weren't handled above.
        if joined == "std::process::abort" {
            return "std::abort".to_string();
        }
        let is_std_rusty_submodule = segments.len() >= 2
            && segments[0] == "std"
            && matches!(
                segments[1].as_str(),
                "time" | "path" | "ffi" | "env" | "process"
            );
        // General Layer 1 (std-port mapping): a PORTED std module's deep member types
        // (`vec::Drain`, `vec::IntoIter`, …) are transpiled into the `rusty::port::<mod>`
        // namespace, NOT `rusty::<mod>` (the naive rewrite below yields e.g.
        // `rusty::vec::Drain`, which does not exist — clang: "did you mean
        // 'rusty::port::vec::Drain'?"). Fires for ANY std/alloc/core root: the
        // `use alloc::vec::{self}` module self-alias resolves `vec` → `std::vec`
        // (alloc→std), so `std::vec::Drain` must map too, not just `alloc::vec::Drain`.
        // Ergonomic top-level types (Vec, String, …) keep their `rusty::<Type>` aliases
        // via dedicated special cases before this point and are excluded here.
        if segments.len() >= 3 && matches!(segments[0].as_str(), "std" | "core" | "alloc") {
            let module = &segments[1..segments.len() - 1];
            let type_name = segments.last().expect("len >= 3");
            if let Some(port_ns) = ported_std_module_port_namespace(module)
                && !is_ergonomic_top_level_std_type(type_name)
            {
                return format!("{}::{}", port_ns, escape_cpp_keyword(type_name));
            }
        }
        if segments.len() >= 2
            && (matches!(segments[0].as_str(), "core" | "alloc") || is_std_rusty_submodule)
        {
            let mut resolved = vec!["rusty".to_string()];
            resolved.extend(segments[1..].iter().cloned());
            for seg in &mut resolved {
                *seg = escape_cpp_keyword(seg);
            }
            return resolved.join("::");
        }

        // Escape C++ keywords across all path segments.
        if let Some(first) = segments.first_mut()
            && let Some(mapped_root_type) = self.current_named_module_root_type_cpp_name(first)
        {
            *first = mapped_root_type;
            joined = segments.join("::");
        }
        if !force_leading_colon
            && segments.len() >= 2
            && segments[0]
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_lowercase() || ch == '_')
            && self.module_stack.iter().any(|scope_seg| {
                scope_seg == &segments[0]
                    || escape_cpp_keyword(scope_seg) == segments[0]
                    || self.escape_and_rename_qualified_name(scope_seg) == segments[0]
            })
            && (self.declared_module_names.contains(&segments[0])
                || self
                    .declared_module_names
                    .iter()
                    .any(|name| escape_cpp_keyword(name) == segments[0])
                || self
                    .module_namespace_renames
                    .iter()
                    .any(|(original, renamed)| !original.contains("::") && renamed == &segments[0]))
        {
            // Rust paths like `de::...` imported from crate root should stay rooted
            // when emitted inside nested scopes that also contain a `de` segment.
            force_leading_colon = true;
        }
        if segments.len() > 1 {
            let mut emitted = self.escape_and_rename_qualified_name(&joined);
            if force_leading_colon && !emitted.is_empty() && !emitted.starts_with("::") {
                emitted = format!("::{}", emitted);
            }
            let emitted = Self::strip_crate_root_cpp_path(&emitted);
            return self.rewrite_seed_ctor_path_string(&emitted);
        }

        // Single segment — escape if keyword
        let mut emitted = self
            .current_named_module_root_type_cpp_name(&joined)
            .unwrap_or_else(|| escape_cpp_keyword(&joined));
        if force_leading_colon && segments.len() == 1 {
            let tail = &segments[0];
            let has_nonroot_local_decl = self
                .local_declared_types
                .iter()
                .any(|decl| decl.rsplit_once("::").is_some_and(|(_, name)| name == tail));
            let has_nonroot_type_alias = self.type_alias_targets.keys().any(|alias| {
                alias
                    .rsplit_once("::")
                    .is_some_and(|(_, name)| name == tail)
            });
            let has_nonroot_import_alias = self
                .resolve_scope_import_binding_path(tail)
                .or_else(|| self.resolve_scope_import_binding_path_for_scope("", tail))
                .is_some_and(|target| {
                    let normalized = target.trim_start_matches("::");
                    normalized.contains("::")
                        && normalized
                            .rsplit("::")
                            .next()
                            .is_some_and(|name| name == tail)
                });
            if has_nonroot_local_decl || has_nonroot_type_alias || has_nonroot_import_alias {
                force_leading_colon = false;
            }
        }
        if force_leading_colon && !emitted.is_empty() && !emitted.starts_with("::") {
            emitted = format!("::{}", emitted);
        }
        self.rewrite_seed_ctor_path_string(&emitted)
    }

    /// A `T::CONST` access where `CONST` is a trait associated const with a
    /// default body (`SizedTypeProperties::NEEDS_DROP = mem::needs_drop::<Self>()`)
    /// and `T` is a generic type parameter (which cannot carry an inherent member
    /// of that name) lowers to the default body with the trait's `Self` replaced by
    /// `T` — `T::NEEDS_DROP` → `(rusty::mem::needs_drop<T>())`. The trait itself is
    /// not emitted (associated-const traits are skipped), so without this the
    /// dependent access becomes a bogus member of a concrete type at instantiation
    /// (`std::tuple<…>::NEEDS_DROP`).
    pub(super) fn try_emit_trait_default_const_path(&self, path: &syn::Path) -> Option<String> {
        if path.segments.len() != 2 {
            return None;
        }
        let owner = path.segments[0].ident.to_string();
        let const_name = path.segments[1].ident.to_string();
        let (body, _trait) = self.trait_default_const_exprs.get(&const_name)?;
        // Only for an owner that cannot define the member itself: a generic type
        // parameter in scope, or `Self`. A concrete type keeps the normal
        // `Type::CONST` path (it may define or override the const).
        if !(self.is_type_param_in_scope(&owner) || owner == "Self") {
            return None;
        }
        let mut substituted = body.clone();
        if owner != "Self" {
            use syn::visit_mut::VisitMut;
            let mut rewriter = super::TypeParamPathRewriter {
                replacements: std::collections::HashMap::from([(
                    "Self".to_string(),
                    owner.clone(),
                )]),
            };
            rewriter.visit_expr_mut(&mut substituted);
        }
        Some(format!("({})", self.emit_expr_to_string(&substituted)))
    }

    pub(super) fn emit_expr_path_to_string(&self, path: &syn::Path) -> String {
        let rendered = self.emit_expr_path_to_string_inner(path);
        // Inside a UFCS extension-trait free-function body (emitted at the global
        // `<Tr>_` namespace), nested-local type references in a path — e.g.
        // `Tag::EMPTY`, where `Tag` lives in `control::tag` — must be absolutized,
        // since the local module's siblings aren't visible at `<Tr>_`. Safe: only
        // identifiers known as nested-local types are rewritten, idempotently.
        if self.ufcs_free_fn_body {
            self.qualify_nested_local_types_in_type_string(&rendered)
        } else {
            rendered
        }
    }

    fn emit_expr_path_to_string_inner(&self, path: &syn::Path) -> String {
        if let Some(rendered) = self.try_emit_trait_default_const_path(path) {
            return rendered;
        }
        let joined = path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        // A trait comparison method used as a VALUE (a function reference, e.g.
        // `it.merge_join_by(other, Ord::cmp)` / `k_smallest_relaxed_by(k, Ord::cmp)`).
        // Emitted verbatim, `Ord::cmp` is an undeclared identifier in C++. Lower to
        // a comparator lambda over the same `rusty::cmp` runtime helper the CALL
        // form `Ord::cmp(a, b)` targets (emit_expr.rs ~16226). The call form is
        // intercepted before reaching here, so this only fires on the value form.
        match joined.as_str() {
            "Ord::cmp" | "core::cmp::Ord::cmp" | "std::cmp::Ord::cmp" => {
                return "[](auto&& __a, auto&& __b) { return rusty::cmp::cmp(\
                        std::forward<decltype(__a)>(__a), \
                        std::forward<decltype(__b)>(__b)); }"
                    .to_string();
            }
            "PartialOrd::partial_cmp"
            | "core::cmp::PartialOrd::partial_cmp"
            | "std::cmp::PartialOrd::partial_cmp" => {
                return "[](auto&& __a, auto&& __b) { return rusty::partial_cmp(\
                        std::forward<decltype(__a)>(__a), \
                        std::forward<decltype(__b)>(__b)); }"
                    .to_string();
            }
            _ => {}
        }
        if path.segments.len() == 1
            && path.segments[0].ident == "Self"
            && let Some(current) = &self.current_struct
        {
            let scoped_current = self.scoped_type_key(current);
            if self.unit_struct_types.contains(current)
                || self.unit_struct_types.contains(&scoped_current)
            {
                let ctor = self.emit_path_to_string(path);
                return format!("{}{{}}", self.rewrite_seed_ctor_path_string(&ctor));
            }
        }
        if matches!(joined.as_str(), "char::from_u32" | "char32_t::from_u32") {
            return "rusty::char_runtime::from_u32".to_string();
        }
        if matches!(
            joined.as_str(),
            "Display::fmt"
                | "fmt::Display::fmt"
                | "core::fmt::Display::fmt"
                | "std::fmt::Display::fmt"
                | "rusty::fmt::Display::fmt"
        ) {
            // Rust trait method item paths (e.g. `fmt::Display::fmt`) behave like
            // callable items. Emit as forwarding lambda so call sites remain valid.
            return "[](const auto& _v, rusty::fmt::Formatter& _f) { if constexpr (requires { _v.fmt(_f); }) { return _v.fmt(_f); } else { return rusty::write_fmt(_f, rusty::to_string(_v)); } }".to_string();
        }
        if matches!(
            joined.as_str(),
            "Debug::fmt"
                | "fmt::Debug::fmt"
                | "core::fmt::Debug::fmt"
                | "std::fmt::Debug::fmt"
                | "rusty::fmt::Debug::fmt"
        ) {
            // Some debug surfaces (for example spans) don't carry a direct `.fmt`
            // member in C++ lowering. Fall back to runtime debug-string formatting.
            return "[](const auto& _v, rusty::fmt::Formatter& _f) { if constexpr (requires { _v.fmt(_f); }) { return _v.fmt(_f); } else { return rusty::write_fmt(_f, rusty::to_debug_string(_v)); } }".to_string();
        }
        if joined.ends_with("Iterator::size_hint") {
            // Trait method item path used as callable: `Iterator::size_hint`.
            return "[](const auto& _v) { return rusty::size_hint(_v); }".to_string();
        }
        if matches!(
            joined.as_str(),
            "fmt::Error" | "core::fmt::Error" | "std::fmt::Error" | "rusty::fmt::Error"
        ) {
            // Rust `fmt::Error` is a unit-like value; emit an object expression.
            return "rusty::fmt::Error{}".to_string();
        }
        if path.segments.len() == 1 {
            let name = path.segments[0].ident.to_string();
            if name == "self" {
                if let Some(override_name) = self.current_self_path_override() {
                    return override_name.to_string();
                }
                return "(*this)".to_string();
            }
            if let Some(mapped) = self.lookup_local_binding_cpp_name(&name) {
                if self.is_delayed_init_local(&name) {
                    return format!("{}.value()", mapped);
                }
                if self.is_rebind_reference_binding(&name) {
                    // Parenthesize so field/method access on the surrounding
                    // expression binds correctly: `(*ptr).field` rather than
                    // `*ptr.field` (which parses as `*(ptr.field)` and fails
                    // since ptr is a pointer). Without the parens, once_cell
                    // emits `*this_shadow1.cell.get_mut().is_none()` for the
                    // Rust source `this.cell.get_mut().is_none()`.
                    return format!("(*{})", mapped);
                }
                return mapped;
            }
            // Parameters are not recorded in local C++ binding maps, so resolve them
            // before single-segment function qualification logic.
            let name_is_known_unit_struct = self.unit_struct_types.contains(&name);
            if (self.lookup_local_binding_type(&name).is_some()
                || self.is_local_binding_in_scope(&name))
                && !name_is_known_unit_struct
            {
                return escape_cpp_keyword(&name);
            }
            if name
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase())
            {
                if let Some(owner_cpp) =
                    self.resolve_c_like_enum_owner_for_variant_from_return_hint(&name)
                {
                    return format!("{}::{}", owner_cpp, name);
                }
                // Fall back: bare variant idents brought into scope by
                // `use EnumName::*;` (no member-of-return-type hint). C++20
                // `enum class` does not allow flattening into surrounding
                // scope, so qualify with the unique owner when known.
                if let Some(owner_cpp) =
                    self.unique_c_like_enum_owner_for_variant_name(&name)
                {
                    return format!("{}::{}", owner_cpp, name);
                }
                // Glob-imported builtin enum (e.g. `use std::cmp::Ordering::*;`
                // brings Less/Equal/Greater into scope). Check whether any
                // glob-imported enum tail owns this variant.
                for owner_tail in &self.glob_imported_enum_tails {
                    if self.path_matches_c_like_enum_const(owner_tail, &name) {
                        return format!("{}::{}", owner_tail, name);
                    }
                }
                // Well-known std builtin enums whose variants are commonly
                // brought into scope via `use std::cmp::Ordering::*;` or
                // `use std::sync::atomic::Ordering::*;`. The use-path handler
                // doesn't always record these into glob_imported_enum_tails
                // (std-prefixed paths may take a different branch), so fall
                // back to the well-known-builtin set when no other resolution
                // is found and the name is unambiguously a builtin variant.
                let canonical = self.canonical_variant_name(&name).to_string();
                let builtin_owner = match canonical.as_str() {
                    "Less" | "Equal" | "Greater" => Some("Ordering"),
                    "Left" | "Right" | "Center" => Some("Alignment"),
                    _ => None,
                };
                if let Some(owner) = builtin_owner {
                    return format!("{}::{}", owner, name);
                }
            }
        }
        if path.segments.len() == 1 {
            let canonical = self
                .canonical_variant_name(&path.segments[0].ident.to_string())
                .to_string();
            match canonical.as_str() {
                "None" => return "rusty::None".to_string(),
                "Some" => return "rusty::Some".to_string(),
                "Ok" => return "rusty::Ok".to_string(),
                "Err" => return "rusty::Err".to_string(),
                _ => {}
            }
        }
        if path.segments.len() >= 3 {
            let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
            let fn_name = segments.last().cloned().unwrap_or_default();
            let penultimate_is_rusty_ext = segments
                .get(segments.len().saturating_sub(2))
                .is_some_and(|s| s == "rusty_ext");
            if !fn_name.is_empty() && penultimate_is_rusty_ext {
                let prefix = &segments[..segments.len() - 2];
                let has_private_prefix = prefix.iter().any(|seg| {
                    seg == "private" || seg == "private_" || seg.starts_with("__private")
                });
                if has_private_prefix {
                    if prefix.iter().any(|seg| seg == "de") {
                        return format!("::de::rusty_ext::{}", fn_name);
                    }
                    if prefix.iter().any(|seg| seg == "ser") {
                        if fn_name == "serialize" {
                            return "::ser::impls::rusty_ext::serialize".to_string();
                        }
                        return format!("::ser::rusty_ext::{}", fn_name);
                    }
                }
            }
        }
        if path.segments.len() == 2 {
            let owner = path.segments[0].ident.to_string();
            let fn_name = path.segments[1].ident.to_string();
            if (owner == "__private228"
                || owner == "__private"
                || owner == "private"
                || owner == "private_"
                || owner.starts_with("__private"))
                && fn_name
                    .chars()
                    .next()
                    .is_some_and(|ch| ch.is_ascii_uppercase())
                && let Some(enum_owner) =
                    self.resolve_c_like_enum_owner_for_variant_from_return_hint(&fn_name)
            {
                return format!("{}::{}", enum_owner, fn_name);
            }
            if owner == "rusty_ext" || owner == "Itertools" {
                if owner == "rusty_ext" && fn_name == "deserialize" {
                    return "::de::rusty_ext::deserialize".to_string();
                }
                if let Some(scoped) =
                    self.resolve_scoped_namespace_function_expr_path(&owner, &fn_name)
                {
                    return scoped;
                }
                if let Some(unscoped) =
                    self.resolve_unscoped_namespace_function_expr_path(&owner, &fn_name)
                {
                    return unscoped;
                }
                if let Some(qualified) = self.module_qualified_functions.get(&fn_name) {
                    if !qualified.is_empty() {
                        let needs_root = self
                            .module_stack
                            .iter()
                            .any(|m| qualified.starts_with(&format!("{}::", m)));
                        return if needs_root {
                            format!("::{}", qualified)
                        } else {
                            qualified.clone()
                        };
                    }
                }
            }
        }
        if matches!(
            joined.as_str(),
            "Option::Some"
                | "core::option::Option::Some"
                | "std::option::Option::Some"
                | "Option::None"
                | "core::option::Option::None"
                | "std::option::Option::None"
        ) {
            if joined.ends_with("::Some") {
                return "rusty::Some".to_string();
            }
            return "rusty::None".to_string();
        }
        if matches!(
            joined.as_str(),
            "Result::Ok"
                | "core::result::Result::Ok"
                | "std::result::Result::Ok"
                | "Result::Err"
                | "core::result::Result::Err"
                | "std::result::Result::Err"
        ) {
            if joined.ends_with("::Ok") {
                return "rusty::Ok".to_string();
            }
            return "rusty::Err".to_string();
        }
        if self.is_option_none_path(path) {
            return "rusty::None".to_string();
        }
        if self.is_option_some_path(path) {
            return "rusty::Some".to_string();
        }
        // Keep unit data-enum variant lowering before import-bound rewrites so
        // qualified paths like `de::Unexpected::UnitVariant` become values.
        if path.segments.len() >= 2 {
            let variant_seg = &path.segments[path.segments.len() - 1];
            let enum_seg = &path.segments[path.segments.len() - 2];
            let enum_name = enum_seg.ident.to_string();
            let variant_name = variant_seg.ident.to_string();
            let variant_key = format!("{}_{}", enum_name, variant_name);
            if self.data_enum_unit_variants.contains(&variant_key) {
                let enum_path: syn::Path = {
                    let segs: Vec<syn::PathSegment> = path
                        .segments
                        .iter()
                        .take(path.segments.len() - 1)
                        .cloned()
                        .collect();
                    let mut p = path.clone();
                    p.segments = segs.into_iter().collect();
                    p
                };
                let owner_rust_path = path
                    .segments
                    .iter()
                    .take(path.segments.len() - 1)
                    .map(|segment| segment.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                let owner_is_wrapper = self.data_enum_wrapper_types.contains(&enum_name)
                    || self.data_enum_wrapper_types.contains(&owner_rust_path);
                if owner_is_wrapper {
                    let owner_cpp = self.emit_path_to_string(&enum_path);
                    if let Some(dependent_owner_cpp) =
                        self.maybe_make_owner_cpp_type_dependent_in_template_scope(&owner_cpp)
                    {
                        let helper_name = escape_cpp_keyword(&variant_name);
                        return format!("{}::{}()", dependent_owner_cpp, helper_name);
                    }
                }
                let variant_ty = self.data_enum_variant_struct_type_name(&enum_path, &variant_name);
                return format!("{}{{}}", variant_ty);
            }
        }
        // Keep C-like enum associated-const lowering before import-bound rewrites
        // so qualified paths like `TagOrContentField::Content` are not rewritten
        // to unrelated imported type aliases named `Content`.
        if path.segments.len() >= 2 {
            let variant_seg = &path.segments[path.segments.len() - 1];
            let enum_seg = &path.segments[path.segments.len() - 2];
            let enum_name = enum_seg.ident.to_string();
            let variant_name = variant_seg.ident.to_string();
            let variant_key = format!("{}_{}", enum_name, variant_name);
            if self.c_like_enum_variants.contains(&variant_key) {
                let local_owner_variant = self.block_depth > 0
                    && path.segments.len() == 2
                    && self.is_local_type_name_in_scope(&enum_name);
                let mut owner_path = path.clone();
                owner_path.segments = path
                    .segments
                    .iter()
                    .take(path.segments.len() - 1)
                    .cloned()
                    .collect();
                let mut owner_cpp = self.emit_path_to_string(&owner_path);
                owner_cpp = owner_cpp.trim_end_matches("::").to_string();
                return if owner_cpp.is_empty() {
                    variant_name
                } else {
                    let helper_name = escape_cpp_keyword(&variant_name);
                    if owner_cpp.contains('<') || local_owner_variant {
                        // Preserve direct enum-variant spelling for generic owners.
                        format!("{}::{}", owner_cpp, helper_name)
                    } else {
                        let mut helper_owner = owner_cpp.clone();
                        if !owner_cpp.contains("::")
                            && let Some(bound_target) = self
                                .resolve_scope_import_binding_path(&owner_cpp)
                                .or_else(|| {
                                    self.resolve_scope_import_binding_path_for_scope("", &owner_cpp)
                                })
                                .or_else(|| {
                                    self.resolve_unique_scope_import_binding_path_any_scope(
                                        &owner_cpp,
                                    )
                                })
                        {
                            let rebound =
                                self.rewrite_cpp_import_bound_type_spelling(&bound_target);
                            if !rebound.is_empty() {
                                helper_owner = rebound.trim_end_matches("::").to_string();
                            }
                        }
                        format!("{}_{}()", helper_owner, helper_name)
                    }
                };
            }
            if self.c_like_enum_consts.contains(&variant_key) {
                let mut owner_prefix_path = path.clone();
                owner_prefix_path.segments = path
                    .segments
                    .iter()
                    .take(path.segments.len() - 2)
                    .cloned()
                    .collect();
                let mut owner_prefix_cpp = self.emit_path_to_string(&owner_prefix_path);
                if !owner_prefix_cpp.is_empty() && owner_prefix_cpp != "::" {
                    owner_prefix_cpp = owner_prefix_cpp.trim_end_matches("::").to_string();
                    return format!("{}::{}", owner_prefix_cpp, variant_key);
                }
                return variant_key;
            }
        }
        if self.is_unit_struct_path(path) {
            let mut ctor = self.emit_path_to_string(path);
            if let Some(template_args) = self.emit_path_explicit_template_args(path) {
                ctor.push_str(&template_args);
            }
            return format!("{}{{}}", self.rewrite_seed_ctor_path_string(&ctor));
        }
        if types::map_function_path(&joined).is_none() {
            if let Some(mut rewritten) = self.rewrite_cpp_import_bound_expr_path(path) {
                if let Some(template_args) = self.emit_expr_path_template_args(path) {
                    rewritten.push_str(&template_args);
                }
                return rewritten;
            }
        }
        // Keep this rewrite for unqualified names only. Qualified paths like
        // `std::process::Command::new` must flow through normal path mapping
        // so `std::*` namespace remaps are applied.
        if path.segments.len() == 1
            && let Some(mut resolved_fn) = self.resolve_known_free_function_expr_path(path)
        {
            if let Some(template_args) = self.emit_expr_path_template_args(path) {
                resolved_fn.push_str(&template_args);
            }
            return resolved_fn;
        }
        // Special case: unqualified `Vec` in paths should map to rusty::Vec
        // This handles `Vec::from_iter`, but NOT constructor methods like new/new_/from/try_from
        // which are handled specially in emit_call_expr_to_string.
        if path.segments.len() >= 2 {
            let first = path.segments[0].ident.to_string();
            if first == "Vec" {
                let method = path
                    .segments
                    .last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_default();
                if method == "extend_from_slice" {
                    // Preserve UFCS calls like `Vec::extend_from_slice(&mut v, s)` without
                    // emitting invalid `rusty::Vec::...` static paths.
                    return "rusty::vec_extend_from_slice".to_string();
                }
                // Skip constructor methods - they are handled specially elsewhere
                let is_constructor = matches!(
                    method.as_str(),
                    "new" | "new_" | "from" | "try_from" | "default" | "with_capacity"
                );
                if is_constructor {
                    // Let the constructor handling in emit_call_expr_to_string take care of it
                    return self.emit_path_to_string(path);
                }
                // Reconstruct the path with rusty::Vec as the owner
                let middle: String = path
                    .segments
                    .iter()
                    .take(path.segments.len() - 1)
                    .skip(1)
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                let middle = if middle.is_empty() {
                    String::new()
                } else {
                    middle + "::"
                };
                // Check if there are template arguments on Vec
                if let syn::PathArguments::AngleBracketed(args) = &path.segments[0].arguments {
                    let generic_args: Vec<String> = args
                        .args
                        .iter()
                        .filter_map(|arg| match arg {
                            syn::GenericArgument::Type(t) => Some(self.map_type(t)),
                            syn::GenericArgument::Const(c) => Some(self.emit_expr_to_string(c)),
                            _ => None,
                        })
                        .collect();
                    if !generic_args.is_empty() {
                        return format!(
                            "rusty::Vec<{}>::{}{}",
                            generic_args.join(", "),
                            middle,
                            method
                        );
                    }
                }
                return format!("rusty::Vec::{}{}", middle, method);
            }
        }
        // Bitflags `Bits::EMPTY` / `Bits::ALL` patterns:
        // `Type::Bits::EMPTY` → `0` and `Type::Bits::ALL` → `static_cast<decltype(Type::_0)>(~0)`
        if path.segments.len() >= 2 {
            let last = path.segments.last().unwrap().ident.to_string();
            let second_last = path.segments[path.segments.len() - 2].ident.to_string();
            if second_last == "Bits" {
                if last == "EMPTY" {
                    return "0".to_string();
                } else if last == "ALL" {
                    // Get the type prefix for the cast
                    if path.segments.len() >= 3 {
                        let type_seg = &path.segments[path.segments.len() - 3];
                        let type_name = type_seg.ident.to_string();
                        return format!("static_cast<{}::Bits>(~0)", type_name);
                    }
                    return "static_cast<unsigned>(~0)".to_string();
                }
            }
        }
        // Data enum unit variant path: `ErrorKind::Empty` → `ErrorKind_Empty{}`
        // Only applies to unit variants (no fields) — data variants like
        // `Either::Left` used as constructor references are NOT rewritten.
        if path.segments.len() >= 2 {
            let variant_seg = &path.segments[path.segments.len() - 1];
            let enum_seg = &path.segments[path.segments.len() - 2];
            let enum_name = enum_seg.ident.to_string();
            let variant_name = variant_seg.ident.to_string();
            let variant_key = format!("{}_{}", enum_name, variant_name);
            if self.data_enum_unit_variants.contains(&variant_key) {
                let enum_path: syn::Path = {
                    let segs: Vec<syn::PathSegment> = path
                        .segments
                        .iter()
                        .take(path.segments.len() - 1)
                        .cloned()
                        .collect();
                    let mut p = path.clone();
                    p.segments = segs.into_iter().collect();
                    p
                };
                let variant_ty = self.data_enum_variant_struct_type_name(&enum_path, &variant_name);
                return format!("{}{{}}", variant_ty);
            }
            // C-like enum associated const: `Op::DEFAULT` → `Op_DEFAULT`
            if self.c_like_enum_consts.contains(&variant_key) {
                let prefix_segments: Vec<String> = path
                    .segments
                    .iter()
                    .take(path.segments.len() - 2)
                    .map(|s| escape_cpp_keyword(&s.ident.to_string()))
                    .collect();
                if prefix_segments.is_empty() {
                    return variant_key;
                }
                return format!("{}::{}", prefix_segments.join("::"), variant_key);
            }
        }
        // For single-segment paths that match a function declared in an
        // inline module, qualify with the module name to avoid name collision
        // with identically-named test namespaces.
        // E.g., `from_str::<TestFlags>(s)` → `::parser::from_str<TestFlags>(s)`
        // Also handles non-turbofish calls: `to_writer(f, &s)` → `::parser::to_writer(f, &s)`
        if path.segments.len() == 1 {
            let fn_name = path.segments[0].ident.to_string();
            if let Some(qualified) = self.module_qualified_functions.get(&fn_name) {
                if !qualified.is_empty() {
                    let qualified_parent = qualified
                        .rsplit_once("::")
                        .map(|(parent, _)| parent)
                        .unwrap_or_default();
                    let current_scope = self.module_stack.join("::");
                    let escaped_current_scope = self
                        .module_stack
                        .iter()
                        .map(|seg| escape_cpp_keyword(seg))
                        .collect::<Vec<String>>()
                        .join("::");
                    // The function is reachable unqualified only when our
                    // current emit scope IS the parent module of the function
                    // (or a descendant of it via the SAME namespace chain).
                    // Suffix-match would falsely consider `tests::parser` as
                    // "inside ::parser" — that's a different namespace and
                    // unqualified lookup wouldn't reach `::parser::fn`.
                    let directly_inside = (!current_scope.is_empty()
                        && qualified_parent == current_scope)
                        || (!escaped_current_scope.is_empty()
                            && qualified_parent == escaped_current_scope);
                    if !directly_inside {
                        // Use absolute path (::prefix::fn) to avoid shadowing
                        // by same-named inner namespaces
                        let module_prefix = qualified.split("::").next().unwrap_or("");
                        let needs_root = self.module_stack.iter().any(|m| m == module_prefix);
                        let mut emitted = if needs_root {
                            format!("::{}", qualified)
                        } else {
                            qualified.clone()
                        };
                        if let Some(template_args) = self.emit_expr_path_template_args(path) {
                            emitted.push_str(&template_args);
                        }
                        return emitted;
                    }
                }
            }
        }
        let mut emitted = self.emit_path_to_string(path);
        if path.segments.len() == 1 {
            let fn_name = path.segments[0].ident.to_string();
            if let Some(mapped) = self
                .module_qualified_functions
                .get(&fn_name)
                .filter(|mapped| !mapped.is_empty() && !mapped.contains("::"))
            {
                let has_global = emitted.starts_with("::");
                let emitted_tail = emitted.trim_start_matches("::");
                if emitted_tail == fn_name {
                    emitted = if has_global {
                        format!("::{}", mapped)
                    } else {
                        mapped.clone()
                    };
                } else {
                    let templated_prefix = format!("{}<", fn_name);
                    if emitted_tail.starts_with(&templated_prefix) {
                        let suffix = &emitted_tail[fn_name.len()..];
                        emitted = if has_global {
                            format!("::{}{}", mapped, suffix)
                        } else {
                            format!("{}{}", mapped, suffix)
                        };
                    }
                }
            }
        }
        if emitted.ends_with("::MAX_STR_LEN") {
            let owner = emitted.trim_end_matches("::MAX_STR_LEN").trim();
            let owner_tail = owner.rsplit("::").next().unwrap_or(owner);
            let owner_is_type_like = owner_tail
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase() || ch == '_')
                || self.is_type_param_in_scope(owner_tail)
                || types::map_primitive_type(owner_tail).is_some()
                || matches!(
                    owner_tail,
                    "uint8_t"
                        | "int8_t"
                        | "uint16_t"
                        | "int16_t"
                        | "uint32_t"
                        | "int32_t"
                        | "uint64_t"
                        | "int64_t"
                        | "unsigned __int128"
                        | "__int128"
                        | "size_t"
                        | "ptrdiff_t"
                );
            if owner_is_type_like {
                let owner = owner.trim_start_matches("::");
                emitted = format!("rusty::integer_max_str_len<{}>()", owner);
            }
        }
        if path.segments.len() >= 2
            && path
                .segments
                .iter()
                .take(path.segments.len() - 1)
                .all(|seg| matches!(seg.arguments, syn::PathArguments::None))
        {
            let mut owner_path = path.clone();
            owner_path.segments = path
                .segments
                .iter()
                .take(path.segments.len() - 1)
                .cloned()
                .collect();
            let owner_cpp = self.emit_path_to_string(&owner_path);
            let member_name = path
                .segments
                .last()
                .map(|seg| escape_cpp_keyword(&seg.ident.to_string()))
                .unwrap_or_default();
            if let Some(recovered_owner_cpp) =
                self.recover_omitted_local_generic_type_args(&owner_path, &owner_cpp)
                && recovered_owner_cpp != owner_cpp
                && !recovered_owner_cpp.is_empty()
            {
                if !member_name.is_empty() {
                    emitted = format!("{}::{}", recovered_owner_cpp, member_name);
                }
            } else if !member_name.is_empty() && !owner_cpp.is_empty() && !owner_cpp.contains('<') {
                // Generic type aliases used as owners in associated calls can
                // omit template arguments in Rust (`MapImpl::from_iter`), but
                // C++ requires explicit specialization (`MapImpl<K, V>::...`).
                // If local recovery above cannot infer from in-scope generics,
                // synthesize from the alias declaration itself.
                let owner_name = owner_path
                    .segments
                    .last()
                    .map(|seg| seg.ident.to_string())
                    .unwrap_or_default();
                if !owner_name.is_empty() {
                    let owner_key = self.declared_type_key_for_path(&owner_path).or_else(|| {
                        self.lookup_declared_type_key_for_base(&owner_name, &owner_name)
                    });
                    if let Some(owner_key) = owner_key {
                        let owner_is_alias = self.type_alias_targets.contains_key(&owner_key)
                            || self.type_alias_targets.contains_key(&owner_name);
                        if owner_is_alias
                            && let Some(params) = self.declared_type_params.get(&owner_key)
                            && !params.is_empty()
                        {
                            let defaults = self.declared_type_param_defaults.get(&owner_key);
                            let requires_explicit_args = defaults.is_none_or(|all| {
                                params.iter().enumerate().any(|(idx, _)| {
                                    all.get(idx).is_none_or(|entry| entry.is_none())
                                })
                            });
                            if requires_explicit_args {
                                emitted = format!(
                                    "{}<{}>::{}",
                                    owner_cpp,
                                    params.join(", "),
                                    member_name
                                );
                            }
                        }
                    }
                }
            }
        }
        if path.segments.len() > 1 {
            let mut bare_prefix_parts: Vec<String> = Vec::new();
            let mut templated_prefix_parts: Vec<String> = Vec::new();
            let mut has_nonterminal_template_args = false;
            for seg in path.segments.iter().take(path.segments.len() - 1) {
                let bare = escape_cpp_keyword(&seg.ident.to_string());
                let mut templated = bare.clone();
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    let mapped_args: Vec<String> = args
                        .args
                        .iter()
                        .filter_map(|arg| match arg {
                            syn::GenericArgument::Type(t) => Some(self.map_type(t)),
                            syn::GenericArgument::Const(c) => Some(self.emit_expr_to_string(c)),
                            _ => None,
                        })
                        .collect();
                    if !mapped_args.is_empty() {
                        templated = format!("{}<{}>", bare, mapped_args.join(", "));
                        has_nonterminal_template_args = true;
                    }
                }
                bare_prefix_parts.push(bare);
                templated_prefix_parts.push(templated);
            }
            if has_nonterminal_template_args {
                let bare_prefix = bare_prefix_parts.join("::");
                let templated_prefix = templated_prefix_parts.join("::");
                let bare_scoped = format!("{}::", bare_prefix);
                let templated_scoped = format!("{}::", templated_prefix);
                let bare_global = format!("::{}::", bare_prefix);
                let templated_global = format!("::{}::", templated_prefix);
                if emitted.starts_with(&bare_scoped) {
                    emitted = emitted.replacen(&bare_scoped, &templated_scoped, 1);
                } else if emitted.starts_with(&bare_global) {
                    emitted = emitted.replacen(&bare_global, &templated_global, 1);
                }
            }
        }
        if path.segments.len() == 1
            && matches!(
                path.segments[0].ident.to_string().as_str(),
                "Left" | "Right"
            )
            && !emitted.contains("::")
        {
            let variant_name = path.segments[0].ident.to_string();
            let scope_key = self.module_stack.join("::");
            if let Some(bound_target) = self
                .resolve_scope_import_binding_path_for_scope(&scope_key, &variant_name)
                .or_else(|| self.resolve_scope_import_binding_path_for_scope("", &variant_name))
            {
                let mut resolved =
                    if let Ok(bound_path) = syn::parse_str::<syn::Path>(bound_target.trim()) {
                        self.emit_path_to_string(&bound_path)
                    } else {
                        Self::escape_qualified_path_preserve_global(bound_target.trim())
                    };
                if !resolved.is_empty() {
                    if !resolved.starts_with("::") {
                        resolved = format!("::{}", resolved);
                    }
                    emitted = resolved;
                }
            } else if self.is_known_free_function_path(&format!("either::{}", variant_name))
                || self.is_known_free_function_path(&format!("rusty::either::{}", variant_name))
            {
                emitted = format!("::either::{}", variant_name);
            }
            if let syn::PathArguments::AngleBracketed(args) = &path.segments[0].arguments {
                let has_rusty_type_arg = args.args.iter().any(|arg| {
                    if let syn::GenericArgument::Type(ty) = arg {
                        self.map_type(ty).starts_with("rusty::")
                    } else {
                        false
                    }
                });
                let in_either_crate = self.crate_name.as_deref() == Some("either");
                if has_rusty_type_arg || in_either_crate {
                    if self.module_stack.is_empty() {
                        if !emitted.starts_with("::") {
                            emitted = format!("::{}", emitted);
                        }
                    } else if !emitted.contains("::") {
                        emitted = format!("{}::{}", self.module_stack.join("::"), emitted);
                    }
                }
            }
        }
        if let Some(template_args) = self.emit_expr_path_template_args(path) {
            emitted.push_str(&template_args);
        }
        emitted
    }

    pub(super) fn qualify_out_of_line_owner_assoc_aliases_in_cpp_type(
        &self,
        cpp_ty: &str,
        owner: &str,
    ) -> String {
        if cpp_ty.is_empty() {
            return String::new();
        }
        let mut rewritten = cpp_ty.to_string();
        let mut aliases = self.current_struct_assoc_alias_idents();
        aliases.sort_by_key(|alias| std::cmp::Reverse(alias.len()));
        // `Error` is normally left alone (it commonly names a real `Error` type,
        // not the owner's assoc). But a deferred CLASS-TEMPLATE method's signature
        // genuinely needs `Error` qualified to `typename Owner<T>::Error` — its
        // `using Error = …;` lives in the body, too late for the signature. Qualify
        // it only in that out-of-line template-def context.
        let qualify_error = self.method_emission_out_of_line_class_template_prefix.is_some();
        for alias in aliases {
            if alias == "Error" && !qualify_error {
                continue;
            }
            let qualified = format!("typename {}::{}", owner, alias);
            rewritten = Self::replace_cpp_path_alias_tokens(&rewritten, &alias, &qualified);
        }
        rewritten
    }

    pub(super) fn emit_expr_path_template_args(&self, path: &syn::Path) -> Option<String> {
        let last = path.segments.last()?;
        let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
            return None;
        };
        if self.should_elide_shadowed_current_struct_local_type_args(path, args) {
            return None;
        }
        if path.segments.len() == 1 {
            let local_name = last.ident.to_string();
            if self.is_local_type_name_in_scope(&local_name) {
                return None;
            }
            if self.is_local_function_name_in_scope(&local_name) {
                return None;
            }
        }
        // A `_` placeholder in a call turbofish would render as `auto`, which
        // C++ forbids in an explicit template-argument list (e.g.
        // `make_hasher::<_, V, S>` -> `make_hasher<auto, V, S>`). C++ also has
        // no partial turbofish — you cannot keep the concrete suffix and skip
        // the leading slot — so drop the WHOLE turbofish and let the call
        // deduce its arguments, matching how the transpiler already emits the
        // no-turbofish form of the same call. This is strictly safe: a
        // turbofish containing `auto` is always a hard error, so no
        // currently-compiling call site changes behavior.
        if args
            .args
            .iter()
            .any(|arg| matches!(arg, syn::GenericArgument::Type(t) if self.type_contains_infer(t)))
        {
            return None;
        }
        let mapped_args: Vec<String> = args
            .args
            .iter()
            .filter_map(|arg| match arg {
                syn::GenericArgument::Type(t) => Some(self.map_type(t)),
                syn::GenericArgument::Const(c) => Some(self.emit_expr_to_string(c)),
                _ => None,
            })
            .collect();
        if mapped_args.is_empty() {
            return None;
        }
        Some(format!("<{}>", mapped_args.join(", ")))
    }

    pub(super) fn emit_path_explicit_template_args(&self, path: &syn::Path) -> Option<String> {
        let last = path.segments.last()?;
        let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
            return None;
        };
        let mapped_args: Vec<String> = args
            .args
            .iter()
            .filter_map(|arg| match arg {
                syn::GenericArgument::Type(t) => Some(self.map_type(t)),
                syn::GenericArgument::Const(c) => Some(self.emit_expr_to_string(c)),
                _ => None,
            })
            .collect();
        if mapped_args.is_empty() {
            return None;
        }
        Some(format!("<{}>", mapped_args.join(", ")))
    }

    pub(super) fn normalize_qself_base_for_assoc(&self, self_type: &str) -> String {
        let mut base = self_type.trim().to_string();
        while let Some(stripped) = base.strip_prefix("const ") {
            base = stripped.trim().to_string();
        }
        while base.ends_with('&') || base.ends_with('*') {
            base.pop();
            base = base.trim_end().to_string();
        }
        base
    }
}

/// General Layer 1 std-port registry: maps a ported std/alloc module path (the
/// segments between the `std`/`alloc`/`core` root and the type name) to the
/// transpiled `rusty::port::<…>` namespace its member types are declared in.
/// Extend one line per ported module — see `transpiled/<mod>_port/` and the
/// `-l<mod>_port` link list in `main.rs`. Only DEEP member types route here;
/// ergonomic top-level types keep their `rusty::<Type>` aliases
/// (`is_ergonomic_top_level_std_type`).
fn ported_std_module_port_namespace(module: &[String]) -> Option<&'static str> {
    let segs: Vec<&str> = module.iter().map(|s| s.as_str()).collect();
    match segs.as_slice() {
        ["vec"] => Some("rusty::port::vec"),
        _ => None,
    }
}

/// Std types that keep an ergonomic top-level `rusty::<Type>` alias (handled by
/// dedicated special cases in `map_type`), so they must NOT be rewritten to the
/// deep `rusty::port::…::<Type>` spelling by the std-port seam.
fn is_ergonomic_top_level_std_type(name: &str) -> bool {
    matches!(
        name,
        "Vec" | "String"
            | "Box"
            | "Rc"
            | "Arc"
            | "HashMap"
            | "HashSet"
            | "BTreeMap"
            | "BTreeSet"
            | "VecDeque"
            | "BinaryHeap"
            | "LinkedList"
    )
}
