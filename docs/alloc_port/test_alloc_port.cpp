// Runtime validation of alloc_port: Vec core (construct/push/len/index/clone/iterate).
import alloc_port;
#include <cassert>
#include <utility>
#include <cstdio>

int main() {
    auto v = vec::Vec<int>::new_();
    for (int i = 0; i < 5; ++i) v.push(i * 10);   // 0,10,20,30,40
    assert(v.len() == 5);
    assert(v[0] == 0 && v[4] == 40);

    // mutate through operator[]
    v[2] = 999;
    assert(v[2] == 999);

    // deep clone independence
    auto c = v.clone();
    c.push(1234);
    assert(v.len() == 5 && c.len() == 6);
    assert(c[5] == 1234 && c[2] == 999);

    // iterate via operator[]
    long sum = 0;
    for (size_t i = 0; i < v.len(); ++i) sum += v[i];
    assert(sum == 0 + 10 + 999 + 30 + 40);

    std::printf("alloc_port Vec runtime OK: push/len/index/assign/clone-independence/as_slice\n");
    return 0;
}
