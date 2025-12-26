
#include <memory>
#include <functional>

void dispatch(std::function<void()> fn);

// @safe
void test() {
    // @unsafe
    {
        for (int i = 0; i < 5; i++) {
            // Fresh variable each iteration
            std::unique_ptr<int> data = std::make_unique<int>(i);

            // Move into lambda - OK since data is fresh each iteration
            auto fn = [d = std::move(data)]() mutable {
                // use d
            };
            dispatch(std::move(fn));
        }
    }
}
