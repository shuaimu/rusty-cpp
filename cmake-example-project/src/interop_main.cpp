import interop.host;
import interop.bridge;

class Session {
public:
    explicit Session(int seed) : counter_(seed) {}

    // This demonstrates C++ -> Rust member-call interop from a C++ class method.
    int run_round(RustAccumulator& acc, int delta) {
        const int pulled = acc.pull_from_cpp(counter_, delta);
        const int biased = acc.add_cpp_bias();
        const int bumped = acc.bump(3);
        return pulled + biased + bumped + acc.current() + counter_.value();
    }

    int counter_value() const {
        return counter_.value();
    }

private:
    interop::api::Counter counter_;
};

int main() {
    RustAccumulator acc = RustAccumulator::new_(5);
    Session session(10);

    // Expected:
    // pull_from_cpp with delta=2 -> counter=12, total=17
    // add_cpp_bias -> total=21; bump(3) -> total=24
    // score = 17 + 21 + 24 + 24 + 12 = 98
    const int score = session.run_round(acc, 2);
    if (score != 98) {
        return 1;
    }
    if (acc.current() != 24) {
        return 2;
    }
    if (session.counter_value() != 12) {
        return 3;
    }
    return 0;
}
