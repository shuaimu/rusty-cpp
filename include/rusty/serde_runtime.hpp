// rusty's serde runtime: the hand-written member-tier dispatchers that
// transpiled serde code calls (`rusty::serde_rt::de::deserialize_any(d, v)`,
// `rusty::serde_rt::ser::serialize_value(v, s)` and family), the serde_test Token
// emulation they fall back to, and the forward declarations those fallbacks
// need. Until 2026-10-12 this text lived inside the transpiler as a string
// literal pasted into every crate's global module fragment (two diverged
// copies — one for the module that owns `::token`, one for everyone else);
// it is now one header, included where the literal was.
//
// Configuration, set by the includer BEFORE the include:
//   RUSTY_SERDE_TOKEN_NS     the namespace the Token fallbacks probe (default
//                            `token`: the single-TU shape, where the real
//                            tokens are defined later in the same TU). A
//                            module that does not own `::token` names a
//                            placeholder namespace instead, which keeps the
//                            `requires`-guarded fallbacks SFINAE-inert.
//   RUSTY_SERDE_TOKEN_OWNER  defined by the module that owns `::token`
//                            (serde_test): it takes the small `de` block
//                            (no Token emulation) and the elaborate `ser`
//                            block, exactly as its runtime text did.
#ifndef RUSTY_SERDE_RUNTIME_HPP
#define RUSTY_SERDE_RUNTIME_HPP

#include <rusty/rusty.hpp>

#ifndef RUSTY_SERDE_TOKEN_NS
#define RUSTY_SERDE_TOKEN_NS token
#endif

namespace RUSTY_SERDE_TOKEN_NS {
struct Token;
struct Token_Bool;
struct Token_I8;
struct Token_I16;
struct Token_I32;
struct Token_I64;
struct Token_U8;
struct Token_U16;
struct Token_U32;
struct Token_U64;
struct Token_F32;
struct Token_F64;
struct Token_Bytes;
struct Token_BorrowedBytes;
struct Token_ByteBuf;
struct Token_None;
struct Token_Some;
struct Token_Seq;
struct Token_SeqEnd {};
struct Token_Tuple;
struct Token_TupleEnd {};
struct Token_TupleStruct;
struct Token_TupleStructEnd {};
struct Token_Unit;
struct Token_UnitStruct;
struct Token_Char;
struct Token_Str;
struct Token_BorrowedStr;
struct Token_String;
inline Token_SeqEnd SeqEnd() { return Token_SeqEnd{}; }
inline Token_TupleEnd TupleEnd() { return Token_TupleEnd{}; }
inline Token_TupleStructEnd TupleStructEnd() { return Token_TupleStructEnd{}; }
}
namespace rusty {
// Declared here, defined by the runtime text the transpiler emits beside
// this include (`rusty::from_into`, `rusty::to_string`, the addr_of_temp
// family): the header is self-contained for parsing; the definitions arrive
// with the emitted module.
template<typename Target, typename Input>
Target from_into(Input&& input);
template<typename T>
std::string to_string(const T& value);
#if !defined(RUSTY_HAS_ADDR_OF_TEMP)
template<typename T>
constexpr T* addr_of_temp(T& value);
template<typename T>
const std::remove_cv_t<std::remove_reference_t<T>>* addr_of_temp(T&& value);
template<typename T>
std::remove_reference_t<T>* addr_of_temp_mut(T&& value);
#endif
}

#ifdef RUSTY_SERDE_TOKEN_OWNER
namespace rusty::serde_rt::de {
inline namespace v1 {
template<typename Seed, typename Deserializer>
decltype(auto) deserialize(Seed&& seed, Deserializer&& deserializer) {
    if constexpr (requires {
        std::forward<Seed>(seed).deserialize(std::forward<Deserializer>(deserializer));
    }) {
        return std::forward<Seed>(seed).deserialize(std::forward<Deserializer>(deserializer));
    } else if constexpr (requires {
        std::remove_cv_t<std::remove_reference_t<Seed>>::deserialize(
            std::forward<Deserializer>(deserializer));
    }) {
        using SeedType = std::remove_cv_t<std::remove_reference_t<Seed>>;
        return SeedType::deserialize(std::forward<Deserializer>(deserializer));
    } else if constexpr (requires {
        std::forward<Seed>(seed).deserialize(
            rusty::detail::deref_if_pointer_like(std::forward<Deserializer>(deserializer)));
    }) {
        return std::forward<Seed>(seed).deserialize(
            rusty::detail::deref_if_pointer_like(std::forward<Deserializer>(deserializer)));
    } else if constexpr (requires {
        std::remove_cv_t<std::remove_reference_t<Seed>>::deserialize(
            rusty::detail::deref_if_pointer_like(std::forward<Deserializer>(deserializer)));
    }) {
        using SeedType = std::remove_cv_t<std::remove_reference_t<Seed>>;
        return SeedType::deserialize(
            rusty::detail::deref_if_pointer_like(std::forward<Deserializer>(deserializer)));
    } else {
        return std::forward<Seed>(seed).deserialize(std::forward<Deserializer>(deserializer));
    }
}
template<typename Deserializer, typename Visitor>
decltype(auto) deserialize_any(Deserializer&& deserializer, Visitor&& visitor) {
    if constexpr (requires {
        std::forward<Deserializer>(deserializer)
            .deserialize_any(std::forward<Visitor>(visitor));
        requires (!std::is_void_v<decltype(std::forward<Deserializer>(deserializer)
                                            .deserialize_any(std::forward<Visitor>(visitor)))>);
    }) {
        return std::forward<Deserializer>(deserializer)
            .deserialize_any(std::forward<Visitor>(visitor));
    } else if constexpr (requires {
        rusty::detail::deref_if_pointer_like(std::forward<Deserializer>(deserializer))
            .deserialize_any(std::forward<Visitor>(visitor));
        requires (!std::is_void_v<decltype(rusty::detail::deref_if_pointer_like(
                                            std::forward<Deserializer>(deserializer))
                                            .deserialize_any(std::forward<Visitor>(visitor)))>);
    }) {
        return rusty::detail::deref_if_pointer_like(std::forward<Deserializer>(deserializer))
            .deserialize_any(std::forward<Visitor>(visitor));
    } else if constexpr (
        requires {
            typename std::remove_cv_t<std::remove_reference_t<Visitor>>::Value;
            ::rusty::serde_rt::de::deserialize(
                rusty::PhantomData<
                    typename std::remove_cv_t<std::remove_reference_t<Visitor>>::Value>{},
                std::forward<Deserializer>(deserializer));
        }) {
        using Target = typename std::remove_cv_t<std::remove_reference_t<Visitor>>::Value;
        return ::rusty::serde_rt::de::deserialize(
            rusty::PhantomData<Target>{}, std::forward<Deserializer>(deserializer));
    } else {
        return std::forward<Deserializer>(deserializer)
            .deserialize_any(std::forward<Visitor>(visitor));
    }
}
template<typename Target, typename Deserializer, typename Place>
decltype(auto) deserialize_in_place(
    rusty::PhantomData<Target>,
    Deserializer&& deserializer,
    Place&& place) {
    if constexpr (requires {
        Target::deserialize_in_place(
            std::forward<Deserializer>(deserializer), std::forward<Place>(place));
    }) {
        return Target::deserialize_in_place(
            std::forward<Deserializer>(deserializer), std::forward<Place>(place));
    } else if constexpr (requires {
        { ::rusty::serde_rt::de::deserialize(
            rusty::PhantomData<Target>{}, std::forward<Deserializer>(deserializer)) };
        ::rusty::serde_rt::de::deserialize(
            rusty::PhantomData<Target>{}, std::forward<Deserializer>(deserializer)).is_ok();
        ::rusty::serde_rt::de::deserialize(
            rusty::PhantomData<Target>{}, std::forward<Deserializer>(deserializer)).unwrap_err();
    }) {
        auto __res = ::rusty::serde_rt::de::deserialize(
            rusty::PhantomData<Target>{}, std::forward<Deserializer>(deserializer));
        using Err = std::remove_cv_t<std::remove_reference_t<decltype(__res.unwrap_err())>>;
        if (__res.is_ok()) {
            static_cast<void>(rusty::mem::replace(
                rusty::detail::deref_if_pointer_like(std::forward<Place>(place)),
                __res.unwrap()));
            return rusty::Result<std::tuple<>, Err>::Ok(std::make_tuple());
        }
        return rusty::Result<std::tuple<>, Err>::Err(__res.unwrap_err());
    } else {
        return Target::deserialize_in_place(
            std::forward<Deserializer>(deserializer), std::forward<Place>(place));
    }
}
template<typename Deserializer, typename Place>
decltype(auto) deserialize_in_place(Deserializer&& deserializer, Place&& place) {
    using Target = std::remove_cv_t<std::remove_reference_t<Place>>;
    return ::rusty::serde_rt::de::deserialize_in_place(
        rusty::PhantomData<Target>{},
        std::forward<Deserializer>(deserializer),
        std::forward<Place>(place));
}
template<typename Value, typename Deserializer, typename Place>
decltype(auto) deserialize_in_place(
    Value&& value,
    Deserializer&& deserializer,
    Place&& place) {
    if constexpr (requires {
        std::forward<Value>(value).deserialize_in_place(
            std::forward<Deserializer>(deserializer), std::forward<Place>(place));
    }) {
        return std::forward<Value>(value).deserialize_in_place(
            std::forward<Deserializer>(deserializer), std::forward<Place>(place));
    } else if constexpr (requires {
        std::remove_cv_t<std::remove_reference_t<Value>>::deserialize_in_place(
            std::forward<Deserializer>(deserializer), std::forward<Place>(place));
    }) {
        using ValueType = std::remove_cv_t<std::remove_reference_t<Value>>;
        return ValueType::deserialize_in_place(
            std::forward<Deserializer>(deserializer), std::forward<Place>(place));
    } else {
        return std::forward<Value>(value).deserialize_in_place(
            std::forward<Deserializer>(deserializer), std::forward<Place>(place));
    }
}
}
}
#else
namespace rusty::serde_rt::de {
namespace impls {
inline namespace v1 {
}
}
inline namespace v1 {
template<typename Seed, typename Deserializer>
decltype(auto) deserialize(Seed&& seed, Deserializer&& deserializer);
}
namespace detail {
template<typename T>
struct is_phantom_data : std::false_type {};
template<typename U>
struct is_phantom_data<rusty::PhantomData<U>> : std::true_type {};
template<typename T>
inline constexpr bool is_phantom_data_v =
    is_phantom_data<std::remove_cv_t<std::remove_reference_t<T>>>::value;

template<typename ValueT, typename VisitorT>
struct identifier_seed {
    using Value = ValueT;

    template<typename D>
    auto deserialize(D&& deserializer) {
        return ([&](auto&& __recv) -> decltype(auto) {
            if constexpr (requires {
                std::forward<decltype(__recv)>(__recv).deserialize_identifier(VisitorT{});
            }) {
                return std::forward<decltype(__recv)>(__recv).deserialize_identifier(VisitorT{});
            } else {
                return std::forward<decltype(__recv)>(__recv)->deserialize_identifier(VisitorT{});
            }
        })(std::forward<D>(deserializer));
    }
};

struct bytes_span_visitor {
    using Value = std::span<const uint8_t>;

    template<typename E>
    rusty::Result<Value, E> visit_bytes(std::span<const uint8_t> value) {
        return rusty::Result<Value, E>::Ok(value);
    }

    template<typename E>
    rusty::Result<Value, E> visit_borrowed_bytes(std::span<const uint8_t> value) {
        return rusty::Result<Value, E>::Ok(value);
    }

    template<typename E>
    rusty::Result<Value, E> visit_byte_buf(auto&& value) {
        return rusty::Result<Value, E>::Ok(rusty::as_u8_slice(std::forward<decltype(value)>(value)));
    }

    template<typename E>
    rusty::Result<Value, E> visit_str(std::string_view value) {
        return rusty::Result<Value, E>::Ok(rusty::as_bytes(value));
    }

    template<typename E>
    rusty::Result<Value, E> visit_borrowed_str(std::string_view value) {
        return rusty::Result<Value, E>::Ok(rusty::as_bytes(value));
    }

    template<typename E>
    rusty::Result<Value, E> visit_string(rusty::String value) {
        return rusty::Result<Value, E>::Ok(rusty::as_bytes(rusty::to_string_view(value)));
    }
};

template<typename Target>
struct scalar_visitor {
    using Value = Target;

    template<typename E, typename V>
    rusty::Result<Value, E> accept(V&& value) const {
        using RawValue = std::remove_cv_t<std::remove_reference_t<Value>>;
        using RawInput = std::remove_cv_t<std::remove_reference_t<V>>;
        if constexpr (std::is_enum_v<RawValue> && std::is_integral_v<RawInput>) {
            using Underlying = std::underlying_type_t<RawValue>;
            Underlying raw = static_cast<Underlying>(value);
            if constexpr (requires { RawValue::Other; }) {
                if (raw > static_cast<Underlying>(RawValue::Other)) {
                    return rusty::Result<Value, E>::Ok(RawValue::Other);
                }
            }
            return rusty::Result<Value, E>::Ok(static_cast<RawValue>(raw));
        } else if constexpr (std::is_constructible_v<Value, V&&>) {
            return rusty::Result<Value, E>::Ok(Value(std::forward<V>(value)));
        } else if constexpr (std::is_convertible_v<V&&, Value>) {
            return rusty::Result<Value, E>::Ok(static_cast<Value>(std::forward<V>(value)));
        } else {
            return rusty::Result<Value, E>::Err(E::custom("expected numeric token"));
        }
    }

    template<typename E>
    rusty::Result<Value, E> visit_bool(bool value) const {
        return accept<E>(value);
    }

    template<typename E>
    rusty::Result<Value, E> visit_u8(uint8_t value) const {
        return accept<E>(value);
    }

    template<typename E>
    rusty::Result<Value, E> visit_u16(uint16_t value) const {
        return accept<E>(value);
    }

    template<typename E>
    rusty::Result<Value, E> visit_u32(uint32_t value) const {
        return accept<E>(value);
    }

    template<typename E>
    rusty::Result<Value, E> visit_u64(uint64_t value) const {
        return accept<E>(value);
    }

    template<typename E>
    rusty::Result<Value, E> visit_i8(int8_t value) const {
        return accept<E>(value);
    }

    template<typename E>
    rusty::Result<Value, E> visit_i16(int16_t value) const {
        return accept<E>(value);
    }

    template<typename E>
    rusty::Result<Value, E> visit_i32(int32_t value) const {
        return accept<E>(value);
    }

    template<typename E>
    rusty::Result<Value, E> visit_i64(int64_t value) const {
        return accept<E>(value);
    }

    template<typename E>
    rusty::Result<Value, E> visit_f32(float value) const {
        return accept<E>(value);
    }

    template<typename E>
    rusty::Result<Value, E> visit_f64(double value) const {
        return accept<E>(value);
    }
};

struct unit_visitor {
    using Value = std::tuple<>;

    template<typename E>
    rusty::Result<Value, E> visit_unit() const {
        return rusty::Result<Value, E>::Ok(std::make_tuple());
    }
};

template<typename Target>
struct option_visitor {
    using Value = Target;
    using Inner = std::remove_cv_t<std::remove_reference_t<typename Target::value_type>>;

    template<typename E>
    rusty::Result<Value, E> visit_none() const {
        return rusty::Result<Value, E>::Ok(Value(rusty::None));
    }

    template<typename E>
    rusty::Result<Value, E> visit_unit() const {
        return visit_none<E>();
    }

    template<typename E, typename Deserializer>
    rusty::Result<Value, E> visit_some(Deserializer&& deserializer) const {
        auto __inner = ::rusty::serde_rt::de::deserialize(
            rusty::PhantomData<Inner>{},
            std::forward<Deserializer>(deserializer));
        if (__inner.is_err()) {
            return rusty::Result<Value, E>::Err(__inner.unwrap_err());
        }
        auto&& __inner_value = __inner.unwrap();
        if constexpr (requires { Value(std::forward<decltype(__inner_value)>(__inner_value)); }) {
            return rusty::Result<Value, E>::Ok(
                Value(std::forward<decltype(__inner_value)>(__inner_value)));
        } else {
            return rusty::Result<Value, E>::Err(E::custom("unsupported option target"));
        }
    }
};

template<typename De, typename EndToken>
struct seq_access_bridge {
    using Error = std::remove_cv_t<std::remove_reference_t<
        decltype(rusty::peek_token(std::declval<De&>()).unwrap_err())>>;

    De& de;
    rusty::Option<size_t> len;
    EndToken end;

    template<typename Seed>
    auto next_element_seed(Seed&& seed) {
        using SeedType = std::remove_cv_t<std::remove_reference_t<Seed>>;
        using Ret = rusty::Result<rusty::Option<typename SeedType::Value>, Error>;

        auto __peek_res = rusty::peek_token(rusty::detail::deref_if_pointer_like(de));
        if (__peek_res.is_err()) {
            return Ret::Err(__peek_res.unwrap_err());
        }
        auto __peek_tok = __peek_res.unwrap();
        if constexpr (requires { rusty::detail::variant_holds<EndToken>(__peek_tok); }) {
            if (rusty::detail::variant_holds<EndToken>(__peek_tok)) {
                return Ret::Ok(rusty::Option<typename SeedType::Value>(rusty::None));
            }
        }
        len = len.map([](auto&& __len) -> size_t {
            return rusty::saturating_sub(__len, static_cast<size_t>(1));
        });

        auto __value = ::rusty::serde_rt::de::deserialize(
            std::forward<Seed>(seed),
            rusty::detail::deref_if_pointer_like(de));
        return __value.map([](auto&& _v) {
            return rusty::Some(std::forward<decltype(_v)>(_v));
        });
    }

    rusty::Option<size_t> size_hint() const {
        return len;
    }
};
}
inline namespace v1 {
template<typename Deserializer, typename Visitor>
decltype(auto) deserialize_any(Deserializer&& deserializer, Visitor&& visitor);

template<typename Seed, typename Deserializer>
decltype(auto) deserialize(Seed&& seed, Deserializer&& deserializer) {
    if constexpr (requires {
        std::forward<Seed>(seed).deserialize(std::forward<Deserializer>(deserializer));
    }) {
        return std::forward<Seed>(seed).deserialize(std::forward<Deserializer>(deserializer));
    } else if constexpr (detail::is_phantom_data_v<Seed>) {
        using SeedType = std::remove_cv_t<std::remove_reference_t<Seed>>;
        using Target = typename SeedType::value_type;
        if constexpr (requires { Target::deserialize(std::forward<Deserializer>(deserializer)); }) {
            return Target::deserialize(std::forward<Deserializer>(deserializer));
        } else if constexpr (requires {
            Target::deserialize(rusty::detail::deref_if_pointer_like(
                std::forward<Deserializer>(deserializer)));
        }) {
            return Target::deserialize(rusty::detail::deref_if_pointer_like(
                std::forward<Deserializer>(deserializer)));
        } else if constexpr (std::is_reference_v<Target>) {
            using Owned = std::remove_cv_t<std::remove_reference_t<Target>>;
            auto __owned = ::rusty::serde_rt::de::deserialize(
                rusty::PhantomData<Owned>{},
                std::forward<Deserializer>(deserializer));
            using Err = std::remove_cv_t<std::remove_reference_t<
                decltype(__owned.unwrap_err())>>;
            using Ret = rusty::Result<Target, Err>;
            if (__owned.is_err()) {
                return Ret::Err(__owned.unwrap_err());
            }
            if constexpr (std::is_const_v<std::remove_reference_t<Target>>) {
                auto __bound = rusty::addr_of_temp(std::move(__owned.unwrap()));
                return Ret::Ok(static_cast<Target>(*__bound));
            } else {
                return Ret::Err(Err::custom("unsupported non-const reference target"));
            }
        } else if constexpr (
            std::is_same_v<Target, std::tuple<>>
            && requires {
                rusty::detail::deref_if_pointer_like(
                    std::forward<Deserializer>(deserializer))
                    .deserialize_unit(detail::unit_visitor{});
            }) {
            return rusty::detail::deref_if_pointer_like(
                std::forward<Deserializer>(deserializer))
                .deserialize_unit(detail::unit_visitor{});
        } else if constexpr (
            std::is_same_v<Target, std::tuple<>>
            && requires {
                rusty::next_token(rusty::detail::deref_if_pointer_like(
                    std::forward<Deserializer>(deserializer)));
            }) {
            auto __tok_res = rusty::next_token(rusty::detail::deref_if_pointer_like(
                std::forward<Deserializer>(deserializer)));
            using Err = std::remove_cv_t<std::remove_reference_t<
                decltype(__tok_res.unwrap_err())>>;
            if (__tok_res.is_err()) {
                return rusty::Result<std::tuple<>, Err>::Err(__tok_res.unwrap_err());
            }
            auto __tok = __tok_res.unwrap();
            if constexpr (requires { rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Unit>(__tok); }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Unit>(__tok)) {
                    return rusty::Result<std::tuple<>, Err>::Ok(std::make_tuple());
                }
            }
            if constexpr (requires {
                rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_UnitStruct>(__tok);
            }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_UnitStruct>(__tok)) {
                    return rusty::Result<std::tuple<>, Err>::Ok(std::make_tuple());
                }
            }
            return rusty::Result<std::tuple<>, Err>::Err(
                Err::custom("expected unit token"));
        } else if constexpr (
            (std::is_arithmetic_v<Target> || std::is_enum_v<Target> || std::is_same_v<Target, bool>)
            && requires {
                ::rusty::serde_rt::de::deserialize_any(
                    rusty::detail::deref_if_pointer_like(
                        std::forward<Deserializer>(deserializer)),
                    detail::scalar_visitor<Target>{});
            }) {
            return ::rusty::serde_rt::de::deserialize_any(
                rusty::detail::deref_if_pointer_like(
                    std::forward<Deserializer>(deserializer)),
                detail::scalar_visitor<Target>{});
        } else if constexpr (
            (std::is_arithmetic_v<Target> || std::is_enum_v<Target> || std::is_same_v<Target, bool>)
            && requires {
                rusty::next_token(rusty::detail::deref_if_pointer_like(
                    std::forward<Deserializer>(deserializer)));
            }) {
            auto __tok_res = rusty::next_token(rusty::detail::deref_if_pointer_like(
                std::forward<Deserializer>(deserializer)));
            using Err = std::remove_cv_t<std::remove_reference_t<
                decltype(__tok_res.unwrap_err())>>;
            using Ret = rusty::Result<Target, Err>;
            if (__tok_res.is_err()) {
                return Ret::Err(__tok_res.unwrap_err());
            }
            auto __tok = __tok_res.unwrap();
            auto __from = [&](auto&& __value) {
                return Ret::Ok(static_cast<Target>(__value));
            };
            if constexpr (requires {
                rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Bool>(__tok);
                rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_Bool>(__tok)._0;
            }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Bool>(__tok)) {
                    return __from(rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_Bool>(__tok)._0);
                }
            }
            if constexpr (requires {
                rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_U8>(__tok);
                rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_U8>(__tok)._0;
            }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_U8>(__tok)) {
                    return __from(rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_U8>(__tok)._0);
                }
            }
            if constexpr (requires {
                rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_U16>(__tok);
                rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_U16>(__tok)._0;
            }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_U16>(__tok)) {
                    return __from(rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_U16>(__tok)._0);
                }
            }
            if constexpr (requires {
                rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_U32>(__tok);
                rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_U32>(__tok)._0;
            }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_U32>(__tok)) {
                    return __from(rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_U32>(__tok)._0);
                }
            }
            if constexpr (requires {
                rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_U64>(__tok);
                rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_U64>(__tok)._0;
            }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_U64>(__tok)) {
                    return __from(rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_U64>(__tok)._0);
                }
            }
            if constexpr (requires {
                rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_I8>(__tok);
                rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_I8>(__tok)._0;
            }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_I8>(__tok)) {
                    return __from(rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_I8>(__tok)._0);
                }
            }
            if constexpr (requires {
                rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_I16>(__tok);
                rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_I16>(__tok)._0;
            }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_I16>(__tok)) {
                    return __from(rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_I16>(__tok)._0);
                }
            }
            if constexpr (requires {
                rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_I32>(__tok);
                rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_I32>(__tok)._0;
            }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_I32>(__tok)) {
                    return __from(rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_I32>(__tok)._0);
                }
            }
            if constexpr (requires {
                rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_I64>(__tok);
                rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_I64>(__tok)._0;
            }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_I64>(__tok)) {
                    return __from(rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_I64>(__tok)._0);
                }
            }
            if constexpr (requires {
                rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_F32>(__tok);
                rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_F32>(__tok)._0;
            }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_F32>(__tok)) {
                    return __from(rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_F32>(__tok)._0);
                }
            }
            if constexpr (requires {
                rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_F64>(__tok);
                rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_F64>(__tok)._0;
            }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_F64>(__tok)) {
                    return __from(rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_F64>(__tok)._0);
                }
            }
            return Ret::Err(Err::custom("expected numeric token"));
        } else if constexpr (
            requires {
                typename Target::value_type;
                Target(rusty::None);
                std::declval<Target&>().is_some();
                std::declval<Target&>().is_none();
                rusty::detail::deref_if_pointer_like(std::forward<Deserializer>(deserializer))
                    .deserialize_option(detail::option_visitor<Target>{});
            }) {
            return rusty::detail::deref_if_pointer_like(std::forward<Deserializer>(deserializer))
                .deserialize_option(detail::option_visitor<Target>{});
        } else if constexpr (
            requires {
                typename Target::value_type;
                Target(rusty::None);
                std::declval<Target&>().is_some();
                std::declval<Target&>().is_none();
                rusty::next_token(rusty::detail::deref_if_pointer_like(
                    std::forward<Deserializer>(deserializer)));
            }) {
            auto __tok_res = rusty::next_token(rusty::detail::deref_if_pointer_like(
                std::forward<Deserializer>(deserializer)));
            using Err = std::remove_cv_t<std::remove_reference_t<
                decltype(__tok_res.unwrap_err())>>;
            using Ret = rusty::Result<Target, Err>;
            if (__tok_res.is_err()) {
                return Ret::Err(__tok_res.unwrap_err());
            }
            auto __tok = __tok_res.unwrap();
            if constexpr (requires { rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_None>(__tok); }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_None>(__tok)) {
                    return Ret::Ok(Target(rusty::None));
                }
            }
            if constexpr (requires { rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Unit>(__tok); }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Unit>(__tok)) {
                    return Ret::Ok(Target(rusty::None));
                }
            }
            if constexpr (requires { rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Some>(__tok); }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Some>(__tok)) {
                    using Inner = std::remove_cv_t<std::remove_reference_t<
                        typename Target::value_type>>;
                    auto __inner = ::rusty::serde_rt::de::deserialize(
                        rusty::PhantomData<Inner>{},
                        std::forward<Deserializer>(deserializer));
                    if (__inner.is_err()) {
                        return Ret::Err(__inner.unwrap_err());
                    }
                    auto&& __inner_value = __inner.unwrap();
                    if constexpr (requires {
                        Target(std::forward<decltype(__inner_value)>(__inner_value));
                    }) {
                        return Ret::Ok(Target(std::forward<decltype(__inner_value)>(
                            __inner_value)));
                    } else {
                        return Ret::Err(Err::custom("unsupported option target"));
                    }
                }
            }
            return Ret::Err(Err::custom("expected option token"));
        } else if constexpr (requires { std::declval<Target&>().value; }) {
            using ValueMember = std::remove_cv_t<std::remove_reference_t<decltype(std::declval<Target&>().value)>>;
            auto __inner = ::rusty::serde_rt::de::deserialize(
                rusty::PhantomData<ValueMember>{},
                std::forward<Deserializer>(deserializer));
            using Err = std::remove_cv_t<std::remove_reference_t<
                decltype(__inner.unwrap_err())>>;
            using Ret = rusty::Result<Target, Err>;
            if (__inner.is_err()) {
                return Ret::Err(__inner.unwrap_err());
            }
            auto&& __inner_value = __inner.unwrap();
            if constexpr (requires {
                Target{.value = std::forward<decltype(__inner_value)>(__inner_value)};
            }) {
                return Ret::Ok(Target{
                    .value = std::forward<decltype(__inner_value)>(__inner_value)});
            } else if constexpr (requires {
                Target{std::forward<decltype(__inner_value)>(__inner_value)};
            }) {
                return Ret::Ok(Target{
                    std::forward<decltype(__inner_value)>(__inner_value)});
            } else {
                return Ret::Err(Err::custom("unsupported seed wrapper"));
            }
        } else if constexpr (requires {
            ::rusty::serde_rt::de::deserialize_any(
                rusty::detail::deref_if_pointer_like(std::forward<Deserializer>(deserializer)),
                detail::bytes_span_visitor{});
            requires (
                std::is_same_v<Target, std::span<const uint8_t>>
                || rusty::detail::is_std_array_type_v<std::remove_reference_t<Target>>
                || requires { Target::from(std::declval<std::span<const uint8_t>>()); }
                || requires { Target::new_(std::declval<std::span<const uint8_t>>()); });
        }) {
            auto __bytes_res = ::rusty::serde_rt::de::deserialize_any(
                rusty::detail::deref_if_pointer_like(std::forward<Deserializer>(deserializer)),
                detail::bytes_span_visitor{});
            using Err = std::remove_cv_t<std::remove_reference_t<
                decltype(__bytes_res.unwrap_err())>>;
            using Ret = rusty::Result<Target, Err>;
            if (__bytes_res.is_err()) {
                return Ret::Err(__bytes_res.unwrap_err());
            }
            auto __bytes = __bytes_res.unwrap();
            if constexpr (std::is_same_v<Target, std::span<const uint8_t>>) {
                return Ret::Ok(__bytes);
            } else if constexpr (rusty::detail::is_std_array_type_v<std::remove_reference_t<Target>>) {
                auto __arr_res = rusty::try_from<Target>(__bytes);
                if (__arr_res.is_err()) {
                    return Ret::Err(Err::custom("expected bytes token"));
                }
                return Ret::Ok(__arr_res.unwrap());
            } else if constexpr (requires { Target::from(__bytes); }) {
                return Ret::Ok(Target::from(__bytes));
            } else {
                return Ret::Ok(Target::new_(__bytes));
            }
        } else if constexpr (requires {
            { Target::new_(std::declval<std::span<const uint8_t>>()) };
            rusty::next_token(rusty::detail::deref_if_pointer_like(
                std::forward<Deserializer>(deserializer)));
        }) {
            auto __tok_res = rusty::next_token(rusty::detail::deref_if_pointer_like(
                std::forward<Deserializer>(deserializer)));
            using Err = std::remove_cv_t<std::remove_reference_t<
                decltype(__tok_res.unwrap_err())>>;
            using Ret = rusty::Result<Target, Err>;
            if (__tok_res.is_err()) {
                return Ret::Err(__tok_res.unwrap_err());
            }
            auto __tok = __tok_res.unwrap();
            if constexpr (requires {
                rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Bytes>(__tok);
                rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_Bytes>(__tok)._0;
            }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Bytes>(__tok)) {
                    return Ret::Ok(Target::new_(rusty::as_u8_slice(
                        rusty::detail::deref_if_pointer(
                            rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_Bytes>(__tok)._0))));
                }
            }
            if constexpr (requires {
                rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedBytes>(__tok);
                rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedBytes>(__tok)._0;
            }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedBytes>(__tok)) {
                    return Ret::Ok(Target::new_(rusty::as_u8_slice(
                        rusty::detail::deref_if_pointer(
                            rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedBytes>(__tok)._0))));
                }
            }
            if constexpr (requires {
                rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_ByteBuf>(__tok);
                rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_ByteBuf>(__tok)._0;
            }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_ByteBuf>(__tok)) {
                    return Ret::Ok(Target::new_(rusty::as_u8_slice(
                        rusty::detail::deref_if_pointer(
                            rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_ByteBuf>(__tok)._0))));
                }
            }
            if constexpr (requires {
                rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Str>(__tok);
                rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_Str>(__tok)._0;
            }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Str>(__tok)) {
                    return Ret::Ok(Target::new_(rusty::as_bytes(rusty::to_string_view(
                        rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_Str>(__tok)._0))));
                }
            }
            if constexpr (requires {
                rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedStr>(__tok);
                rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedStr>(__tok)._0;
            }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedStr>(__tok)) {
                    return Ret::Ok(Target::new_(rusty::as_bytes(rusty::to_string_view(
                        rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedStr>(__tok)._0))));
                }
            }
            if constexpr (requires {
                rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_String>(__tok);
                rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_String>(__tok)._0;
            }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_String>(__tok)) {
                    return Ret::Ok(Target::new_(rusty::as_bytes(rusty::to_string_view(
                        rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_String>(__tok)._0))));
                }
            }
            return Ret::Err(Err::custom("expected bytes token"));
        } else if constexpr (requires {
            rusty::next_token(rusty::detail::deref_if_pointer_like(
                std::forward<Deserializer>(deserializer)));
            requires std::is_same_v<Target, std::span<const uint8_t>>;
        }) {
            auto __tok_res = rusty::next_token(rusty::detail::deref_if_pointer_like(
                std::forward<Deserializer>(deserializer)));
            using Err = std::remove_cv_t<std::remove_reference_t<
                decltype(__tok_res.unwrap_err())>>;
            using Ret = rusty::Result<Target, Err>;
            if (__tok_res.is_err()) {
                return Ret::Err(__tok_res.unwrap_err());
            }
            auto __tok = __tok_res.unwrap();
            auto __as_target = [&](auto&& __bytes_expr) {
                return Ret::Ok(rusty::as_u8_slice(
                    std::forward<decltype(__bytes_expr)>(__bytes_expr)));
            };
            if constexpr (requires {
                rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Bytes>(__tok);
                rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_Bytes>(__tok)._0;
            }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Bytes>(__tok)) {
                    return __as_target(rusty::detail::deref_if_pointer(
                        rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_Bytes>(__tok)._0));
                }
            }
            if constexpr (requires {
                rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedBytes>(__tok);
                rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedBytes>(__tok)._0;
            }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedBytes>(__tok)) {
                    return __as_target(rusty::detail::deref_if_pointer(
                        rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedBytes>(__tok)._0));
                }
            }
            if constexpr (requires {
                rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_ByteBuf>(__tok);
                rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_ByteBuf>(__tok)._0;
            }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_ByteBuf>(__tok)) {
                    return __as_target(rusty::detail::deref_if_pointer(
                        rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_ByteBuf>(__tok)._0));
                }
            }
            if constexpr (requires {
                rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Str>(__tok);
                rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_Str>(__tok)._0;
            }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Str>(__tok)) {
                    return __as_target(rusty::as_bytes(rusty::to_string_view(
                        rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_Str>(__tok)._0)));
                }
            }
            if constexpr (requires {
                rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedStr>(__tok);
                rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedStr>(__tok)._0;
            }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedStr>(__tok)) {
                    return __as_target(rusty::as_bytes(rusty::to_string_view(
                        rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedStr>(__tok)._0)));
                }
            }
            if constexpr (requires {
                rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_String>(__tok);
                rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_String>(__tok)._0;
            }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_String>(__tok)) {
                    return __as_target(rusty::as_bytes(rusty::to_string_view(
                        rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_String>(__tok)._0)));
                }
            }
            return Ret::Err(Err::custom("expected bytes token"));
        } else if constexpr (requires {
            ::rusty::serde_rt::de::deserialize_any(
                rusty::detail::deref_if_pointer_like(std::forward<Deserializer>(deserializer)),
                detail::bytes_span_visitor{});
        }) {
            auto __bytes_res = ::rusty::serde_rt::de::deserialize_any(
                rusty::detail::deref_if_pointer_like(std::forward<Deserializer>(deserializer)),
                detail::bytes_span_visitor{});
            using Err = std::remove_cv_t<std::remove_reference_t<
                decltype(__bytes_res.unwrap_err())>>;
            using Ret = rusty::Result<Target, Err>;
            if (__bytes_res.is_err()) {
                return Ret::Err(__bytes_res.unwrap_err());
            }
            auto __bytes = __bytes_res.unwrap();
            if constexpr (std::is_same_v<Target, std::span<const uint8_t>>) {
                return Ret::Ok(__bytes);
            } else if constexpr (
                rusty::detail::is_std_array_type_v<std::remove_reference_t<Target>>) {
                auto __arr_res = rusty::try_from<Target>(__bytes);
                if (__arr_res.is_err()) {
                    return Ret::Err(Err::custom("expected bytes token"));
                }
                return Ret::Ok(__arr_res.unwrap());
            } else if constexpr (requires { Target::from(__bytes); }) {
                return Ret::Ok(Target::from(__bytes));
            } else if constexpr (requires { Target::new_(__bytes); }) {
                return Ret::Ok(Target::new_(__bytes));
            } else if constexpr (std::is_constructible_v<Target, decltype(__bytes)>) {
                return Ret::Ok(Target(__bytes));
            } else if constexpr (std::is_convertible_v<decltype(__bytes), Target>) {
                return Ret::Ok(static_cast<Target>(__bytes));
            } else if constexpr (requires {
                typename std::variant_alternative_t<
                    0,
                    std::remove_cv_t<std::remove_reference_t<Target>>>;
            }) {
                using VariantTarget = std::remove_cv_t<std::remove_reference_t<Target>>;
                using Alt0 = std::variant_alternative_t<0, VariantTarget>;
                if constexpr (std::is_constructible_v<Alt0, decltype(__bytes)>) {
                    return Ret::Ok(VariantTarget{Alt0(__bytes)});
                } else if constexpr (
                    std::is_constructible_v<Alt0, decltype(rusty::to_vec(__bytes))>) {
                    return Ret::Ok(VariantTarget{Alt0(rusty::to_vec(__bytes))});
                } else if constexpr (
                    std::is_constructible_v<Alt0, std::span<const uint8_t>>) {
                    return Ret::Ok(VariantTarget{Alt0(rusty::as_u8_slice(__bytes))});
                }
            } else if constexpr (requires { Target::from(rusty::to_vec(__bytes)); }) {
                return Ret::Ok(Target::from(rusty::to_vec(__bytes)));
            } else if constexpr (requires { Target::new_(rusty::to_vec(__bytes)); }) {
                return Ret::Ok(Target::new_(rusty::to_vec(__bytes)));
            } else if constexpr (
                std::is_constructible_v<Target, decltype(rusty::to_vec(__bytes))>) {
                return Ret::Ok(Target(rusty::to_vec(__bytes)));
            } else if constexpr (
                std::is_convertible_v<decltype(rusty::to_vec(__bytes)), Target>) {
                return Ret::Ok(static_cast<Target>(rusty::to_vec(__bytes)));
            } else if constexpr (requires {
                Target::from(rusty::into_boxed_slice(rusty::to_vec(__bytes)));
            }) {
                return Ret::Ok(Target::from(rusty::into_boxed_slice(rusty::to_vec(__bytes))));
            } else if constexpr (requires {
                Target::new_(rusty::into_boxed_slice(rusty::to_vec(__bytes)));
            }) {
                return Ret::Ok(Target::new_(rusty::into_boxed_slice(rusty::to_vec(__bytes))));
            } else if constexpr (
                std::is_constructible_v<
                    Target,
                    decltype(rusty::into_boxed_slice(rusty::to_vec(__bytes)))>) {
                return Ret::Ok(Target(rusty::into_boxed_slice(rusty::to_vec(__bytes))));
            } else if constexpr (
                std::is_convertible_v<
                    decltype(rusty::into_boxed_slice(rusty::to_vec(__bytes))),
                    Target>) {
                return Ret::Ok(static_cast<Target>(
                    rusty::into_boxed_slice(rusty::to_vec(__bytes))));
            } else if constexpr (
                requires { *std::declval<Target&>(); }
                && requires {
                    Target::new_(
                        std::declval<std::remove_cvref_t<
                            decltype(*std::declval<Target&>())>>());
                }) {
                using Inner = std::remove_cv_t<std::remove_reference_t<
                    decltype(*std::declval<Target&>())>>;
                if constexpr (std::is_same_v<Inner, std::span<uint8_t>>) {
                    return Ret::Ok(rusty::into_boxed_slice(rusty::to_vec(__bytes)));
                } else if constexpr (std::is_same_v<Inner, std::span<const uint8_t>>) {
                    return Ret::Ok(Target::new_(rusty::as_u8_slice(__bytes)));
                } else if constexpr (rusty::detail::is_std_array_type_v<Inner>) {
                    auto __arr_res = rusty::try_from<Inner>(__bytes);
                    if (__arr_res.is_err()) {
                        return Ret::Err(Err::custom("expected bytes token"));
                    }
                    return Ret::Ok(Target::new_(__arr_res.unwrap()));
                } else if constexpr (requires { Inner::from(__bytes); }) {
                    return Ret::Ok(Target::new_(Inner::from(__bytes)));
                } else if constexpr (requires { Inner::new_(__bytes); }) {
                    return Ret::Ok(Target::new_(Inner::new_(__bytes)));
                } else if constexpr (std::is_constructible_v<Inner, decltype(__bytes)>) {
                    return Ret::Ok(Target::new_(Inner(__bytes)));
                } else if constexpr (std::is_convertible_v<decltype(__bytes), Inner>) {
                    return Ret::Ok(Target::new_(static_cast<Inner>(__bytes)));
                } else if constexpr (requires { Inner::from(rusty::to_vec(__bytes)); }) {
                    return Ret::Ok(Target::new_(Inner::from(rusty::to_vec(__bytes))));
                } else if constexpr (requires { Inner::new_(rusty::to_vec(__bytes)); }) {
                    return Ret::Ok(Target::new_(Inner::new_(rusty::to_vec(__bytes))));
                } else if constexpr (
                    std::is_constructible_v<Inner, decltype(rusty::to_vec(__bytes))>) {
                    return Ret::Ok(Target::new_(Inner(rusty::to_vec(__bytes))));
                } else if constexpr (
                    std::is_convertible_v<decltype(rusty::to_vec(__bytes)), Inner>) {
                    return Ret::Ok(Target::new_(static_cast<Inner>(rusty::to_vec(__bytes))));
                } else {
                    return Ret::Err(Err::custom(std::format("unsupported bytes target: {0}", typeid(Target).name())));
                }
            } else {
                return Ret::Err(Err::custom(std::format("unsupported bytes target: {0}", typeid(Target).name())));
            }
        } else if constexpr (requires { std::declval<Target&>().value; }) {
            using ValueMember = std::remove_cv_t<std::remove_reference_t<decltype(std::declval<Target&>().value)>>;
            auto __inner = ::rusty::serde_rt::de::deserialize(
                rusty::PhantomData<ValueMember>{},
                std::forward<Deserializer>(deserializer));
            using Err = std::remove_cv_t<std::remove_reference_t<
                decltype(__inner.unwrap_err())>>;
            using Ret = rusty::Result<Target, Err>;
            if (__inner.is_err()) {
                return Ret::Err(__inner.unwrap_err());
            }
            auto&& __inner_value = __inner.unwrap();
            if constexpr (requires {
                Target{.value = std::forward<decltype(__inner_value)>(__inner_value)};
            }) {
                return Ret::Ok(Target{
                    .value = std::forward<decltype(__inner_value)>(__inner_value)});
            } else if constexpr (requires {
                Target{std::forward<decltype(__inner_value)>(__inner_value)};
            }) {
                return Ret::Ok(Target{
                    std::forward<decltype(__inner_value)>(__inner_value)});
            } else {
                return Ret::Err(Err::custom("unsupported seed wrapper"));
            }
        } else {
            return std::forward<Seed>(seed).deserialize(std::forward<Deserializer>(deserializer));
        }
    } else if constexpr (requires {
        std::remove_cv_t<std::remove_reference_t<Seed>>::deserialize(
            std::forward<Deserializer>(deserializer));
    }) {
        using SeedType = std::remove_cv_t<std::remove_reference_t<Seed>>;
        return SeedType::deserialize(std::forward<Deserializer>(deserializer));
    } else if constexpr (requires {
        std::remove_cv_t<std::remove_reference_t<Seed>>::deserialize(
            rusty::detail::deref_if_pointer_like(std::forward<Deserializer>(deserializer)));
    }) {
        using SeedType = std::remove_cv_t<std::remove_reference_t<Seed>>;
        return SeedType::deserialize(
            rusty::detail::deref_if_pointer_like(std::forward<Deserializer>(deserializer)));
    } else {
        return std::forward<Seed>(seed).deserialize(std::forward<Deserializer>(deserializer));
    }
}
template<typename Deserializer, typename Visitor>
decltype(auto) deserialize_any(Deserializer&& deserializer, Visitor&& visitor) {
    if constexpr (requires {
        std::forward<Deserializer>(deserializer)
            .deserialize_any(std::forward<Visitor>(visitor));
        requires (!std::is_void_v<decltype(std::forward<Deserializer>(deserializer)
                                            .deserialize_any(std::forward<Visitor>(visitor)))>);
    }) {
        return std::forward<Deserializer>(deserializer).deserialize_any(std::forward<Visitor>(visitor));
    } else if constexpr (requires {
        rusty::detail::deref_if_pointer_like(std::forward<Deserializer>(deserializer))
            .deserialize_any(std::forward<Visitor>(visitor));
        requires (!std::is_void_v<decltype(rusty::detail::deref_if_pointer_like(
                                            std::forward<Deserializer>(deserializer))
                                            .deserialize_any(std::forward<Visitor>(visitor)))>);
    }) {
        return rusty::detail::deref_if_pointer_like(std::forward<Deserializer>(deserializer))
            .deserialize_any(std::forward<Visitor>(visitor));
    } else if constexpr (requires {
        typename std::remove_cv_t<std::remove_reference_t<Visitor>>::Value;
        rusty::next_token(rusty::detail::deref_if_pointer_like(
            std::forward<Deserializer>(deserializer)));
    }) {
        using Value = typename std::remove_cv_t<std::remove_reference_t<Visitor>>::Value;
        auto __tok_res = rusty::next_token(rusty::detail::deref_if_pointer_like(
            std::forward<Deserializer>(deserializer)));
        using Err = std::remove_cv_t<std::remove_reference_t<
            decltype(__tok_res.unwrap_err())>>;
        using Ret = rusty::Result<Value, Err>;
        if (__tok_res.is_err()) {
            return Ret::Err(__tok_res.unwrap_err());
        }
        auto __tok = __tok_res.unwrap();
        if constexpr (requires {
            rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Unit>(__tok);
            std::forward<Visitor>(visitor).visit_unit();
        }) {
            if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Unit>(__tok)) {
                return std::forward<Visitor>(visitor).visit_unit();
            }
        }
        if constexpr (requires {
            rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_UnitStruct>(__tok);
            std::forward<Visitor>(visitor).visit_unit();
        }) {
            if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_UnitStruct>(__tok)) {
                return std::forward<Visitor>(visitor).visit_unit();
            }
        }
        if constexpr (requires {
            rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Bytes>(__tok);
            rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_Bytes>(__tok)._0;
        }) {
            if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Bytes>(__tok)) {
                auto __bytes = rusty::as_u8_slice(
                    rusty::detail::deref_if_pointer(rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_Bytes>(__tok)._0));
                if constexpr (requires {
                    std::forward<Visitor>(visitor).template visit_bytes<Err>(__bytes);
                }) {
                    return std::forward<Visitor>(visitor).template visit_bytes<Err>(__bytes);
                } else if constexpr (requires {
                    std::forward<Visitor>(visitor).visit_bytes(__bytes);
                }) {
                    return std::forward<Visitor>(visitor).visit_bytes(__bytes);
                }
            }
        }
        if constexpr (requires {
            rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedBytes>(__tok);
            rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedBytes>(__tok)._0;
        }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedBytes>(__tok)) {
                    auto __bytes = rusty::as_u8_slice(rusty::detail::deref_if_pointer(
                        rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedBytes>(__tok)._0));
                    if constexpr (requires {
                        std::forward<Visitor>(visitor).template visit_borrowed_bytes<Err>(__bytes);
                    }) {
                        return std::forward<Visitor>(visitor).template visit_borrowed_bytes<Err>(__bytes);
                    } else if constexpr (requires {
                        std::forward<Visitor>(visitor).visit_borrowed_bytes(__bytes);
                    }) {
                        return std::forward<Visitor>(visitor).visit_borrowed_bytes(__bytes);
                    } else if constexpr (requires {
                        std::forward<Visitor>(visitor).template visit_bytes<Err>(__bytes);
                    }) {
                        return std::forward<Visitor>(visitor).template visit_bytes<Err>(__bytes);
                    } else if constexpr (requires {
                        std::forward<Visitor>(visitor).visit_bytes(__bytes);
                    }) {
                        return std::forward<Visitor>(visitor).visit_bytes(__bytes);
                    }
            }
        }
        if constexpr (requires {
            rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_ByteBuf>(__tok);
            rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_ByteBuf>(__tok)._0;
        }) {
                if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_ByteBuf>(__tok)) {
                    auto&& __bytes = rusty::detail::deref_if_pointer(
                        rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_ByteBuf>(__tok)._0);
                    if constexpr (requires {
                        std::forward<Visitor>(visitor).template visit_byte_buf<Err>(__bytes);
                    }) {
                        return std::forward<Visitor>(visitor).template visit_byte_buf<Err>(__bytes);
                    } else if constexpr (requires {
                        std::forward<Visitor>(visitor).visit_byte_buf(__bytes);
                    }) {
                        return std::forward<Visitor>(visitor).visit_byte_buf(__bytes);
                    } else if constexpr (requires {
                        std::forward<Visitor>(visitor).template visit_bytes<Err>(rusty::as_u8_slice(__bytes));
                    }) {
                        return std::forward<Visitor>(visitor).template visit_bytes<Err>(rusty::as_u8_slice(__bytes));
                    } else if constexpr (requires {
                        std::forward<Visitor>(visitor).visit_bytes(rusty::as_u8_slice(__bytes));
                    }) {
                        return std::forward<Visitor>(visitor).visit_bytes(rusty::as_u8_slice(__bytes));
                    }
            }
        }
        if constexpr (requires {
            rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Seq>(__tok);
            rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_Seq>(__tok).len;
        }) {
            if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Seq>(__tok)) {
                auto __len = rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_Seq>(__tok).len;
                if constexpr (requires {
                    std::forward<Visitor>(visitor).visit_seq(
                        ::rusty::serde_rt::de::detail::seq_access_bridge<
                            std::remove_cv_t<std::remove_reference_t<
                                decltype(rusty::detail::deref_if_pointer_like(deserializer))>>,
                            ::RUSTY_SERDE_TOKEN_NS::Token_SeqEnd>{
                            .de = rusty::detail::deref_if_pointer_like(deserializer),
                            .len = __len,
                            .end = ::RUSTY_SERDE_TOKEN_NS::SeqEnd()});
                }) {
                    auto __ret = std::forward<Visitor>(visitor).visit_seq(
                        ::rusty::serde_rt::de::detail::seq_access_bridge<
                            std::remove_cv_t<std::remove_reference_t<
                                decltype(rusty::detail::deref_if_pointer_like(deserializer))>>,
                            ::RUSTY_SERDE_TOKEN_NS::Token_SeqEnd>{
                            .de = rusty::detail::deref_if_pointer_like(deserializer),
                            .len = __len,
                            .end = ::RUSTY_SERDE_TOKEN_NS::SeqEnd()});
                    if constexpr (requires {
                        __ret.is_ok();
                        rusty::peek_token(rusty::detail::deref_if_pointer_like(deserializer));
                    }) {
                        if (__ret.is_ok()) {
                            auto __peek = rusty::peek_token(
                                rusty::detail::deref_if_pointer_like(deserializer));
                            if (__peek.is_ok()) {
                                auto __peek_tok = __peek.unwrap();
                                if constexpr (requires {
                                    rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_SeqEnd>(__peek_tok);
                                }) {
                                    if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_SeqEnd>(__peek_tok)) {
                                        static_cast<void>(rusty::next_token(
                                            rusty::detail::deref_if_pointer_like(deserializer)));
                                    }
                                }
                            }
                        }
                    }
                    return __ret;
                }
            }
        }
        if constexpr (requires {
            rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Tuple>(__tok);
            rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_Tuple>(__tok).len;
        }) {
            if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Tuple>(__tok)) {
                auto __len = rusty::Option<size_t>(
                    rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_Tuple>(__tok).len);
                if constexpr (requires {
                    std::forward<Visitor>(visitor).visit_seq(
                        ::rusty::serde_rt::de::detail::seq_access_bridge<
                            std::remove_cv_t<std::remove_reference_t<
                                decltype(rusty::detail::deref_if_pointer_like(deserializer))>>,
                            ::RUSTY_SERDE_TOKEN_NS::Token_TupleEnd>{
                            .de = rusty::detail::deref_if_pointer_like(deserializer),
                            .len = __len,
                            .end = ::RUSTY_SERDE_TOKEN_NS::TupleEnd()});
                }) {
                    auto __ret = std::forward<Visitor>(visitor).visit_seq(
                        ::rusty::serde_rt::de::detail::seq_access_bridge<
                            std::remove_cv_t<std::remove_reference_t<
                                decltype(rusty::detail::deref_if_pointer_like(deserializer))>>,
                            ::RUSTY_SERDE_TOKEN_NS::Token_TupleEnd>{
                            .de = rusty::detail::deref_if_pointer_like(deserializer),
                            .len = __len,
                            .end = ::RUSTY_SERDE_TOKEN_NS::TupleEnd()});
                    if constexpr (requires {
                        __ret.is_ok();
                        rusty::peek_token(rusty::detail::deref_if_pointer_like(deserializer));
                    }) {
                        if (__ret.is_ok()) {
                            auto __peek = rusty::peek_token(
                                rusty::detail::deref_if_pointer_like(deserializer));
                            if (__peek.is_ok()) {
                                auto __peek_tok = __peek.unwrap();
                                if constexpr (requires {
                                    rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_TupleEnd>(__peek_tok);
                                }) {
                                    if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_TupleEnd>(__peek_tok)) {
                                        static_cast<void>(rusty::next_token(
                                            rusty::detail::deref_if_pointer_like(deserializer)));
                                    }
                                }
                            }
                        }
                    }
                    return __ret;
                }
            }
        }
        if constexpr (requires {
            rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_TupleStruct>(__tok);
            rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_TupleStruct>(__tok).len;
        }) {
            if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_TupleStruct>(__tok)) {
                auto __len = rusty::Option<size_t>(
                    rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_TupleStruct>(__tok).len);
                if constexpr (requires {
                    std::forward<Visitor>(visitor).visit_seq(
                        ::rusty::serde_rt::de::detail::seq_access_bridge<
                            std::remove_cv_t<std::remove_reference_t<
                                decltype(rusty::detail::deref_if_pointer_like(deserializer))>>,
                            ::RUSTY_SERDE_TOKEN_NS::Token_TupleStructEnd>{
                            .de = rusty::detail::deref_if_pointer_like(deserializer),
                            .len = __len,
                            .end = ::RUSTY_SERDE_TOKEN_NS::TupleStructEnd()});
                }) {
                    auto __ret = std::forward<Visitor>(visitor).visit_seq(
                        ::rusty::serde_rt::de::detail::seq_access_bridge<
                            std::remove_cv_t<std::remove_reference_t<
                                decltype(rusty::detail::deref_if_pointer_like(deserializer))>>,
                            ::RUSTY_SERDE_TOKEN_NS::Token_TupleStructEnd>{
                            .de = rusty::detail::deref_if_pointer_like(deserializer),
                            .len = __len,
                            .end = ::RUSTY_SERDE_TOKEN_NS::TupleStructEnd()});
                    if constexpr (requires {
                        __ret.is_ok();
                        rusty::peek_token(rusty::detail::deref_if_pointer_like(deserializer));
                    }) {
                        if (__ret.is_ok()) {
                            auto __peek = rusty::peek_token(
                                rusty::detail::deref_if_pointer_like(deserializer));
                            if (__peek.is_ok()) {
                                auto __peek_tok = __peek.unwrap();
                                if constexpr (requires {
                                    rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_TupleStructEnd>(__peek_tok);
                                }) {
                                    if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_TupleStructEnd>(__peek_tok)) {
                                        static_cast<void>(rusty::next_token(
                                            rusty::detail::deref_if_pointer_like(deserializer)));
                                    }
                                }
                            }
                        }
                    }
                    return __ret;
                }
            }
        }
        if constexpr (requires {
            rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Str>(__tok);
            rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_Str>(__tok)._0;
        }) {
            if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Str>(__tok)) {
                auto __s = rusty::to_string_view(rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_Str>(__tok)._0);
                auto __bytes = rusty::as_bytes(__s);
                if constexpr (requires {
                    std::forward<Visitor>(visitor).template visit_bytes<Err>(__bytes);
                }) {
                    return std::forward<Visitor>(visitor).template visit_bytes<Err>(__bytes);
                } else if constexpr (requires {
                    std::forward<Visitor>(visitor).visit_bytes(__bytes);
                }) {
                    return std::forward<Visitor>(visitor).visit_bytes(__bytes);
                }
                if constexpr (requires {
                    std::forward<Visitor>(visitor).template visit_str<Err>(__s);
                }) {
                    return std::forward<Visitor>(visitor).template visit_str<Err>(__s);
                } else if constexpr (requires {
                    std::forward<Visitor>(visitor).visit_str(__s);
                }) {
                    return std::forward<Visitor>(visitor).visit_str(__s);
                }
            }
        }
        if constexpr (requires {
            rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedStr>(__tok);
            rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedStr>(__tok)._0;
        }) {
            if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedStr>(__tok)) {
                auto __s = rusty::to_string_view(rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedStr>(__tok)._0);
                auto __bytes = rusty::as_bytes(__s);
                if constexpr (requires {
                    std::forward<Visitor>(visitor).template visit_borrowed_bytes<Err>(__bytes);
                }) {
                    return std::forward<Visitor>(visitor).template visit_borrowed_bytes<Err>(__bytes);
                } else if constexpr (requires {
                    std::forward<Visitor>(visitor).visit_borrowed_bytes(__bytes);
                }) {
                    return std::forward<Visitor>(visitor).visit_borrowed_bytes(__bytes);
                } else if constexpr (requires {
                    std::forward<Visitor>(visitor).template visit_bytes<Err>(__bytes);
                }) {
                    return std::forward<Visitor>(visitor).template visit_bytes<Err>(__bytes);
                } else if constexpr (requires {
                    std::forward<Visitor>(visitor).visit_bytes(__bytes);
                }) {
                    return std::forward<Visitor>(visitor).visit_bytes(__bytes);
                }
                if constexpr (requires {
                    std::forward<Visitor>(visitor).template visit_borrowed_str<Err>(__s);
                }) {
                    return std::forward<Visitor>(visitor).template visit_borrowed_str<Err>(__s);
                } else if constexpr (requires {
                    std::forward<Visitor>(visitor).visit_borrowed_str(__s);
                }) {
                    return std::forward<Visitor>(visitor).visit_borrowed_str(__s);
                } else if constexpr (requires {
                    std::forward<Visitor>(visitor).template visit_str<Err>(__s);
                }) {
                    return std::forward<Visitor>(visitor).template visit_str<Err>(__s);
                } else if constexpr (requires {
                    std::forward<Visitor>(visitor).visit_str(__s);
                }) {
                    return std::forward<Visitor>(visitor).visit_str(__s);
                }
            }
        }
        if constexpr (requires {
            rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_String>(__tok);
            rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_String>(__tok)._0;
        }) {
            if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_String>(__tok)) {
                auto&& __s = rusty::detail::deref_if_pointer(
                    rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_String>(__tok)._0);
                if constexpr (requires {
                    std::forward<Visitor>(visitor).template visit_string<Err>(__s);
                }) {
                    return std::forward<Visitor>(visitor).template visit_string<Err>(__s);
                } else if constexpr (requires {
                    std::forward<Visitor>(visitor).visit_string(__s);
                }) {
                    return std::forward<Visitor>(visitor).visit_string(__s);
                } else if constexpr (requires {
                    std::forward<Visitor>(visitor).template visit_bytes<Err>(rusty::as_bytes(rusty::to_string_view(__s)));
                }) {
                    return std::forward<Visitor>(visitor).template visit_bytes<Err>(rusty::as_bytes(rusty::to_string_view(__s)));
                } else if constexpr (requires {
                    std::forward<Visitor>(visitor).visit_bytes(rusty::as_bytes(rusty::to_string_view(__s)));
                }) {
                    return std::forward<Visitor>(visitor).visit_bytes(rusty::as_bytes(rusty::to_string_view(__s)));
                } else if constexpr (requires {
                    std::forward<Visitor>(visitor).template visit_str<Err>(rusty::to_string_view(__s));
                }) {
                    return std::forward<Visitor>(visitor).template visit_str<Err>(rusty::to_string_view(__s));
                } else if constexpr (requires {
                    std::forward<Visitor>(visitor).visit_str(rusty::to_string_view(__s));
                }) {
                    return std::forward<Visitor>(visitor).visit_str(rusty::to_string_view(__s));
                }
            }
        }
        return Ret::Err(Err::custom(std::format(
            "unsupported token for deserialize_any: {0}",
            rusty::to_string(__tok))));
    } else if constexpr (requires {
        typename std::remove_cv_t<std::remove_reference_t<Visitor>>::Value;
        ::rusty::serde_rt::de::deserialize(
            rusty::PhantomData<typename std::remove_cv_t<std::remove_reference_t<Visitor>>::Value>{},
            std::forward<Deserializer>(deserializer));
    }) {
        using Target = typename std::remove_cv_t<std::remove_reference_t<Visitor>>::Value;
        return ::rusty::serde_rt::de::deserialize(
            rusty::PhantomData<Target>{}, std::forward<Deserializer>(deserializer));
    } else {
        return std::forward<Deserializer>(deserializer).deserialize_any(std::forward<Visitor>(visitor));
    }
}
template<typename Target, typename Deserializer, typename Place>
decltype(auto) deserialize_in_place(
    rusty::PhantomData<Target>,
    Deserializer&& deserializer,
    Place&& place) {
    if constexpr (requires {
        Target::deserialize_in_place(
            std::forward<Deserializer>(deserializer), std::forward<Place>(place));
    }) {
        return Target::deserialize_in_place(
            std::forward<Deserializer>(deserializer), std::forward<Place>(place));
    } else if constexpr (requires {
        { ::rusty::serde_rt::de::deserialize(
            rusty::PhantomData<Target>{}, std::forward<Deserializer>(deserializer)) };
        ::rusty::serde_rt::de::deserialize(
            rusty::PhantomData<Target>{}, std::forward<Deserializer>(deserializer)).is_ok();
        ::rusty::serde_rt::de::deserialize(
            rusty::PhantomData<Target>{}, std::forward<Deserializer>(deserializer)).unwrap_err();
    }) {
        auto __res = ::rusty::serde_rt::de::deserialize(
            rusty::PhantomData<Target>{}, std::forward<Deserializer>(deserializer));
        using Err = std::remove_cv_t<std::remove_reference_t<decltype(__res.unwrap_err())>>;
        if (__res.is_ok()) {
            static_cast<void>(rusty::mem::replace(
                rusty::detail::deref_if_pointer_like(std::forward<Place>(place)),
                __res.unwrap()));
            return rusty::Result<std::tuple<>, Err>::Ok(std::make_tuple());
        }
        return rusty::Result<std::tuple<>, Err>::Err(__res.unwrap_err());
    } else {
        return Target::deserialize_in_place(
            std::forward<Deserializer>(deserializer), std::forward<Place>(place));
    }
}
template<typename Deserializer, typename Place>
decltype(auto) deserialize_in_place(Deserializer&& deserializer, Place&& place) {
    using Target = std::remove_cv_t<std::remove_reference_t<Place>>;
    return ::rusty::serde_rt::de::deserialize_in_place(
        rusty::PhantomData<Target>{},
        std::forward<Deserializer>(deserializer),
        std::forward<Place>(place));
}
template<typename Value, typename Deserializer, typename Place>
decltype(auto) deserialize_in_place(
    Value&& value,
    Deserializer&& deserializer,
    Place&& place) {
    if constexpr (requires {
        std::forward<Value>(value).deserialize_in_place(
            std::forward<Deserializer>(deserializer), std::forward<Place>(place));
    }) {
        return std::forward<Value>(value).deserialize_in_place(
            std::forward<Deserializer>(deserializer), std::forward<Place>(place));
    } else if constexpr (requires {
        std::remove_cv_t<std::remove_reference_t<Value>>::deserialize_in_place(
            std::forward<Deserializer>(deserializer), std::forward<Place>(place));
    }) {
        using ValueType = std::remove_cv_t<std::remove_reference_t<Value>>;
        return ValueType::deserialize_in_place(
            std::forward<Deserializer>(deserializer), std::forward<Place>(place));
    } else {
        return std::forward<Value>(value).deserialize_in_place(
            std::forward<Deserializer>(deserializer), std::forward<Place>(place));
    }
}
}
namespace impls {
inline namespace v1 {
template<typename Seed, typename Deserializer>
decltype(auto) deserialize(Seed&& seed, Deserializer&& deserializer) {
    return ::rusty::serde_rt::de::deserialize(
        std::forward<Seed>(seed), std::forward<Deserializer>(deserializer));
}
template<typename Value, typename Deserializer, typename Place>
decltype(auto) deserialize_in_place(
    Value&& value,
    Deserializer&& deserializer,
    Place&& place) {
    return ::rusty::serde_rt::de::deserialize_in_place(
        std::forward<Value>(value),
        std::forward<Deserializer>(deserializer),
        std::forward<Place>(place));
}
}
}
}
#endif

namespace rusty::serde_rt::ser {
namespace impls {
inline namespace v1 {
}
}
inline namespace v1 {
namespace detail {
template<typename T>
struct is_std_span : std::false_type {};
template<typename Elem, std::size_t Extent>
struct is_std_span<std::span<Elem, Extent>> : std::true_type {};
template<typename T>
inline constexpr bool is_std_span_v =
    is_std_span<std::remove_cv_t<std::remove_reference_t<T>>>::value;
}
template<typename Serializer>
struct serializer_ref {
    using serializer_ref_marker = void;
    using Target = std::remove_cv_t<std::remove_reference_t<Serializer>>;
    Target* ptr;
    using Ok = typename Target::Ok;
    using Error = typename Target::Error;
    using SerializeSeq = std::conditional_t<
        std::is_same_v<typename Target::SerializeSeq, Target>,
        serializer_ref<Serializer>,
        typename Target::SerializeSeq>;
    using SerializeTuple = std::conditional_t<
        std::is_same_v<typename Target::SerializeTuple, Target>,
        serializer_ref<Serializer>,
        typename Target::SerializeTuple>;
    using SerializeTupleStruct = std::conditional_t<
        std::is_same_v<typename Target::SerializeTupleStruct, Target>,
        serializer_ref<Serializer>,
        typename Target::SerializeTupleStruct>;
    using SerializeTupleVariant = std::conditional_t<
        std::is_same_v<typename Target::SerializeTupleVariant, Target>,
        serializer_ref<Serializer>,
        typename Target::SerializeTupleVariant>;
    using SerializeMap = std::conditional_t<
        std::is_same_v<typename Target::SerializeMap, Target>,
        serializer_ref<Serializer>,
        typename Target::SerializeMap>;
    using SerializeStruct = std::conditional_t<
        std::is_same_v<typename Target::SerializeStruct, Target>,
        serializer_ref<Serializer>,
        typename Target::SerializeStruct>;
    using SerializeStructVariant = std::conditional_t<
        std::is_same_v<typename Target::SerializeStructVariant, Target>,
        serializer_ref<Serializer>,
        typename Target::SerializeStructVariant>;
    template<typename ResultLike>
    static decltype(auto) rebind_self_result(Target* __ptr, ResultLike&& __result) {
        using StoredResult = std::remove_cv_t<std::remove_reference_t<ResultLike>>;
        if constexpr (
            requires (StoredResult __state) {
                __state.is_ok();
                __state.is_err();
                __state.unwrap();
                __state.unwrap_err();
            }
            && std::is_same_v<
                std::remove_cv_t<std::remove_reference_t<decltype(
                    std::declval<StoredResult&>().unwrap())>>,
                Target>
        ) {
            auto __state = std::forward<ResultLike>(__result);
            using Err = std::remove_cv_t<std::remove_reference_t<
                decltype(std::declval<StoredResult&>().unwrap_err())>>;
            using Ret = rusty::Result<serializer_ref<Serializer>, Err>;
            if (__state.is_err()) {
                return Ret::Err(__state.unwrap_err());
            }
            static_cast<void>(__state.unwrap());
            return Ret::Ok(serializer_ref<Serializer>{__ptr});
        } else {
            auto __state = std::forward<ResultLike>(__result);
            return __state;
        }
    }
    Target& operator*() { return *ptr; }
    const Target& operator*() const { return *ptr; }
    Target* operator->() { return ptr; }
    const Target* operator->() const { return ptr; }
    template<typename T>
    requires requires (Target& __target, T&& __value) {
        __target.serialize_str(std::forward<T>(__value));
    }
    decltype(auto) serialize_str(T&& value) const {
        return ptr->serialize_str(std::forward<T>(value));
    }
    template<typename T>
    requires requires (Target& __target, T&& __value) {
        __target.serialize_bool(std::forward<T>(__value));
    }
    decltype(auto) serialize_bool(T&& value) const {
        return ptr->serialize_bool(std::forward<T>(value));
    }
    template<typename T>
    requires requires (Target& __target, T&& __value) {
        __target.serialize_i8(std::forward<T>(__value));
    }
    decltype(auto) serialize_i8(T&& value) const {
        return ptr->serialize_i8(std::forward<T>(value));
    }
    template<typename T>
    requires requires (Target& __target, T&& __value) {
        __target.serialize_i16(std::forward<T>(__value));
    }
    decltype(auto) serialize_i16(T&& value) const {
        return ptr->serialize_i16(std::forward<T>(value));
    }
    template<typename T>
    requires requires (Target& __target, T&& __value) {
        __target.serialize_i32(std::forward<T>(__value));
    }
    decltype(auto) serialize_i32(T&& value) const {
        return ptr->serialize_i32(std::forward<T>(value));
    }
    template<typename T>
    requires requires (Target& __target, T&& __value) {
        __target.serialize_i64(std::forward<T>(__value));
    }
    decltype(auto) serialize_i64(T&& value) const {
        return ptr->serialize_i64(std::forward<T>(value));
    }
    template<typename T>
    requires requires (Target& __target, T&& __value) {
        __target.serialize_u8(std::forward<T>(__value));
    }
    decltype(auto) serialize_u8(T&& value) const {
        return ptr->serialize_u8(std::forward<T>(value));
    }
    template<typename T>
    requires requires (Target& __target, T&& __value) {
        __target.serialize_u16(std::forward<T>(__value));
    }
    decltype(auto) serialize_u16(T&& value) const {
        return ptr->serialize_u16(std::forward<T>(value));
    }
    template<typename T>
    requires requires (Target& __target, T&& __value) {
        __target.serialize_u32(std::forward<T>(__value));
    }
    decltype(auto) serialize_u32(T&& value) const {
        return ptr->serialize_u32(std::forward<T>(value));
    }
    template<typename T>
    requires requires (Target& __target, T&& __value) {
        __target.serialize_u64(std::forward<T>(__value));
    }
    decltype(auto) serialize_u64(T&& value) const {
        return ptr->serialize_u64(std::forward<T>(value));
    }
    template<typename T>
    requires requires (Target& __target, T&& __value) {
        __target.serialize_f32(std::forward<T>(__value));
    }
    decltype(auto) serialize_f32(T&& value) const {
        return ptr->serialize_f32(std::forward<T>(value));
    }
    template<typename T>
    requires requires (Target& __target, T&& __value) {
        __target.serialize_f64(std::forward<T>(__value));
    }
    decltype(auto) serialize_f64(T&& value) const {
        return ptr->serialize_f64(std::forward<T>(value));
    }
    template<typename T>
    requires requires (Target& __target, T&& __value) {
        __target.serialize_char(std::forward<T>(__value));
    }
    decltype(auto) serialize_char(T&& value) const {
        return ptr->serialize_char(std::forward<T>(value));
    }
    decltype(auto) serialize_unit() const
    requires requires (Target& __target) {
        __target.serialize_unit();
    } {
        return ptr->serialize_unit();
    }
    template<typename T>
    requires requires (Target& __target, T&& __value) {
        __target.serialize_bytes(std::forward<T>(__value));
    }
    decltype(auto) serialize_bytes(T&& value) const {
        return ptr->serialize_bytes(std::forward<T>(value));
    }
    template<typename T>
    requires requires (Target& __target, T&& __value) {
        __target.serialize_some(std::forward<T>(__value));
    }
    decltype(auto) serialize_some(T&& value) const {
        return ptr->serialize_some(std::forward<T>(value));
    }
    decltype(auto) serialize_none() const
    requires requires (Target& __target) {
        __target.serialize_none();
    } {
        return ptr->serialize_none();
    }
    template<typename T>
    requires requires (Target& __target, T&& __value) {
        __target.serialize_element(std::forward<T>(__value));
    }
    decltype(auto) serialize_element(T&& value) const {
        return ptr->serialize_element(std::forward<T>(value));
    }
    template<typename T>
    requires requires (Target& __target, T&& __value) {
        __target.serialize_field(std::forward<T>(__value));
    }
    decltype(auto) serialize_field(T&& value) const {
        return ptr->serialize_field(std::forward<T>(value));
    }
    template<typename K, typename V>
    requires requires (Target& __target, K&& __key, V&& __value) {
        __target.serialize_field(std::forward<K>(__key), std::forward<V>(__value));
    }
    decltype(auto) serialize_field(K&& key, V&& value) const {
        return ptr->serialize_field(std::forward<K>(key), std::forward<V>(value));
    }
    template<typename T>
    requires requires (Target& __target, T&& __value) {
        __target.serialize_key(std::forward<T>(__value));
    }
    decltype(auto) serialize_key(T&& value) const {
        return ptr->serialize_key(std::forward<T>(value));
    }
    template<typename T>
    requires requires (Target& __target, T&& __value) {
        __target.serialize_value(std::forward<T>(__value));
    }
    decltype(auto) serialize_value(T&& value) const {
        return ptr->serialize_value(std::forward<T>(value));
    }
    template<typename... Args>
    requires requires (Target& __target, Args&&... __args) {
        __target.serialize_seq(std::forward<Args>(__args)...);
    }
    decltype(auto) serialize_seq(Args&&... args) const {
        return rebind_self_result(
            ptr,
            ptr->serialize_seq(std::forward<Args>(args)...));
    }
    template<typename... Args>
    requires requires (Target& __target, Args&&... __args) {
        __target.serialize_tuple(std::forward<Args>(__args)...);
    }
    decltype(auto) serialize_tuple(Args&&... args) const {
        return rebind_self_result(
            ptr,
            ptr->serialize_tuple(std::forward<Args>(args)...));
    }
    template<typename... Args>
    requires requires (Target& __target, Args&&... __args) {
        __target.serialize_tuple_struct(std::forward<Args>(__args)...);
    }
    decltype(auto) serialize_tuple_struct(Args&&... args) const {
        return rebind_self_result(
            ptr,
            ptr->serialize_tuple_struct(std::forward<Args>(args)...));
    }
    template<typename... Args>
    requires requires (Target& __target, Args&&... __args) {
        __target.serialize_tuple_variant(std::forward<Args>(__args)...);
    }
    decltype(auto) serialize_tuple_variant(Args&&... args) const {
        return rebind_self_result(
            ptr,
            ptr->serialize_tuple_variant(std::forward<Args>(args)...));
    }
    template<typename... Args>
    requires requires (Target& __target, Args&&... __args) {
        __target.serialize_map(std::forward<Args>(__args)...);
    }
    decltype(auto) serialize_map(Args&&... args) const {
        return rebind_self_result(
            ptr,
            ptr->serialize_map(std::forward<Args>(args)...));
    }
    template<typename... Args>
    requires requires (Target& __target, Args&&... __args) {
        __target.serialize_struct(std::forward<Args>(__args)...);
    }
    decltype(auto) serialize_struct(Args&&... args) const {
        return rebind_self_result(
            ptr,
            ptr->serialize_struct(std::forward<Args>(args)...));
    }
    template<typename... Args>
    requires requires (Target& __target, Args&&... __args) {
        __target.serialize_struct_variant(std::forward<Args>(__args)...);
    }
    decltype(auto) serialize_struct_variant(Args&&... args) const {
        return rebind_self_result(
            ptr,
            ptr->serialize_struct_variant(std::forward<Args>(args)...));
    }
    decltype(auto) end() const
    requires requires (Target& __target) {
        __target.end();
    } {
        return ptr->end();
    }
    decltype(auto) next_token() const
    requires requires (Target& __target) {
        __target.next_token();
    } {
        return ptr->next_token();
    }
};
template<typename Serializer>
decltype(auto) forward_serializer(Serializer&& serializer) {
    using Raw = std::remove_cv_t<std::remove_reference_t<Serializer>>;
    if constexpr (
        std::is_lvalue_reference_v<Serializer&&>
        && std::is_class_v<Raw>
        && !requires { typename Raw::serializer_ref_marker; }
        && requires { typename Raw::Ok; typename Raw::Error; }
    ) {
        return serializer_ref<Serializer&&>{&serializer};
    } else {
        return std::forward<Serializer>(serializer);
    }
}
// Explicit (non-deduced) result type for the recursive serialize dispatchers.
// serialize/serialize_value/serialize_bytes call each other, so a deduced
// `decltype(auto)` return makes the mutual recursion ill-formed (function with
// deduced return type cannot be used before it is defined) for any value that
// reaches the recursive fall-through edge. Every serde serialize path yields
// Result<S::Ok, S::Error>, so we can state it explicitly and break the cycle.
// deref_if_pointer_like handles serializers arriving as a raw pointer / ref /
// serializer_ref proxy; a serializer without Ok/Error makes this SFINAE-fail
// (return-type substitution), so `requires { serialize(...) }` stays well-behaved.
template<typename S>
using __ser_result_t = rusty::Result<
    typename std::remove_cv_t<std::remove_reference_t<decltype(
        rusty::detail::deref_if_pointer_like(std::declval<S>()))>>::Ok,
    typename std::remove_cv_t<std::remove_reference_t<decltype(
        rusty::detail::deref_if_pointer_like(std::declval<S>()))>>::Error>;
template<typename Serializer, typename BytesLike>
concept serialize_bytes_compatible =
    requires(Serializer&& serializer, BytesLike&& bytes) {
        std::forward<Serializer>(serializer).serialize_bytes(std::forward<BytesLike>(bytes));
    }
    || requires(Serializer&& serializer, BytesLike&& bytes) {
        std::forward<Serializer>(serializer).serialize_bytes(
            rusty::as_u8_slice(rusty::detail::deref_if_pointer_like(
                std::forward<BytesLike>(bytes))));
    }
    || (requires(Serializer&& serializer) {
        rusty::detail::deref_if_pointer_like(std::forward<Serializer>(serializer)).next_token();
    } && requires(BytesLike&& bytes) {
        rusty::as_u8_slice(rusty::detail::deref_if_pointer_like(std::forward<BytesLike>(bytes)));
    });
// The forward declaration carries the definition's constraint so the two are
// ONE template: `serialize_value` calls `::rusty::serde_rt::ser::serialize_bytes`
// QUALIFIED (no ADL, definition-time lookup), so an unconstrained forward
// declaration was the only candidate it ever bound — a declaration with no
// definition, `undefined reference to serialize_bytes<Serializer*, span>`
// at link (serde_bytes' `#[serde(with)]` wrappers through serde_test's
// serializer, measured).
template<typename Serializer, typename BytesLike>
requires serialize_bytes_compatible<Serializer, BytesLike>
auto serialize_bytes(Serializer&& serializer, BytesLike&& bytes)
    -> ::rusty::serde_rt::ser::__ser_result_t<Serializer>;
struct fallback_error {
    rusty::String message;

    static fallback_error custom(std::string_view msg) {
        return fallback_error{rusty::String::from(msg)};
    }
};
template<typename Value, typename Serializer>
auto serialize_value(Value&& value, Serializer&& serializer)
    -> ::rusty::serde_rt::ser::__ser_result_t<Serializer> {
    using ValueType = std::remove_cv_t<std::remove_reference_t<Value>>;
    if constexpr (requires {
        std::forward<Value>(value).serialize(std::forward<Serializer>(serializer));
    }) {
        return std::forward<Value>(value).serialize(std::forward<Serializer>(serializer));
    } else if constexpr (requires {
        rusty::detail::deref_if_pointer_like(std::forward<Value>(value))
            .serialize(std::forward<Serializer>(serializer));
    }) {
        return rusty::detail::deref_if_pointer_like(std::forward<Value>(value))
            .serialize(std::forward<Serializer>(serializer));
    } else if constexpr (requires {
        std::remove_cv_t<std::remove_reference_t<Value>>::serialize(
            std::forward<Serializer>(serializer));
    }) {
        using ValueType = std::remove_cv_t<std::remove_reference_t<Value>>;
        return ValueType::serialize(std::forward<Serializer>(serializer));
    } else if constexpr (requires {
        std::remove_cv_t<std::remove_reference_t<Value>>::serialize(
            std::forward<Value>(value), std::forward<Serializer>(serializer));
    }) {
        using ValueType = std::remove_cv_t<std::remove_reference_t<Value>>;
        return ValueType::serialize(
            std::forward<Value>(value), std::forward<Serializer>(serializer));
    } else if constexpr (requires {
        ::rusty::serde_rt::ser::serialize_bytes(
            std::forward<Serializer>(serializer), std::forward<Value>(value));
    } && !(
        std::is_convertible_v<
            std::remove_cv_t<std::remove_reference_t<Value>>,
            std::string_view>
        || requires { std::forward<Value>(value).as_str(); }
    )) {
        return ::rusty::serde_rt::ser::serialize_bytes(
            std::forward<Serializer>(serializer), std::forward<Value>(value));
    } else if constexpr (requires {
        std::forward<Serializer>(serializer)
            .serialize_str(rusty::to_string_view(std::forward<Value>(value)));
    }) {
        return std::forward<Serializer>(serializer)
            .serialize_str(rusty::to_string_view(std::forward<Value>(value)));
    } else if constexpr (std::is_same_v<ValueType, bool> && requires {
        std::forward<Serializer>(serializer).serialize_bool(std::forward<Value>(value));
    }) {
        return std::forward<Serializer>(serializer).serialize_bool(std::forward<Value>(value));
    } else if constexpr (std::is_integral_v<ValueType> && std::is_signed_v<ValueType> && sizeof(ValueType) <= sizeof(int8_t) && requires {
        std::forward<Serializer>(serializer).serialize_i8(static_cast<int8_t>(value));
    }) {
        return std::forward<Serializer>(serializer).serialize_i8(static_cast<int8_t>(value));
    } else if constexpr (std::is_integral_v<ValueType> && std::is_signed_v<ValueType> && sizeof(ValueType) <= sizeof(int16_t) && requires {
        std::forward<Serializer>(serializer).serialize_i16(static_cast<int16_t>(value));
    }) {
        return std::forward<Serializer>(serializer).serialize_i16(static_cast<int16_t>(value));
    } else if constexpr (std::is_integral_v<ValueType> && std::is_signed_v<ValueType> && sizeof(ValueType) <= sizeof(int32_t) && requires {
        std::forward<Serializer>(serializer).serialize_i32(static_cast<int32_t>(value));
    }) {
        return std::forward<Serializer>(serializer).serialize_i32(static_cast<int32_t>(value));
    } else if constexpr (std::is_integral_v<ValueType> && std::is_signed_v<ValueType> && sizeof(ValueType) <= sizeof(int64_t) && requires {
        std::forward<Serializer>(serializer).serialize_i64(static_cast<int64_t>(value));
    }) {
        return std::forward<Serializer>(serializer).serialize_i64(static_cast<int64_t>(value));
    } else if constexpr (std::is_integral_v<ValueType> && std::is_signed_v<ValueType> && sizeof(ValueType) > sizeof(int64_t) && requires {
        std::forward<Serializer>(serializer).serialize_i128(static_cast<__int128>(value));
    }) {
        return std::forward<Serializer>(serializer).serialize_i128(static_cast<__int128>(value));
    } else if constexpr (std::is_integral_v<ValueType> && !std::is_signed_v<ValueType> && !std::is_same_v<ValueType, bool> && sizeof(ValueType) <= sizeof(uint8_t) && requires {
        std::forward<Serializer>(serializer).serialize_u8(static_cast<uint8_t>(value));
    }) {
        return std::forward<Serializer>(serializer).serialize_u8(static_cast<uint8_t>(value));
    } else if constexpr (std::is_integral_v<ValueType> && !std::is_signed_v<ValueType> && !std::is_same_v<ValueType, bool> && sizeof(ValueType) <= sizeof(uint16_t) && requires {
        std::forward<Serializer>(serializer).serialize_u16(static_cast<uint16_t>(value));
    }) {
        return std::forward<Serializer>(serializer).serialize_u16(static_cast<uint16_t>(value));
    } else if constexpr (std::is_integral_v<ValueType> && !std::is_signed_v<ValueType> && !std::is_same_v<ValueType, bool> && sizeof(ValueType) <= sizeof(uint32_t) && requires {
        std::forward<Serializer>(serializer).serialize_u32(static_cast<uint32_t>(value));
    }) {
        return std::forward<Serializer>(serializer).serialize_u32(static_cast<uint32_t>(value));
    } else if constexpr (std::is_integral_v<ValueType> && !std::is_signed_v<ValueType> && !std::is_same_v<ValueType, bool> && sizeof(ValueType) <= sizeof(uint64_t) && requires {
        std::forward<Serializer>(serializer).serialize_u64(static_cast<uint64_t>(value));
    }) {
        return std::forward<Serializer>(serializer).serialize_u64(static_cast<uint64_t>(value));
    } else if constexpr (std::is_integral_v<ValueType> && !std::is_signed_v<ValueType> && !std::is_same_v<ValueType, bool> && sizeof(ValueType) > sizeof(uint64_t) && requires {
        std::forward<Serializer>(serializer).serialize_u128(static_cast<unsigned __int128>(value));
    }) {
        return std::forward<Serializer>(serializer).serialize_u128(static_cast<unsigned __int128>(value));
    } else if constexpr (std::is_same_v<ValueType, float> && requires {
        std::forward<Serializer>(serializer).serialize_f32(std::forward<Value>(value));
    }) {
        return std::forward<Serializer>(serializer).serialize_f32(std::forward<Value>(value));
    } else if constexpr (std::is_same_v<ValueType, double> && requires {
        std::forward<Serializer>(serializer).serialize_f64(std::forward<Value>(value));
    }) {
        return std::forward<Serializer>(serializer).serialize_f64(std::forward<Value>(value));
    } else if constexpr (std::is_same_v<ValueType, char32_t> && requires {
        std::forward<Serializer>(serializer).serialize_char(std::forward<Value>(value));
    }) {
        return std::forward<Serializer>(serializer).serialize_char(std::forward<Value>(value));
    } else if constexpr (std::is_same_v<ValueType, std::tuple<>> && requires {
        std::forward<Serializer>(serializer).serialize_unit();
    }) {
        return std::forward<Serializer>(serializer).serialize_unit();
    } else if constexpr (requires {
        serialize(std::forward<Value>(value), std::forward<Serializer>(serializer));
    }) {
        return serialize(std::forward<Value>(value), std::forward<Serializer>(serializer));
    } else {
        return std::forward<Value>(value).serialize(std::forward<Serializer>(serializer));
    }
}
template<typename Value, typename Serializer>
auto serialize(Value&& value, Serializer&& serializer)
    -> ::rusty::serde_rt::ser::__ser_result_t<Serializer> {
    return ::rusty::serde_rt::ser::serialize_value(
        std::forward<Value>(value), std::forward<Serializer>(serializer));
}
template<typename Serializer, typename BytesLike>
requires serialize_bytes_compatible<Serializer, BytesLike>
auto serialize_bytes(Serializer&& serializer, BytesLike&& bytes)
    -> ::rusty::serde_rt::ser::__ser_result_t<Serializer> {
    if constexpr (requires {
        std::forward<Serializer>(serializer).serialize_bytes(std::forward<BytesLike>(bytes));
    }) {
        return std::forward<Serializer>(serializer).serialize_bytes(std::forward<BytesLike>(bytes));
    } else if constexpr (requires {
        std::forward<Serializer>(serializer).serialize_bytes(
            rusty::as_u8_slice(rusty::detail::deref_if_pointer_like(
                std::forward<BytesLike>(bytes))));
    }) {
        return std::forward<Serializer>(serializer).serialize_bytes(
            rusty::as_u8_slice(rusty::detail::deref_if_pointer_like(
                std::forward<BytesLike>(bytes))));
    } else if constexpr (requires {
        rusty::detail::deref_if_pointer_like(std::forward<Serializer>(serializer)).next_token();
    }) {
        // The serializer may arrive as the POINTER representation of a Rust
        // `&mut S` (a `&mut ser` into a by-value type parameter): its
        // `Error` is the pointee's.
        using SerializerType = std::remove_cv_t<std::remove_reference_t<decltype(
            rusty::detail::deref_if_pointer_like(std::declval<Serializer>()))>>;
        using SerializerError = std::conditional_t<
            requires { typename SerializerType::Error; },
            typename SerializerType::Error,
            fallback_error>;
        auto&& __bytes = bytes;
        auto&& __serializer_recv = rusty::detail::deref_if_pointer_like(std::forward<Serializer>(serializer));
        auto expected = [&]() -> std::span<const uint8_t> {
            auto&& __bytes_value = rusty::detail::deref_if_pointer_like(__bytes);
            if constexpr (requires { rusty::as_u8_slice(__bytes_value); }) {
                return rusty::as_u8_slice(__bytes_value);
            } else if constexpr (requires { rusty::as_slice(__bytes_value); }) {
                return rusty::as_slice(__bytes_value);
            } else if constexpr (requires { __bytes_value.as_ref(); }) {
                return rusty::as_u8_slice(__bytes_value.as_ref());
            } else {
                return std::span<const uint8_t>();
            }
        }();
        auto same_bytes = [&](std::span<const uint8_t> got) {
            if (got.size() != expected.size()) {
                return false;
            }
            for (size_t i = 0; i < got.size(); ++i) {
                if (got[i] != expected[i]) {
                    return false;
                }
            }
            return true;
        };
        auto token_opt = __serializer_recv.next_token();
        if (token_opt.is_none()) {
            return rusty::Result<std::tuple<>, SerializerError>::Err(
                SerializerError::custom("expected Token::Bytes, Token::BorrowedBytes, or Token::ByteBuf"));
        }
        auto token = token_opt.unwrap();
        if constexpr (requires {
            rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Bytes>(token);
            rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_Bytes>(token)._0;
        }) {
            if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_Bytes>(token)
                && same_bytes(rusty::as_u8_slice(
                    rusty::detail::deref_if_pointer(rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_Bytes>(token)._0)))) {
                return rusty::Result<std::tuple<>, SerializerError>::Ok(std::make_tuple());
            }
        }
        if constexpr (requires {
            rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedBytes>(token);
            rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedBytes>(token)._0;
        }) {
            if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedBytes>(token)
                && same_bytes(rusty::as_u8_slice(
                    rusty::detail::deref_if_pointer(rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_BorrowedBytes>(token)._0)))) {
                return rusty::Result<std::tuple<>, SerializerError>::Ok(std::make_tuple());
            }
        }
        if constexpr (requires {
            rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_ByteBuf>(token);
            rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_ByteBuf>(token)._0;
        }) {
            if (rusty::detail::variant_holds<::RUSTY_SERDE_TOKEN_NS::Token_ByteBuf>(token)
                && same_bytes(rusty::as_u8_slice(
                    rusty::detail::deref_if_pointer(rusty::detail::variant_get<::RUSTY_SERDE_TOKEN_NS::Token_ByteBuf>(token)._0)))) {
                return rusty::Result<std::tuple<>, SerializerError>::Ok(std::make_tuple());
            }
        }
        return rusty::Result<std::tuple<>, SerializerError>::Err(
            SerializerError::custom(std::format(
                "serialized bytes did not match expected token (expected_len={0}, token={1})",
                expected.size(), rusty::to_string(token))));
    } else {
        return std::forward<Serializer>(serializer).serialize_bytes(std::forward<BytesLike>(bytes));
    }
}
}
namespace impls {
inline namespace v1 {
template<typename Value, typename Serializer>
auto serialize(Value&& value, Serializer&& serializer)
    -> ::rusty::serde_rt::ser::__ser_result_t<Serializer> {
    return ::rusty::serde_rt::ser::serialize_value(
        std::forward<Value>(value), std::forward<Serializer>(serializer));
}
}
}
}

#endif // RUSTY_SERDE_RUNTIME_HPP
