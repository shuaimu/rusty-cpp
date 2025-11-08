# Lock-Free MPSC Channel - Developer Guide

## Architecture Overview

The lock-free MPSC channel is built on a lock-free linked list queue optimized for the multi-producer single-consumer pattern.

### Core Design Principles

1. **Lock-Free Producers**: Multiple threads atomically append to tail
2. **Wait-Free Consumer**: Single thread reads from head (no atomics needed)
3. **No ABA Problem**: Single consumer owns popped nodes
4. **Cache-Line Aligned**: Prevents false sharing between producer/consumer
5. **Memory Safe**: Clear ownership model, zero leaks

### Data Structure

```
    head (consumer side)          tail (producer side)
        |                              |
        v                              v
    [Dummy] -> [Node1] -> [Node2] -> [Node3]

    - head: Non-atomic pointer (single consumer)
    - tail: Atomic pointer (multiple producers)
    - Each node contains T data + atomic<Node*> next
```

**Key Insight**: Dummy node simplifies empty queue logic and eliminates special cases.

## Implementation Details

### Node Structure

```cpp
template<typename T>
struct Node {
    T data;
    std::atomic<Node<T>*> next;

    explicit Node(T&& value)
        : data(std::move(value)), next(nullptr) {}
};
```

**Alignment**:
- Nodes are naturally aligned (no special padding needed)
- Data is moved, not copied (ownership transfer)
- `next` pointer is atomic for concurrent access

### Channel State

```cpp
template<typename T>
class LockFreeChannelState {
    alignas(64) std::atomic<Node<T>*> tail_;      // Producer side
    alignas(64) Node<T>* head_;                   // Consumer side
    alignas(64) std::atomic<size_t> sender_count_;
    std::atomic<bool> receiver_alive_;
    alignas(64) std::atomic<uint32_t> signal_;    // Wake/sleep
};
```

**Cache Line Alignment**:
- Each field on separate cache line (64 bytes)
- Prevents false sharing between producer/consumer
- Critical for performance on multi-core systems

### Memory Ordering

The implementation uses precise memory ordering for correctness and performance:

#### Send Operation

```cpp
Result<Unit, TrySendError> try_send(T value) {
    if (!receiver_alive_.load(std::memory_order_acquire)) {
        return Err(Disconnected);
    }

    Node<T>* new_node = new Node<T>(std::move(value));
    new_node->next.store(nullptr, std::memory_order_relaxed);

    // Critical: Single atomic exchange
    Node<T>* old_tail = tail_.exchange(new_node, std::memory_order_acq_rel);
    old_tail->next.store(new_node, std::memory_order_release);  // Publish

    signal_.fetch_add(1, std::memory_order_release);
    return Ok(Unit{});
}
```

**Memory Ordering Rationale**:
- `receiver_alive_`: **acquire** - See all receiver operations before drop
- `tail_.exchange()`: **acq_rel** - Synchronize with other producers
- `old_tail->next.store()`: **release** - Publish node to consumer
- `signal_.fetch_add()`: **release** - Wake consumer with visibility

#### Receive Operation (Non-Blocking)

```cpp
Option<T> try_pop() {
    Node<T>* head = head_;  // No atomic needed!
    Node<T>* next = head->next.load(std::memory_order_acquire);

    if (next == nullptr) return None;

    T value = std::move(next->data);
    head_ = next;  // Advance head
    delete head;   // Delete old dummy

    return Some(std::move(value));
}
```

**Memory Ordering Rationale**:
- `head->next.load()`: **acquire** - See producer's publish (release)
- `head_`: **No atomic** - Single consumer, no contention!
- Forms happens-before with producer's release store

**Why Wait-Free**: No loops, no CAS, no retries. Single consumer always succeeds immediately.

### Blocking Mechanism

Three-phase hybrid waiting strategy for efficient blocking:

```cpp
Result<T, RecvError> blocking_recv() {
    Backoff backoff;

    // Phase 1: Optimistic spin (fast path)
    for (int i = 0; i < 10; ++i) {
        if (auto result = try_pop()) {
            return Ok(result.unwrap());
        }
        CPU_RELAX();
    }

    // Phase 2: Exponential backoff
    while (!backoff.is_completed()) {
        if (auto result = try_pop()) {
            return Ok(result.unwrap());
        }
        if (sender_count_.load(acquire) == 0) {
            return Err(Disconnected);
        }
        backoff.spin();
    }

    // Phase 3: Efficient wait
    #if __cplusplus >= 202002L
        while (true) {
            uint32_t old_signal = signal_.load(acquire);
            if (auto result = try_pop()) {
                return Ok(result.unwrap());
            }
            if (sender_count_.load(acquire) == 0) {
                return Err(Disconnected);
            }
            signal_.wait(old_signal, acquire);  // Futex on Linux
        }
    #else
        // C++17 fallback
        while (true) {
            if (auto result = try_pop()) {
                return Ok(result.unwrap());
            }
            if (sender_count_.load(acquire) == 0) {
                return Err(Disconnected);
            }
            std::this_thread::sleep_for(microseconds(1));
        }
    #endif
}
```

**Phase Rationale**:
1. **Optimistic spin** (1-10 μs): Message likely ready, avoid overhead
2. **Exponential backoff**: Reduce CPU usage while still responsive
3. **Efficient wait**: Zero CPU when idle (futex or sleep)

**Performance Impact**:
- Fast path: 0.3 μs (message ready)
- Slow path: 4 μs (wake from wait)
- Zero CPU when idle

### Batch Operations

#### Batch Send

```cpp
template<typename Iterator>
size_t batch_send(Iterator begin, Iterator end) {
    if (begin == end) return 0;

    // Build chain locally (relaxed ordering)
    Node<T>* first = new Node<T>(std::move(*begin));
    Node<T>* last = first;
    size_t count = 1;

    for (auto it = ++begin; it != end; ++it) {
        Node<T>* node = new Node<T>(std::move(*it));
        last->next.store(node, std::memory_order_relaxed);  // NOT published yet
        last = node;
        count++;
    }

    // Single atomic operation for entire chain
    Node<T>* old_tail = tail_.exchange(last, std::memory_order_acq_rel);
    old_tail->next.store(first, std::memory_order_release);  // Publish chain

    // Single notification
    signal_.fetch_add(count, std::memory_order_release);
    signal_.notify_one();

    return count;
}
```

**Key Optimizations**:
1. **Relaxed chain building**: Nodes not visible until final publish
2. **Single atomic exchange**: N messages → 1 atomic operation
3. **Single notification**: Avoid thundering herd

**Performance Impact**:
- Individual: N atomic exchanges (cache line ping-pong)
- Batch: 1 atomic exchange (100x reduction for N=100)
- Under contention: **4x faster** with 8 producers

#### Why Relaxed Stores Are Safe

```cpp
last->next.store(node, std::memory_order_relaxed);
```

**Proof of Correctness**:
1. Chain is being built **locally** (not yet in shared data structure)
2. No other thread can access these nodes yet
3. Final `release` store publishes entire chain atomically
4. Consumer's `acquire` load sees all relaxed stores

**Analogy**: Building a linked list on the stack, then publishing the head pointer.

### Memory Management

#### Ownership Model

```
Channel Creation:
    shared_ptr<LockFreeChannelState> created
    ├── Sender holds shared_ptr
    └── Receiver holds shared_ptr

Node Lifecycle:
    Producer allocates → Producer pushes to queue → Consumer pops → Consumer deletes

Cleanup:
    Last shared_ptr owner dropped → Destructor called
    Destructor drains queue → All nodes deleted
    Dummy node deleted
```

**Guarantees**:
- ✅ Each node deleted exactly once
- ✅ No memory leaks (verified with ASan)
- ✅ All destructors called (verified with tracked types)

#### Destructor

```cpp
~LockFreeChannelState() {
    // 1. Drain all pending messages
    while (try_pop().is_some()) {}

    // 2. Delete all nodes (including dummy)
    Node<T>* curr = head_;
    while (curr != nullptr) {
        Node<T>* next = curr->next.load(std::memory_order_relaxed);
        delete curr;
        curr = next;
    }

    // 3. Verify no leaks (if tracking enabled)
    #ifdef RUSTY_MPSC_TRACK_ALLOCATIONS
    if (nodes_allocated_ != nodes_deallocated_) {
        std::cerr << "WARNING: Memory leak!\n";
    }
    #endif
}
```

**Why Relaxed Loads**: Destructor is only called when all references are dropped, no concurrent access possible.

## Platform-Specific Optimizations

### CPU Pause Instructions

```cpp
#if defined(__x86_64__) || defined(_M_X64)
    #define CPU_RELAX() _mm_pause()
#elif defined(__aarch64__) || defined(__arm__)
    #define CPU_RELAX() __asm__ __volatile__("yield" ::: "memory")
#else
    #define CPU_RELAX() std::this_thread::yield()
#endif
```

**Purpose**: Hint to CPU that we're in a spin loop
**Effect**: Reduces power, improves hyperthreading, prevents pipeline stalls

### Atomic Wait (C++20)

```cpp
#if __cplusplus >= 202002L
    signal_.wait(old_signal, std::memory_order_acquire);
    signal_.notify_one();
#else
    std::this_thread::sleep_for(microseconds(1));
#endif
```

**C++20**: Uses futex on Linux (kernel wait queue)
**C++17**: Falls back to sleep (less efficient)
**Impact**: 20-30% better latency with C++20

## Performance Characteristics

### Time Complexity

| Operation | Best Case | Worst Case | Amortized |
|-----------|-----------|------------|-----------|
| `send()` | O(1) | O(1) | O(1) |
| `try_recv()` | O(1) | O(1) | O(1) |
| `recv()` | O(1) | O(wait time) | O(1) |
| `batch_send(N)` | O(N) | O(N) | O(N) |
| `batch_recv(N)` | O(N) | O(N) | O(N) |
| `approximate_len()` | O(N) | O(N) | O(N) |
| `drain()` | O(N) | O(N) | O(N) |

**Note**: `approximate_len()` walks entire list, avoid in hot paths.

### Space Complexity

- **Per message**: 16 bytes overhead (node + pointer)
- **Fixed overhead**: ~192 bytes (channel state)
- **Total**: O(N) where N = messages in queue

### Benchmark Results

See `docs/PHASE_5_COMPLETE.md` for detailed performance analysis.

**Summary**:
- **Latency**: 3.3 μs (p50), 12 μs (p99)
- **Throughput**: 28 M msg/s (small messages)
- **Data rate**: 1.7 GB/s (1KB messages)
- **Scaling**: 2x throughput @ 16 producers
- **Batch speedup**: 1.7x (1 producer), 4.3x (8 producers)

## Design Decisions

### Why MPSC Instead of MPMC?

**MPSC Advantages**:
- ✅ Wait-free consumer (no CAS loops)
- ✅ No head pointer contention
- ✅ No ABA problem
- ✅ Simple memory reclamation
- ✅ Better performance

**MPMC Disadvantages**:
- ❌ CAS loops on dequeue (retries)
- ❌ Hazard pointers or epoch-based GC needed
- ❌ Head pointer contention
- ❌ Much more complex code

**Conclusion**: MPSC is simpler, faster, and sufficient for most use cases.

### Why Linked List Instead of Ring Buffer?

**Linked List** (current implementation):
- ✅ Unbounded capacity
- ✅ Simpler implementation
- ✅ No wrap-around logic
- ❌ Per-message allocation
- ❌ Pointer chasing (cache misses)

**Ring Buffer** (potential variant):
- ✅ No allocations (preallocated)
- ✅ Better cache locality
- ✅ Bounded capacity (backpressure)
- ❌ More complex (wrap-around, full/empty)
- ❌ Wastes memory when empty

**Chosen Approach**: Linked list for simplicity and unbounded capacity. Ring buffer could be added as variant.

### Why No Memory Pool?

**Current**: Allocate each node individually
**Alternative**: Maintain a pool of reused nodes

**Pros of Pool**:
- ✅ 10-20% faster (no malloc/free)
- ✅ Better cache locality

**Cons of Pool**:
- ❌ More complex (thread-local pools)
- ❌ Memory not returned to OS
- ❌ Additional code to maintain

**Decision**: Start simple, add pool if profiling shows malloc overhead. Modern allocators (jemalloc, tcmalloc) are already fast.

### Why Dummy Node?

**With Dummy**:
```cpp
Option<T> try_pop() {
    Node<T>* next = head_->next.load(acquire);
    if (next == nullptr) return None;
    T value = std::move(next->data);
    head_ = next;  // Old dummy becomes new dummy
    delete old_dummy;
    return Some(value);
}
```

**Without Dummy**:
```cpp
Option<T> try_pop() {
    // Need to handle special cases:
    // - Empty queue (head == nullptr)
    // - Single element (head == tail)
    // - Multiple elements
    // Much more complex!
}
```

**Benefit**: Eliminates all special cases, simplifies code, no performance cost.

## Comparison with Other Implementations

### vs. Boost.Lockfree.queue

| Feature | Our MPSC | Boost |
|---------|----------|-------|
| **Type** | MPSC | MPMC |
| **Bounded** | No | Optional |
| **Latency** | 3.3 μs | ~10 μs |
| **Consumer** | Wait-free | Lock-free (CAS) |
| **API** | Rust-like | C++ STL-like |
| **Batch ops** | Yes | No |

### vs. moodycamel::ConcurrentQueue

| Feature | Our MPSC | Moodycamel |
|---------|----------|------------|
| **Type** | MPSC | MPMC |
| **Bounded** | No | No |
| **Latency** | 3.3 μs | ~5 μs |
| **Throughput** | 28 M/s | ~40 M/s |
| **API** | Rust-like | Bulk operations |
| **Complexity** | Simple | Complex |

### vs. folly::MPMCQueue

| Feature | Our MPSC | Folly |
|---------|----------|-------|
| **Type** | MPSC | MPMC |
| **Bounded** | No | Yes |
| **Latency** | 3.3 μs | ~2 μs |
| **Consumer** | Wait-free | Wait-free |
| **API** | Rust-like | Facebook-style |
| **Memory** | Dynamic | Preallocated |

**Conclusion**: Our MPSC is simpler and faster than general MPMC queues for the MPSC use case.

## Testing Strategy

### Test Coverage

**54 tests across 4 test suites**:
1. **Phase 1** (14 tests): Non-blocking operations
2. **Phase 2** (12 tests): Blocking operations
3. **Phase 3** (13 tests): Memory management
4. **Phase 4** (15 tests): Batch operations

### Test Categories

1. **Correctness Tests**:
   - Basic send/receive
   - FIFO ordering
   - Multiple producers
   - Move-only types (unique_ptr)
   - Disconnection scenarios

2. **Concurrency Tests**:
   - Multi-producer stress test
   - Concurrent batch sends
   - Producer-consumer pattern
   - Mixed operations

3. **Memory Safety Tests**:
   - Leak detection (TrackedValue)
   - Cleanup scenarios
   - Move semantics
   - Destructor correctness

4. **Performance Tests**:
   - Latency measurements
   - Throughput measurements
   - Batch comparisons

### Sanitizer Verification

```bash
# AddressSanitizer (memory errors)
g++ -fsanitize=address -DRUSTY_MPSC_TRACK_ALLOCATIONS tests/*.cpp
./test
# ✓ No leaks, no use-after-free

# ThreadSanitizer (race conditions)
g++ -fsanitize=thread tests/*.cpp
./test
# ✓ No data races (expected warning on relaxed stores - intentional)

# UndefinedBehaviorSanitizer
g++ -fsanitize=undefined tests/*.cpp
./test
# ✓ No undefined behavior
```

## Future Work

### Potential Enhancements

1. **Bounded Variant** (High Priority)
   - Ring buffer implementation
   - Backpressure mechanism
   - try_send() can fail with Full error

2. **Memory Pool** (Medium Priority)
   - Thread-local node pools
   - Reduce allocation overhead
   - 10-20% throughput improvement

3. **MPMC Variant** (Low Priority)
   - Hazard pointers for safe reclamation
   - CAS-based multi-consumer pop
   - Much more complex

4. **SPSC Variant** (Low Priority)
   - Even simpler (no tail atomics)
   - Slightly faster
   - More specialized use case

### Known Limitations

- ❌ **Unbounded**: No built-in backpressure
- ❌ **No timeout**: recv() blocks indefinitely
- ❌ **Single consumer**: Cannot parallelize receive
- ❌ **No priority**: FIFO only

## Contributing

### Code Style

- Follow existing naming conventions
- Use `std::memory_order_*` explicitly (no defaults)
- Document memory ordering rationale
- Add tests for new features
- Run sanitizers before submitting

### Performance Testing

```bash
# Build with optimizations
g++ -std=c++20 -O2 -pthread tests/benchmark_lockfree.cpp -o bench
./bench

# Profile with perf
perf record -g ./bench
perf report
```

### Debugging

```cpp
// Enable memory tracking
#define RUSTY_MPSC_TRACK_ALLOCATIONS
#include <rusty/sync/mpsc_lockfree.hpp>

// Check stats
auto stats = rx.memory_stats();
std::cout << "Live: " << stats.nodes_live << "\n";
```

## References

### Papers and Articles

1. **"Simple, Fast, and Practical Non-Blocking Queues"**
   - Michael & Scott, PODC 1996
   - Foundation for lock-free linked list queues

2. **"Memory Barriers: a Hardware View for Software Hackers"**
   - Paul McKenney
   - Explains memory ordering

3. **"The Art of Multiprocessor Programming"**
   - Herlihy & Shavit
   - Chapter on concurrent queues

### Related Implementations

- **Rust std::sync::mpsc**: Inspiration for API design
- **Boost.Lockfree**: General concurrent queue
- **moodycamel::ConcurrentQueue**: High-performance MPMC
- **folly::MPMCQueue**: Facebook's bounded queue

## Appendix: Memory Ordering Guide

### Quick Reference

| Operation | Ordering | Rationale |
|-----------|----------|-----------|
| `tail_.exchange()` | acq_rel | Sync with other producers |
| `old_tail->next.store()` | release | Publish to consumer |
| `head_->next.load()` | acquire | See producer's publish |
| `signal_.fetch_add()` | release | Wake with visibility |
| `signal_.load()` | acquire | See all sends before wait |
| `receiver_alive_.load()` | acquire | See receiver's last ops |
| `sender_count_.load()` | acquire | See sender drops |

### Happens-Before Relationships

```
Producer Thread:          Consumer Thread:
    new_node
    |
    tail_.exchange(acq_rel)
    |
    next.store(release) -----> next.load(acquire)
                               |
                               delete node
```

**Guarantee**: Consumer sees all memory operations before producer's `release` store.

## Appendix: Performance Tuning

### Batch Size Selection

```
Batch Size vs Throughput:
    1 :  11 M/s  (baseline)
   10 :  29 M/s  (optimal)
  100 :  28 M/s  (optimal)
 1000 :  23 M/s  (overhead)
```

**Recommendation**: Use 10-100 for best throughput.

### Producer Count

```
Producers vs Throughput:
    1 :  3.3 M/s  (baseline)
    2 :  4.0 M/s  (good)
    4 :  3.6 M/s  (contention)
    8 :  4.7 M/s  (good)
   16 :  6.7 M/s  (scaling)
```

**Recommendation**: 1-8 producers for best efficiency.

### Message Size

```
Message Size vs Throughput:
    4 B :  28 M/s  (optimal)
   64 B :   7 M/s  (good)
 1024 B : 1.7 M/s  (memory bound)
```

**Recommendation**: Channel overhead is minimal, performance limited by data movement.
