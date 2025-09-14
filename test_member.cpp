// @safe
class Timer {
public:
    // @safe
    void reset() {}
    // @safe
    void start() {
        reset();           // implicit this - should be Timer::reset
        this->reset();     // explicit this - should be Timer::reset
    }
};

// @safe
void test_function() {
    Timer t;
    t.reset();            // should be Timer::reset
    t.start();            // should be Timer::start
}