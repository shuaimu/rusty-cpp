import interop.host;
import interop.bridge;

class Session {
public:
    explicit Session(int seed) : counter_(seed) {}

    // This demonstrates C++ -> Rust member-call interop from a C++ class method.
    int run_round(RustAccumulator& acc, int delta) {
        const int pulled = acc.pull_from_cpp(counter_, delta);
        const int bumped = acc.bump(3);
        return pulled + bumped + acc.current() + counter_.value();
    }

    int counter_value() const {
        return counter_.value();
    }

private:
    interop::host::Counter counter_;
};

int main() {
    RustAccumulator acc = RustAccumulator::new_(5);
    Session session(10);

    // Expected:
    // pull_from_cpp with delta=2 -> counter=12, total=17
    // bump(3) -> total=20
    // score = 17 + 20 + 20 + 12 = 69
    const int score = session.run_round(acc, 2);
    if (score != 69) {
        return 1;
    }
    if (acc.current() != 20) {
        return 2;
    }
    if (session.counter_value() != 12) {
        return 3;
    }
    return 0;
}
