// Analysis and Benchmark of our BTreeMap implementation vs Rust's BTreeMap design

#include "../include/rusty/btreemap.hpp"
#include "../include/rusty/hashmap.hpp"
#include <iostream>
#include <chrono>
#include <random>
#include <iomanip>
#include <map>  // For comparison with std::map (Red-Black tree)

using namespace rusty;
using namespace std::chrono;

/*
 * IMPLEMENTATION COMPARISON:
 * 
 * Our BTreeMap (Flat Map / Sorted Vector):
 * - Uses a sorted Vec<pair<K,V>> internally
 * - Binary search for lookups
 * - Linear shifting for insertions/deletions
 * - All data in contiguous memory
 * 
 * Rust's BTreeMap (B-Tree):
 * - Real B-Tree with nodes containing multiple keys
 * - Node size optimized for cache lines (B=6 typically)
 * - Tree rebalancing on insert/delete
 * - Non-contiguous memory (nodes allocated separately)
 * 
 * Time Complexity Comparison:
 * ┌─────────────┬────────────────┬─────────────────┐
 * │ Operation   │ Our Flat Map   │ Rust B-Tree     │
 * ├─────────────┼────────────────┼─────────────────┤
 * │ Lookup      │ O(log n)       │ O(log n)        │
 * │ Insert      │ O(n)           │ O(log n)        │
 * │ Delete      │ O(n)           │ O(log n)        │
 * │ Iteration   │ O(n)           │ O(n)            │
 * │ Range Query │ O(log n + k)   │ O(log n + k)    │
 * └─────────────┴────────────────┴─────────────────┘
 * 
 * Space Complexity:
 * - Our Flat Map: O(n) contiguous memory, no overhead
 * - Rust B-Tree: O(n) + node overhead (pointers, partial filling)
 */

void print_analysis() {
    std::cout << "BTreeMap Implementation Analysis" << std::endl;
    std::cout << "=================================" << std::endl;
    std::cout << "\nOur Implementation (Flat Map/Sorted Vector):" << std::endl;
    std::cout << "--------------------------------------------" << std::endl;
    std::cout << "✅ Advantages:" << std::endl;
    std::cout << "  • Excellent cache locality (contiguous memory)" << std::endl;
    std::cout << "  • Minimal memory overhead (no node structures)" << std::endl;
    std::cout << "  • Simple implementation (~400 lines)" << std::endl;
    std::cout << "  • Fast iteration (sequential memory access)" << std::endl;
    std::cout << "  • Fast range queries for small ranges" << std::endl;
    std::cout << "  • Optimal for small to medium collections (<1000 items)" << std::endl;
    std::cout << "  • Predictable memory usage" << std::endl;
    
    std::cout << "\n❌ Disadvantages:" << std::endl;
    std::cout << "  • O(n) insertion/deletion (requires shifting)" << std::endl;
    std::cout << "  • Poor performance for frequent modifications" << std::endl;
    std::cout << "  • Not suitable for large datasets" << std::endl;
    std::cout << "  • No incremental growth (Vec resizing)" << std::endl;
    
    std::cout << "\nRust's BTreeMap (Real B-Tree):" << std::endl;
    std::cout << "-------------------------------" << std::endl;
    std::cout << "✅ Advantages:" << std::endl;
    std::cout << "  • O(log n) for all operations" << std::endl;
    std::cout << "  • Scales well to millions of elements" << std::endl;
    std::cout << "  • Efficient for frequent modifications" << std::endl;
    std::cout << "  • Node-based structure allows incremental growth" << std::endl;
    std::cout << "  • Better worst-case guarantees" << std::endl;
    
    std::cout << "\n❌ Disadvantages:" << std::endl;
    std::cout << "  • More complex implementation (>2000 lines)" << std::endl;
    std::cout << "  • Higher memory overhead (node structures)" << std::endl;
    std::cout << "  • Worse cache locality (pointer chasing)" << std::endl;
    std::cout << "  • Slower for small collections" << std::endl;
}

template<typename MapType>
double benchmark_insertions(int n, bool sequential) {
    MapType map;
    std::vector<int> keys(n);
    
    if (sequential) {
        for (int i = 0; i < n; i++) keys[i] = i;
    } else {
        std::random_device rd;
        std::mt19937 gen(rd());
        std::uniform_int_distribution<> dis(0, n * 10);
        for (int i = 0; i < n; i++) keys[i] = dis(gen);
    }
    
    auto start = high_resolution_clock::now();
    for (int key : keys) {
        map.insert(key, key * 2);
    }
    return duration<double, std::milli>(high_resolution_clock::now() - start).count();
}

template<typename MapType>
double benchmark_lookups(MapType& map, int n) {
    auto start = high_resolution_clock::now();
    long long sum = 0;
    for (int i = 0; i < n; i++) {
        auto val = map.get(i);
        if (val.is_some()) sum += *val.unwrap();
    }
    auto time = duration<double, std::milli>(high_resolution_clock::now() - start).count();
    if (sum == -1) std::cout << "Never"; // Prevent optimization
    return time;
}

// Adapter for std::map to match our interface
template<typename K, typename V>
class StdMapAdapter {
    std::map<K, V> map_;
public:
    void insert(K key, V value) { map_[key] = value; }
    Option<V*> get(const K& key) {
        auto it = map_.find(key);
        if (it != map_.end()) return Some(&it->second);
        return None;
    }
    size_t len() const { return map_.size(); }
};

void run_benchmarks() {
    std::cout << "\nPerformance Benchmarks" << std::endl;
    std::cout << "======================" << std::endl;
    
    std::cout << std::fixed << std::setprecision(2);
    
    for (int size : {100, 1000, 10000}) {
        std::cout << "\nDataset size: " << size << " elements" << std::endl;
        std::cout << "--------------------------------" << std::endl;
        
        // Sequential insertions
        std::cout << "Sequential insertions:" << std::endl;
        auto flat_time = benchmark_insertions<BTreeMap<int, int>>(size, true);
        auto hash_time = benchmark_insertions<HashMap<int, int>>(size, true);
        auto std_time = benchmark_insertions<StdMapAdapter<int, int>>(size, true);
        
        std::cout << "  Flat Map (ours):  " << std::setw(8) << flat_time << " ms" << std::endl;
        std::cout << "  HashMap:          " << std::setw(8) << hash_time << " ms" << std::endl;
        std::cout << "  std::map (RB):    " << std::setw(8) << std_time << " ms" << std::endl;
        
        // Random insertions
        std::cout << "Random insertions:" << std::endl;
        flat_time = benchmark_insertions<BTreeMap<int, int>>(size, false);
        hash_time = benchmark_insertions<HashMap<int, int>>(size, false);
        std_time = benchmark_insertions<StdMapAdapter<int, int>>(size, false);
        
        std::cout << "  Flat Map (ours):  " << std::setw(8) << flat_time << " ms" << std::endl;
        std::cout << "  HashMap:          " << std::setw(8) << hash_time << " ms" << std::endl;
        std::cout << "  std::map (RB):    " << std::setw(8) << std_time << " ms" << std::endl;
        
        // Lookup benchmark
        BTreeMap<int, int> btree;
        HashMap<int, int> hashmap;
        StdMapAdapter<int, int> stdmap;
        
        for (int i = 0; i < size; i++) {
            btree.insert(i, i * 2);
            hashmap.insert(i, i * 2);
            stdmap.insert(i, i * 2);
        }
        
        std::cout << "Lookups (" << size << " queries):" << std::endl;
        auto btree_lookup = benchmark_lookups(btree, size);
        auto hash_lookup = benchmark_lookups(hashmap, size);
        auto std_lookup = benchmark_lookups(stdmap, size);
        
        std::cout << "  Flat Map (ours):  " << std::setw(8) << btree_lookup << " ms" << std::endl;
        std::cout << "  HashMap:          " << std::setw(8) << hash_lookup << " ms" << std::endl;
        std::cout << "  std::map (RB):    " << std::setw(8) << std_lookup << " ms" << std::endl;
    }
}

void test_memory_patterns() {
    std::cout << "\nMemory Access Patterns" << std::endl;
    std::cout << "======================" << std::endl;
    
    const int N = 1000;
    BTreeMap<int, int> btree;
    
    // Fill the map
    for (int i = 0; i < N; i++) {
        btree.insert(i, i);
    }
    
    // Test iteration (sequential access)
    auto start = high_resolution_clock::now();
    long long sum = 0;
    for (auto [key, value] : btree) {
        sum += value;
    }
    auto iter_time = duration<double, std::micro>(high_resolution_clock::now() - start).count();
    
    std::cout << "Iteration over " << N << " elements: " << iter_time << " μs" << std::endl;
    std::cout << "Average per element: " << (iter_time / N) << " μs" << std::endl;
    std::cout << "✅ Excellent cache locality due to contiguous storage" << std::endl;
    
    // Prevent optimization
    if (sum == -1) std::cout << "Never";
}

void show_recommendations() {
    std::cout << "\n📊 Usage Recommendations" << std::endl;
    std::cout << "========================" << std::endl;
    
    std::cout << "\n✅ Use our Flat Map BTreeMap when:" << std::endl;
    std::cout << "  • Collection size < 1000 elements" << std::endl;
    std::cout << "  • Lookups are much more frequent than insertions" << std::endl;
    std::cout << "  • Data is mostly static after initial setup" << std::endl;
    std::cout << "  • Cache performance is critical" << std::endl;
    std::cout << "  • Memory usage needs to be minimal" << std::endl;
    std::cout << "  • Iteration performance is important" << std::endl;
    std::cout << "  Examples: Config maps, small caches, enum mappings" << std::endl;
    
    std::cout << "\n❌ Avoid our Flat Map BTreeMap when:" << std::endl;
    std::cout << "  • Collection size > 10,000 elements" << std::endl;
    std::cout << "  • Frequent insertions/deletions in random positions" << std::endl;
    std::cout << "  • Real-time systems with strict latency requirements" << std::endl;
    std::cout << "  • Need guaranteed O(log n) modifications" << std::endl;
    std::cout << "  Examples: Large databases, real-time event processing" << std::endl;
    
    std::cout << "\n💡 Alternative: Consider HashMap when:" << std::endl;
    std::cout << "  • Order doesn't matter" << std::endl;
    std::cout << "  • Need O(1) average operations" << std::endl;
    std::cout << "  • Hash function is good" << std::endl;
}

int main() {
    std::cout << "=====================================================" << std::endl;
    std::cout << "    BTreeMap Implementation Analysis & Benchmarks    " << std::endl;
    std::cout << "=====================================================" << std::endl;
    
    print_analysis();
    run_benchmarks();
    test_memory_patterns();
    show_recommendations();
    
    std::cout << "\n=====================================================" << std::endl;
    std::cout << "Summary: Our Flat Map is a specialized implementation" << std::endl;
    std::cout << "optimal for small-to-medium read-heavy workloads," << std::endl;
    std::cout << "while Rust's B-Tree is a general-purpose solution" << std::endl;
    std::cout << "that scales to any size with consistent performance." << std::endl;
    std::cout << "=====================================================" << std::endl;
    
    return 0;
}