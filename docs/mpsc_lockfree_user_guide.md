# Lock-Free MPSC Channel - User Guide

## Overview

A high-performance, lock-free Multi-Producer Single-Consumer (MPSC) channel for C++17/C++20. Provides Rust-like channel semantics with excellent throughput and low latency.

**Key Features**:
- 🚀 **High Performance**: 28M msg/s throughput, 3.3μs p50 latency
- 🔒 **Lock-Free**: No mutexes, wait-free consumer operations
- 📦 **Batch Operations**: 4x speedup under contention
- 💾 **Memory Safe**: Zero leaks, verified with sanitizers
- 🎯 **Predictable**: Tight latency distribution, no performance collapse
- 🔧 **Easy to Use**: Simple, intuitive API

## Quick Start

### Basic Usage

```cpp
#include <rusty/sync/mpsc_lockfree.hpp>

using namespace rusty::sync::mpsc::lockfree;

// Create a channel
auto [tx, rx] = channel<int>();

// Send messages
tx.send(42);
tx.send(100);

// Receive messages
auto result = rx.recv();  // Blocking
if (result.is_ok()) {
    int value = result.unwrap();  // 42
}

auto result2 = rx.try_recv();  // Non-blocking
if (result2.is_ok()) {
    int value = result2.unwrap();  // 100
}
```

### Multi-Producer Example

```cpp
auto [tx, rx] = channel<std::string>();

// Clone sender for multiple producers
std::vector<std::thread> producers;
for (int i = 0; i < 4; ++i) {
    auto tx_clone = tx.clone();
    producers.emplace_back([tx_clone = std::move(tx_clone), i]() mutable {
        tx_clone.send("Message from producer " + std::to_string(i));
    });
}

// Drop original sender
tx = Sender<std::string>(nullptr);

// Receive all messages
for (auto& t : producers) {
    t.join();
}

while (auto msg = rx.try_recv()) {
    if (msg.is_ok()) {
        std::cout << msg.unwrap() << "\n";
    }
}
```

## API Reference

### Creating Channels

```cpp
// Create a channel for type T
auto [tx, rx] = channel<T>();
```

**Type Requirements**:
- `T` must be marked as `Send` (thread-safe to transfer)
- Primitives (int, float, etc.) are Send by default
- Custom types need explicit marking

**Marking Custom Types as Send**:
```cpp
struct MyData {
    int x;
    std::string s;
    static constexpr bool is_send = true;  // Mark as Send
};

// Or use macro
RUSTY_MARK_SEND(MyData);
```

### Sender API

#### Send Operations

```cpp
// Blocking send (never fails unless receiver dropped)
Result<Unit, TrySendError> send(T value) const;

// Non-blocking send
Result<Unit, TrySendError> try_send(T value) const;

// Batch send (single atomic operation)
template<typename Iterator>
size_t send_batch(Iterator begin, Iterator end) const;

template<typename Container>
size_t send_batch(Container&& container) const;
```

**Examples**:
```cpp
// Individual send
tx.send(42);

// Batch send from vector
std::vector<int> batch = {1, 2, 3, 4, 5};
size_t sent = tx.send_batch(batch);

// Batch send from iterators
size_t sent = tx.send_batch(batch.begin(), batch.end());
```

#### Sender Management

```cpp
// Clone sender for another producer
Sender<T> clone() const;

// Check if receiver is still alive
bool is_connected() const;
```

**Example**:
```cpp
auto tx2 = tx.clone();  // Create second sender
std::thread producer([tx2 = std::move(tx2)]() mutable {
    tx2.send(100);
});
```

### Receiver API

#### Receive Operations

```cpp
// Blocking receive (waits for message)
Result<T, RecvError> recv();

// Non-blocking receive
Result<T, TryRecvError> try_recv();

// Batch receive (up to max_count messages)
std::vector<T> recv_batch(size_t max_count);
```

**Examples**:
```cpp
// Blocking receive
auto result = rx.recv();
if (result.is_ok()) {
    process(result.unwrap());
}

// Non-blocking receive
while (auto msg = rx.try_recv()) {
    if (msg.is_ok()) {
        process(msg.unwrap());
    } else if (msg.unwrap_err() == TryRecvError::Empty) {
        break;  // No more messages
    }
}

// Batch receive
auto messages = rx.recv_batch(100);
for (const auto& msg : messages) {
    process(msg);
}
```

#### Helper Methods

```cpp
// Check if queue is empty
bool is_empty() const;

// Check if all senders disconnected
bool is_disconnected() const;

// Drain all pending messages (returns count)
size_t drain();

// Get approximate queue length (O(n), use sparingly)
size_t approximate_len() const;
```

**Examples**:
```cpp
// Check before receive
if (!rx.is_empty()) {
    auto msg = rx.try_recv();
    // ...
}

// Wait for disconnection
while (!rx.is_disconnected()) {
    auto msg = rx.recv();
    // ...
}

// Discard pending messages
size_t discarded = rx.drain();
std::cout << "Discarded " << discarded << " messages\n";
```

#### Memory Tracking (Debug Only)

```cpp
#ifdef RUSTY_MPSC_TRACK_ALLOCATIONS
struct MemoryStats {
    size_t nodes_allocated;
    size_t nodes_deallocated;
    size_t nodes_live;
};

MemoryStats memory_stats() const;
#endif
```

**Example**:
```cpp
// Compile with -DRUSTY_MPSC_TRACK_ALLOCATIONS
#ifdef RUSTY_MPSC_TRACK_ALLOCATIONS
auto stats = rx.memory_stats();
std::cout << "Live nodes: " << stats.nodes_live << "\n";
#endif
```

### Error Handling

```cpp
// Send errors
enum class TrySendError {
    Disconnected,  // Receiver dropped
};

// Receive errors
enum class RecvError {
    Disconnected,  // All senders dropped
};

enum class TryRecvError {
    Empty,         // No messages available
    Disconnected,  // All senders dropped
};
```

**Pattern Matching**:
```cpp
auto result = rx.try_recv();
if (result.is_ok()) {
    T value = result.unwrap();
    // Use value
} else {
    auto err = result.unwrap_err();
    if (err == TryRecvError::Empty) {
        // No messages
    } else if (err == TryRecvError::Disconnected) {
        // All senders gone
    }
}
```

## Common Patterns

### Producer-Consumer

```cpp
auto [tx, rx] = channel<Task>();

// Producer thread
std::thread producer([tx = std::move(tx)]() mutable {
    while (true) {
        Task task = get_task();
        if (task.is_done()) break;
        tx.send(std::move(task));
    }
});

// Consumer thread
std::thread consumer([rx = std::move(rx)]() mutable {
    while (auto result = rx.recv()) {
        if (result.is_ok()) {
            process_task(result.unwrap());
        } else {
            break;  // All producers finished
        }
    }
});

producer.join();
consumer.join();
```

### Multiple Producers

```cpp
auto [tx, rx] = channel<Event>();

std::vector<std::thread> producers;
for (int i = 0; i < 8; ++i) {
    auto tx_clone = tx.clone();
    producers.emplace_back([tx_clone = std::move(tx_clone), i]() mutable {
        for (int j = 0; j < 1000; ++j) {
            tx_clone.send(Event{i, j});
        }
    });
}

tx = Sender<Event>(nullptr);  // Drop original

// Consumer
for (auto& t : producers) {
    t.join();
}

while (!rx.is_disconnected()) {
    auto event = rx.try_recv();
    if (event.is_ok()) {
        handle_event(event.unwrap());
    }
}
```

### Batch Processing

```cpp
auto [tx, rx] = channel<LogEntry>();

// Producer: batch send log entries
std::thread logger([tx = std::move(tx)]() mutable {
    while (true) {
        std::vector<LogEntry> batch = collect_logs(100);
        if (batch.empty()) break;
        tx.send_batch(batch);
        std::this_thread::sleep_for(milliseconds(10));
    }
});

// Consumer: batch receive and process
std::thread writer([rx = std::move(rx)]() mutable {
    while (!rx.is_disconnected()) {
        auto entries = rx.recv_batch(1000);
        if (!entries.empty()) {
            write_to_disk(entries);
        }
    }
});

logger.join();
writer.join();
```

### Graceful Shutdown

```cpp
auto [tx, rx] = channel<Request>();

// Shutdown signal
std::atomic<bool> shutdown{false};

// Producer respects shutdown
std::thread producer([tx = std::move(tx), &shutdown]() mutable {
    while (!shutdown.load()) {
        if (auto req = get_request()) {
            tx.send(*req);
        }
    }
    // tx dropped here, signals consumer
});

// Consumer processes remaining messages
std::thread consumer([rx = std::move(rx)]() mutable {
    while (auto result = rx.recv()) {
        if (result.is_ok()) {
            process(result.unwrap());
        } else {
            break;  // Producer finished
        }
    }
});

// Signal shutdown
std::this_thread::sleep_for(seconds(5));
shutdown.store(true);

producer.join();
consumer.join();
```

## Performance Guidelines

### When to Use Batch Operations

**Use Batch Send When**:
- ✅ Multiple messages available at once
- ✅ High contention (many producers)
- ✅ Throughput > latency priority
- ✅ Message bursts (file reading, network packets)

**Use Individual Send When**:
- ✅ Messages arrive one at a time
- ✅ Low latency critical
- ✅ Single producer
- ✅ Immediate processing needed

**Performance Impact**:
- Single producer: 1.7x speedup with batching
- 8 producers: **4.3x speedup** with batching
- Optimal batch size: 10-100 messages

### Memory Considerations

**Per-Message Overhead**:
- 16 bytes per message in queue (node + pointer)
- No overhead for empty queue
- Messages are moved, not copied

**Memory Usage**:
```cpp
// Small messages: minimal overhead
channel<int>();  // 4 bytes data + 16 bytes node = 20 bytes/msg

// Large messages: data dominates
channel<LargeStruct>();  // sizeof(LargeStruct) + 16 bytes node
```

### Latency vs Throughput

**Low Latency Mode** (individual operations):
- p50: 3.3 μs
- p99: 12 μs
- Throughput: 22-28 M msg/s (single producer)

**High Throughput Mode** (batch operations):
- Batch latency: ~270 μs for 10K messages (27 ns/msg)
- Throughput: 37 M msg/s (single producer)
- Throughput: 20 M msg/s (8 producers)

**Recommendation**: Use individual sends for latency-sensitive paths, batching for high-volume paths.

## Troubleshooting

### Common Issues

**Issue**: `static assertion failed: Channel type T must be Send`
```cpp
// Problem: Type not marked as Send
struct MyType { int x; };
auto [tx, rx] = channel<MyType>();  // ERROR

// Solution: Mark type as Send
struct MyType {
    int x;
    static constexpr bool is_send = true;
};
```

**Issue**: Use after move
```cpp
auto [tx, rx] = channel<int>();
auto tx2 = std::move(tx);
tx.send(42);  // ERROR: tx has been moved
```

**Issue**: Receiver blocks forever
```cpp
// Problem: All senders dropped before send
auto [tx, rx] = channel<int>();
tx = Sender<int>(nullptr);  // Drop sender
auto msg = rx.recv();  // Returns Err(Disconnected)

// Solution: Check for disconnection
while (auto msg = rx.recv()) {
    if (msg.is_err()) {
        break;  // All senders gone
    }
}
```

### Performance Issues

**Low Throughput with Multiple Producers**:
- Use batch operations (`send_batch`) instead of individual sends
- Expected: 4x improvement with 8 producers

**High Memory Usage**:
- Call `drain()` periodically if receiver is slower than producers
- Messages accumulate in unbounded queue
- Consider rate limiting producers

**High Latency**:
- Check if using C++17 (sleep fallback) instead of C++20 (atomic::wait)
- Compile with C++20 for 20-30% latency improvement
- Reduce batch size if batching (smaller batches = lower latency)

## Platform Support

### C++ Standards

**C++17** (minimum):
- Fully supported
- Uses `sleep_for()` fallback for blocking
- Good performance, slightly higher latency

**C++20** (recommended):
- Uses `atomic::wait/notify` for efficient blocking
- 20-30% better latency
- Native futex support on Linux

### Compilers

- ✅ GCC 7+ (C++17), GCC 10+ (C++20)
- ✅ Clang 5+ (C++17), Clang 10+ (C++20)
- ✅ MSVC 2017+ (C++17), MSVC 2019+ (C++20)

### Operating Systems

- ✅ Linux (excellent performance, futex support)
- ✅ macOS (good performance)
- ✅ Windows (good performance, WaitOnAddress in C++20)

## Best Practices

### Do's ✅

1. **Use batch operations for high-volume scenarios**
   ```cpp
   std::vector<Message> batch = collect_messages();
   tx.send_batch(batch);  // Much faster
   ```

2. **Clone senders for each producer thread**
   ```cpp
   auto tx_clone = tx.clone();
   std::thread producer([tx_clone = std::move(tx_clone)]() { ... });
   ```

3. **Check for disconnection in loops**
   ```cpp
   while (!rx.is_disconnected()) { ... }
   ```

4. **Use try_recv() for non-blocking scenarios**
   ```cpp
   if (auto msg = rx.try_recv()) { ... }
   ```

5. **Mark custom types as Send**
   ```cpp
   struct MyData {
       static constexpr bool is_send = true;
   };
   ```

### Don'ts ❌

1. **Don't call approximate_len() in hot paths**
   ```cpp
   // BAD: O(n) operation
   while (true) {
       if (rx.approximate_len() > 0) { ... }
   }

   // GOOD: Use is_empty() or try_recv()
   while (!rx.is_empty()) {
       auto msg = rx.try_recv();
   }
   ```

2. **Don't share receivers between threads**
   ```cpp
   // ERROR: Receiver is not thread-safe
   std::thread t1([&rx]() { rx.recv(); });
   std::thread t2([&rx]() { rx.recv(); });  // Race condition!
   ```

3. **Don't use after move**
   ```cpp
   auto tx2 = std::move(tx);
   tx.send(42);  // ERROR: tx is invalid
   ```

4. **Don't forget to drop original sender**
   ```cpp
   for (int i = 0; i < 4; ++i) {
       auto tx_clone = tx.clone();
       // ...
   }
   // IMPORTANT: Drop original sender
   tx = Sender<T>(nullptr);
   ```

## Examples

See the test files for comprehensive examples:
- `tests/test_lockfree_channel.cpp` - Basic usage
- `tests/test_lockfree_blocking.cpp` - Blocking operations
- `tests/test_lockfree_batch.cpp` - Batch operations
- `tests/benchmark_lockfree.cpp` - Performance examples

## Further Reading

- [Developer Guide](mpsc_lockfree_developer_guide.md) - Implementation details
- [Benchmark Results](PHASE_5_COMPLETE.md) - Performance analysis
- Header file: `include/rusty/sync/mpsc_lockfree.hpp`
