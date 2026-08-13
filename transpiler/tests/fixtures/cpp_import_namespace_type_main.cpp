#include <cstdint>

#include "rusty/option.hpp"

import rrr.channel;
import rrr.consumer;

int main() {
    rrr::ChannelFrame frame = rrr::make_frame(40);
    if (rrr::destructure_frame(rrr::make_frame(9)) != 9
        || rrr::tuple_value(10) != 10
        || rrr::unit_value() != 1) {
        return 6;
    }
    if (rrr::lexical_matrix::construct_qualified(12) != 12
        || rrr::lexical_matrix::construct_imported(13) != 13
        || rrr::lexical_matrix::enum_qualified() != 0
        || rrr::lexical_matrix::enum_imported() != 0
        || rrr::lexical_matrix::tuple_qualified(14) != 14
        || rrr::lexical_matrix::tuple_imported(15) != 15
        || rrr::lexical_matrix::unit_qualified() != 2
        || rrr::lexical_matrix::unit_imported() != 3) {
        return 7;
    }
    rrr::external::ChannelFrame foreign = rrr::make_external(1);
    rrr::LocalChannel channel{2};
    if (rrr::enum_value() != rrr::ChannelError::None) {
        return 1;
    }
    if (rrr::external_enum_value() != rrr::external::ChannelError::Foreign) {
        return 2;
    }
    if (rrr::external_enum_crate() != rrr::external::ChannelError::Foreign) {
        return 3;
    }
    if (rrr::associated_const_same_tail() != 17
        || rrr::associated_alias_const_same_tail() != 17
        || rrr::associated_method_same_tail() != 19
        || rrr::associated_variant_same_tail() != 23) {
        return 8;
    }
    if (!rrr::clear_option(rusty::Option<int32_t>(7)).is_none()) {
        return 4;
    }
    const auto qualified = rrr::inspect_self(foreign) + rrr::inspect_crate(foreign)
        + rrr::nested::inspect_super(foreign) + rrr::nested::inspect_crate(foreign);
    return rrr::inspect(frame, foreign, foreign) + channel.code() + qualified == 49 ? 0 : 5;
}
