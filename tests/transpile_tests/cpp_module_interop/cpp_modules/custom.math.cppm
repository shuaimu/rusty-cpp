export module custom.math;

import std;

export namespace custom::math {
constexpr int DEFAULT_BIAS = 1;

int add_one(int value) {
    return value + DEFAULT_BIAS;
}
}
