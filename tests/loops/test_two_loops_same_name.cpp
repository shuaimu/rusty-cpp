#include <memory>
#include <list>

void consume(std::unique_ptr<int> p);

// @safe
void test() {
    std::list<std::unique_ptr<int>> items;
    
    // @unsafe
    {
        // First loop: creates 'item' and moves it
        for (int i = 0; i < 5; i++) {
            auto item = std::make_unique<int>(i);
            items.push_back(std::move(item));
        }
        
        // Second loop: creates ANOTHER 'item' (different scope!)
        while (!items.empty()) {
            std::unique_ptr<int> item = std::move(items.front());
            items.pop_front();
            consume(std::move(item));
        }
    }
}
