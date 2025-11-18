#include <iostream>
#include <string>
#include "../include/rusty/option.hpp"

// @safe
int main() {
    std::cout << "Testing Option::as_ref() and as_mut()...\n\n";

    // Test 1: Basic as_ref()
    {
        std::cout << "Test 1: Basic as_ref()\n";
        auto opt = rusty::Some<std::string>("hello");

        auto ref_opt = opt.as_ref();
        if (ref_opt.is_some()) {
            const auto& s = ref_opt.unwrap();
            std::cout << "  Value: " << s << "\n";
            std::cout << "  Length: " << s.length() << "\n";
        }

        // Original opt still usable
        std::cout << "  Original: " << opt.unwrap() << "\n\n";
    }

    // Test 2: as_ref() on None
    {
        std::cout << "Test 2: as_ref() on None\n";
        rusty::Option<std::string> opt = rusty::None;

        auto ref_opt = opt.as_ref();
        std::cout << "  Is None: " << (ref_opt.is_none() ? "yes" : "no") << "\n\n";
    }

    // Test 3: as_mut()
    {
        std::cout << "Test 3: as_mut()\n";
        auto opt = rusty::Some<std::string>("hello");

        auto mut_opt = opt.as_mut();
        if (mut_opt.is_some()) {
            auto& s = mut_opt.unwrap();
            s.append(" world");
            std::cout << "  Modified: " << s << "\n";
        }

        std::cout << "  Final value: " << opt.unwrap() << "\n\n";
    }

    // Test 4: as_ref() with map()
    {
        std::cout << "Test 4: as_ref() with map()\n";
        auto opt = rusty::Some<std::string>("hello");

        auto len_opt = opt.as_ref().map([](const std::string& s) {
            return s.length();
        });

        if (len_opt.is_some()) {
            std::cout << "  Length: " << len_opt.unwrap() << "\n";
        }

        std::cout << "  Original still exists: " << opt.unwrap() << "\n\n";
    }

    // Test 5: Option<T&> directly
    {
        std::cout << "Test 5: Option<T&> directly\n";
        std::string s = "hello";
        rusty::Option<std::string&> ref_opt(s);

        if (ref_opt.is_some()) {
            auto& str_ref = ref_opt.unwrap();
            str_ref.append(" world");
            std::cout << "  Modified through Option<T&>: " << str_ref << "\n";
        }

        std::cout << "  Original string: " << s << "\n\n";
    }

    // Test 6: Option<const T&>
    {
        std::cout << "Test 6: Option<const T&>\n";
        const std::string s = "hello";
        rusty::Option<const std::string&> ref_opt(s);

        if (ref_opt.is_some()) {
            const auto& str_ref = ref_opt.unwrap();
            std::cout << "  Value: " << str_ref << "\n";
            std::cout << "  Length: " << str_ref.length() << "\n";
        }
        std::cout << "\n";
    }

    // Test 7: Multiple as_ref() calls
    {
        std::cout << "Test 7: Multiple as_ref() calls\n";
        auto opt = rusty::Some<std::string>("hello");

        auto ref1 = opt.as_ref();
        auto ref2 = opt.as_ref();

        if (ref1.is_some() && ref2.is_some()) {
            std::cout << "  Both references valid\n";
            std::cout << "  ref1: " << ref1.unwrap() << "\n";
            std::cout << "  ref2: " << ref2.unwrap() << "\n";
        }
        std::cout << "\n";
    }

    // Test 8: contains() on Option<T&>
    {
        std::cout << "Test 8: contains() on Option<T&>\n";
        std::string s = "hello";
        rusty::Option<std::string&> ref_opt(s);

        std::cout << "  Contains 'hello': " << (ref_opt.contains(s) ? "yes" : "no") << "\n";

        std::string other = "world";
        std::cout << "  Contains 'world': " << (ref_opt.contains(other) ? "yes" : "no") << "\n";
        std::cout << "\n";
    }

    std::cout << "All tests completed successfully!\n";
    return 0;
}
