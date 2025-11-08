#include <type_traits>
#include <iostream>

class ThirdPartyType {
public:
    ThirdPartyType() = default;
    ThirdPartyType(ThirdPartyType&&) = default;
};

int main() {
    std::cout << "is_move_constructible: " << std::is_move_constructible_v<ThirdPartyType> << "\n";
    std::cout << "is_move_assignable: " << std::is_move_assignable_v<ThirdPartyType> << "\n";
   std::cout << "is_destructible: " << std::is_destructible_v<ThirdPartyType> << "\n";

    return 0;
}
