// Comprehensive benchmarks for lock-free MPSC channel (Phase 5)

#include <iostream>
#include <thread>
#include <vector>
#include <chrono>
#include <algorithm>
#include <numeric>
#include <iomanip>
#include <cmath>
#include "../include/rusty/sync/mpsc_lockfree.hpp"

using namespace rusty::sync::mpsc::lockfree;
using namespace std::chrono;

// =============================================================================
// Benchmark Infrastructure
// =============================================================================

struct BenchmarkResult {
    std::vector<uint64_t> latencies_ns;  // Individual latencies
    uint64_t total_time_ns;              // Total duration
    size_t message_count;                // Messages sent

    double throughput_msg_per_sec() const {
        return (message_count * 1e9) / total_time_ns;
    }

    uint64_t percentile(double p) const {
        if (latencies_ns.empty()) return 0;
        auto sorted = latencies_ns;
        std::sort(sorted.begin(), sorted.end());
        size_t idx = static_cast<size_t>(p * sorted.size());
        if (idx >= sorted.size()) idx = sorted.size() - 1;
        return sorted[idx];
    }

    double mean_latency_ns() const {
        if (latencies_ns.empty()) return 0.0;
        return std::accumulate(latencies_ns.begin(), latencies_ns.end(), 0.0) / latencies_ns.size();
    }

    double stddev_latency_ns() const {
        if (latencies_ns.size() < 2) return 0.0;
        double mean = mean_latency_ns();
        double sq_sum = 0.0;
        for (auto lat : latencies_ns) {
            sq_sum += (lat - mean) * (lat - mean);
        }
        return std::sqrt(sq_sum / latencies_ns.size());
    }
};

void print_separator() {
    std::cout << std::string(80, '=') << "\n";
}

void print_result_header() {
    std::cout << std::setw(25) << "Metric"
              << std::setw(15) << "Value"
              << std::setw(10) << "Unit" << "\n";
    std::cout << std::string(50, '-') << "\n";
}

void print_metric(const std::string& name, double value, const std::string& unit) {
    std::cout << std::setw(25) << name
              << std::setw(15) << std::fixed << std::setprecision(2) << value
              << std::setw(10) << unit << "\n";
}

// =============================================================================
// Benchmark 1: Latency Distribution (Single Producer, Single Consumer)
// =============================================================================

BenchmarkResult benchmark_latency_distribution(size_t message_count = 10000) {
    std::cout << "\n[Benchmark 1] Latency Distribution\n";
    std::cout << "Measuring end-to-end latency for " << message_count << " messages\n";
    print_separator();

    auto [tx, rx] = channel<uint64_t>();

    BenchmarkResult result;
    result.message_count = message_count;
    result.latencies_ns.reserve(message_count);

    // Producer thread: send messages with timestamps
    std::thread producer([tx = std::move(tx), message_count]() mutable {
        for (size_t i = 0; i < message_count; ++i) {
            auto send_time = high_resolution_clock::now().time_since_epoch().count();
            tx.send(static_cast<uint64_t>(send_time));

            // Small delay to simulate realistic workload
            if (i % 100 == 0) {
                std::this_thread::sleep_for(microseconds(10));
            }
        }
    });

    // Consumer thread: receive and measure latency
    auto total_start = high_resolution_clock::now();

    for (size_t i = 0; i < message_count; ++i) {
        auto recv_result = rx.recv();
        auto recv_time = high_resolution_clock::now().time_since_epoch().count();

        if (recv_result.is_ok()) {
            uint64_t send_time = recv_result.unwrap();
            uint64_t latency = recv_time - send_time;
            result.latencies_ns.push_back(latency);
        }
    }

    auto total_end = high_resolution_clock::now();
    result.total_time_ns = duration_cast<nanoseconds>(total_end - total_start).count();

    producer.join();

    // Print results
    print_result_header();
    print_metric("Message count", static_cast<double>(message_count), "msgs");
    print_metric("Total time", result.total_time_ns / 1e6, "ms");
    print_metric("Throughput", result.throughput_msg_per_sec() / 1e6, "M msg/s");
    print_metric("Mean latency", result.mean_latency_ns() / 1e3, "μs");
    print_metric("Std dev", result.stddev_latency_ns() / 1e3, "μs");
    print_metric("p50 latency", result.percentile(0.50) / 1e3, "μs");
    print_metric("p95 latency", result.percentile(0.95) / 1e3, "μs");
    print_metric("p99 latency", result.percentile(0.99) / 1e3, "μs");
    print_metric("p99.9 latency", result.percentile(0.999) / 1e3, "μs");

    return result;
}

// =============================================================================
// Benchmark 2: Throughput Scaling (Varying Producer Count)
// =============================================================================

struct ThroughputResult {
    size_t producer_count;
    double throughput_msg_per_sec;
    uint64_t duration_ms;
};

std::vector<ThroughputResult> benchmark_throughput_scaling(
    const std::vector<size_t>& producer_counts,
    size_t messages_per_producer = 10000
) {
    std::cout << "\n[Benchmark 2] Throughput Scaling\n";
    std::cout << "Measuring throughput with varying producer counts\n";
    std::cout << "Messages per producer: " << messages_per_producer << "\n";
    print_separator();

    std::vector<ThroughputResult> results;

    for (size_t num_producers : producer_counts) {
        auto [tx, rx] = channel<int>();

        auto start = high_resolution_clock::now();

        // Producer threads
        std::vector<std::thread> producers;
        for (size_t p = 0; p < num_producers; ++p) {
            auto tx_clone = tx.clone();
            producers.emplace_back([tx_clone = std::move(tx_clone), messages_per_producer, p]() mutable {
                for (size_t i = 0; i < messages_per_producer; ++i) {
                    tx_clone.send(static_cast<int>(p * 1000000 + i));
                }
            });
        }

        tx = Sender<int>(nullptr);  // Drop original sender

        // Consumer thread
        size_t total_messages = num_producers * messages_per_producer;
        std::thread consumer([rx = std::move(rx), total_messages]() mutable {
            for (size_t i = 0; i < total_messages; ++i) {
                rx.recv();
            }
        });

        for (auto& t : producers) {
            t.join();
        }
        consumer.join();

        auto end = high_resolution_clock::now();
        auto duration_ms = duration_cast<milliseconds>(end - start).count();

        ThroughputResult result;
        result.producer_count = num_producers;
        result.throughput_msg_per_sec = (total_messages * 1000.0) / duration_ms;
        result.duration_ms = duration_ms;

        results.push_back(result);
    }

    // Print results
    std::cout << std::setw(15) << "Producers"
              << std::setw(20) << "Throughput"
              << std::setw(15) << "Duration" << "\n";
    std::cout << std::string(50, '-') << "\n";

    for (const auto& r : results) {
        std::cout << std::setw(15) << r.producer_count
                  << std::setw(20) << std::fixed << std::setprecision(2)
                  << (r.throughput_msg_per_sec / 1e6) << " M msg/s"
                  << std::setw(15) << r.duration_ms << " ms\n";
    }

    return results;
}

// =============================================================================
// Benchmark 3: Batch Size Impact
// =============================================================================

struct BatchResult {
    size_t batch_size;
    double throughput_msg_per_sec;
    uint64_t duration_us;
};

std::vector<BatchResult> benchmark_batch_sizes(
    const std::vector<size_t>& batch_sizes,
    size_t total_messages = 100000
) {
    std::cout << "\n[Benchmark 3] Batch Size Impact\n";
    std::cout << "Comparing batch send performance with different batch sizes\n";
    std::cout << "Total messages: " << total_messages << "\n";
    print_separator();

    std::vector<BatchResult> results;

    for (size_t batch_size : batch_sizes) {
        auto [tx, rx] = channel<int>();

        // Prepare batches
        size_t num_batches = total_messages / batch_size;

        auto start = high_resolution_clock::now();

        // Send batches
        for (size_t b = 0; b < num_batches; ++b) {
            std::vector<int> batch;
            batch.reserve(batch_size);
            for (size_t i = 0; i < batch_size; ++i) {
                batch.push_back(static_cast<int>(b * batch_size + i));
            }
            tx.send_batch(batch);
        }

        // Receive all
        for (size_t i = 0; i < total_messages; ++i) {
            rx.try_recv();
        }

        auto end = high_resolution_clock::now();
        auto duration_us = duration_cast<microseconds>(end - start).count();

        BatchResult result;
        result.batch_size = batch_size;
        result.throughput_msg_per_sec = (total_messages * 1e6) / duration_us;
        result.duration_us = duration_us;

        results.push_back(result);
    }

    // Print results
    std::cout << std::setw(15) << "Batch Size"
              << std::setw(20) << "Throughput"
              << std::setw(15) << "Duration" << "\n";
    std::cout << std::string(50, '-') << "\n";

    for (const auto& r : results) {
        std::cout << std::setw(15) << r.batch_size
                  << std::setw(20) << std::fixed << std::setprecision(2)
                  << (r.throughput_msg_per_sec / 1e6) << " M msg/s"
                  << std::setw(15) << r.duration_us << " μs\n";
    }

    return results;
}

// =============================================================================
// Benchmark 4: Message Size Impact
// =============================================================================

template<size_t N>
struct LargeMessage {
    char data[N];
    static constexpr bool is_send = true;
};

struct MessageSizeResult {
    size_t message_size;
    double throughput_msg_per_sec;
    double throughput_mb_per_sec;
    uint64_t duration_us;
};

std::vector<MessageSizeResult> benchmark_message_sizes(size_t message_count = 10000) {
    std::cout << "\n[Benchmark 4] Message Size Impact\n";
    std::cout << "Comparing performance with different message sizes\n";
    std::cout << "Message count: " << message_count << "\n";
    print_separator();

    std::vector<MessageSizeResult> results;

    // Small message (4 bytes)
    {
        auto [tx, rx] = channel<int>();
        auto start = high_resolution_clock::now();

        for (size_t i = 0; i < message_count; ++i) {
            tx.send(static_cast<int>(i));
        }
        for (size_t i = 0; i < message_count; ++i) {
            rx.try_recv();
        }

        auto end = high_resolution_clock::now();
        auto duration_us = duration_cast<microseconds>(end - start).count();

        MessageSizeResult result;
        result.message_size = sizeof(int);
        result.throughput_msg_per_sec = (message_count * 1e6) / duration_us;
        result.throughput_mb_per_sec = (message_count * sizeof(int) * 1e6) / (duration_us * 1024 * 1024);
        result.duration_us = duration_us;
        results.push_back(result);
    }

    // Medium message (64 bytes)
    {
        auto [tx, rx] = channel<LargeMessage<64>>();
        auto start = high_resolution_clock::now();

        for (size_t i = 0; i < message_count; ++i) {
            tx.send(LargeMessage<64>{});
        }
        for (size_t i = 0; i < message_count; ++i) {
            rx.try_recv();
        }

        auto end = high_resolution_clock::now();
        auto duration_us = duration_cast<microseconds>(end - start).count();

        MessageSizeResult result;
        result.message_size = sizeof(LargeMessage<64>);
        result.throughput_msg_per_sec = (message_count * 1e6) / duration_us;
        result.throughput_mb_per_sec = (message_count * sizeof(LargeMessage<64>) * 1e6) / (duration_us * 1024 * 1024);
        result.duration_us = duration_us;
        results.push_back(result);
    }

    // Large message (1 KB)
    {
        auto [tx, rx] = channel<LargeMessage<1024>>();
        auto start = high_resolution_clock::now();

        for (size_t i = 0; i < message_count; ++i) {
            tx.send(LargeMessage<1024>{});
        }
        for (size_t i = 0; i < message_count; ++i) {
            rx.try_recv();
        }

        auto end = high_resolution_clock::now();
        auto duration_us = duration_cast<microseconds>(end - start).count();

        MessageSizeResult result;
        result.message_size = sizeof(LargeMessage<1024>);
        result.throughput_msg_per_sec = (message_count * 1e6) / duration_us;
        result.throughput_mb_per_sec = (message_count * sizeof(LargeMessage<1024>) * 1e6) / (duration_us * 1024 * 1024);
        result.duration_us = duration_us;
        results.push_back(result);
    }

    // Print results
    std::cout << std::setw(12) << "Msg Size"
              << std::setw(20) << "Msg Throughput"
              << std::setw(20) << "Data Throughput"
              << std::setw(12) << "Duration" << "\n";
    std::cout << std::string(64, '-') << "\n";

    for (const auto& r : results) {
        std::cout << std::setw(12) << r.message_size << " B"
                  << std::setw(20) << std::fixed << std::setprecision(2)
                  << (r.throughput_msg_per_sec / 1e6) << " M/s"
                  << std::setw(20) << std::fixed << std::setprecision(2)
                  << r.throughput_mb_per_sec << " MB/s"
                  << std::setw(12) << r.duration_us << " μs\n";
    }

    return results;
}

// =============================================================================
// Benchmark 5: Individual vs Batch Operations
// =============================================================================

void benchmark_individual_vs_batch(size_t message_count = 10000) {
    std::cout << "\n[Benchmark 5] Individual vs Batch Operations\n";
    std::cout << "Comparing individual send vs batch send performance\n";
    std::cout << "Message count: " << message_count << "\n";
    print_separator();

    // Individual sends
    uint64_t individual_time_us;
    {
        auto [tx, rx] = channel<int>();
        auto start = high_resolution_clock::now();

        for (size_t i = 0; i < message_count; ++i) {
            tx.send(static_cast<int>(i));
        }
        for (size_t i = 0; i < message_count; ++i) {
            rx.try_recv();
        }

        auto end = high_resolution_clock::now();
        individual_time_us = duration_cast<microseconds>(end - start).count();
    }

    // Batch sends (batch size 100)
    uint64_t batch_time_us;
    {
        auto [tx, rx] = channel<int>();
        auto start = high_resolution_clock::now();

        size_t batch_size = 100;
        size_t num_batches = message_count / batch_size;

        for (size_t b = 0; b < num_batches; ++b) {
            std::vector<int> batch;
            batch.reserve(batch_size);
            for (size_t i = 0; i < batch_size; ++i) {
                batch.push_back(static_cast<int>(b * batch_size + i));
            }
            tx.send_batch(batch);
        }

        for (size_t i = 0; i < message_count; ++i) {
            rx.try_recv();
        }

        auto end = high_resolution_clock::now();
        batch_time_us = duration_cast<microseconds>(end - start).count();
    }

    // Print comparison
    print_result_header();
    print_metric("Individual send time", individual_time_us, "μs");
    print_metric("Batch send time", batch_time_us, "μs");
    print_metric("Speedup", static_cast<double>(individual_time_us) / batch_time_us, "x");
    print_metric("Individual throughput", (message_count * 1e6) / individual_time_us / 1e6, "M msg/s");
    print_metric("Batch throughput", (message_count * 1e6) / batch_time_us / 1e6, "M msg/s");
}

// =============================================================================
// Benchmark 6: Contention Under Multi-Producer Load
// =============================================================================

void benchmark_contention(size_t num_producers = 8, size_t msgs_per_producer = 10000) {
    std::cout << "\n[Benchmark 6] Contention Under Multi-Producer Load\n";
    std::cout << "Comparing individual vs batch under high contention\n";
    std::cout << "Producers: " << num_producers << ", Messages/producer: " << msgs_per_producer << "\n";
    print_separator();

    // Individual sends under contention
    uint64_t individual_time_ms;
    {
        auto [tx, rx] = channel<int>();
        auto start = high_resolution_clock::now();

        std::vector<std::thread> producers;
        for (size_t p = 0; p < num_producers; ++p) {
            auto tx_clone = tx.clone();
            producers.emplace_back([tx_clone = std::move(tx_clone), msgs_per_producer, p]() mutable {
                for (size_t i = 0; i < msgs_per_producer; ++i) {
                    tx_clone.send(static_cast<int>(p * 1000000 + i));
                }
            });
        }

        tx = Sender<int>(nullptr);

        size_t total_msgs = num_producers * msgs_per_producer;
        std::thread consumer([rx = std::move(rx), total_msgs]() mutable {
            for (size_t i = 0; i < total_msgs; ++i) {
                rx.try_recv();
            }
        });

        for (auto& t : producers) t.join();
        consumer.join();

        auto end = high_resolution_clock::now();
        individual_time_ms = duration_cast<milliseconds>(end - start).count();
    }

    // Batch sends under contention
    uint64_t batch_time_ms;
    {
        auto [tx, rx] = channel<int>();
        auto start = high_resolution_clock::now();

        std::vector<std::thread> producers;
        for (size_t p = 0; p < num_producers; ++p) {
            auto tx_clone = tx.clone();
            producers.emplace_back([tx_clone = std::move(tx_clone), msgs_per_producer, p]() mutable {
                // Send in batches of 100
                size_t batch_size = 100;
                for (size_t b = 0; b < msgs_per_producer / batch_size; ++b) {
                    std::vector<int> batch;
                    batch.reserve(batch_size);
                    for (size_t i = 0; i < batch_size; ++i) {
                        batch.push_back(static_cast<int>(p * 1000000 + b * batch_size + i));
                    }
                    tx_clone.send_batch(batch);
                }
            });
        }

        tx = Sender<int>(nullptr);

        size_t total_msgs = num_producers * msgs_per_producer;
        std::thread consumer([rx = std::move(rx), total_msgs]() mutable {
            for (size_t i = 0; i < total_msgs; ++i) {
                rx.try_recv();
            }
        });

        for (auto& t : producers) t.join();
        consumer.join();

        auto end = high_resolution_clock::now();
        batch_time_ms = duration_cast<milliseconds>(end - start).count();
    }

    // Print comparison
    print_result_header();
    print_metric("Individual sends time", individual_time_ms, "ms");
    print_metric("Batch sends time", batch_time_ms, "ms");
    print_metric("Speedup", static_cast<double>(individual_time_ms) / batch_time_ms, "x");

    size_t total_msgs = num_producers * msgs_per_producer;
    print_metric("Individual throughput", (total_msgs * 1000.0) / individual_time_ms / 1e6, "M msg/s");
    print_metric("Batch throughput", (total_msgs * 1000.0) / batch_time_ms / 1e6, "M msg/s");
}

// =============================================================================
// Main
// =============================================================================

int main() {
    std::cout << "\n";
    print_separator();
    std::cout << "Lock-Free MPSC Channel - Comprehensive Benchmarks (Phase 5)\n";
    print_separator();

    std::cout << "\nBuild Info:\n";
    std::cout << "  C++ Standard: " << __cplusplus << "\n";
    #if __cplusplus >= 202002L
        std::cout << "  atomic::wait support: YES (C++20)\n";
    #else
        std::cout << "  atomic::wait support: NO (using sleep fallback)\n";
    #endif
    std::cout << "  Hardware concurrency: " << std::thread::hardware_concurrency() << " threads\n";

    // Run all benchmarks
    benchmark_latency_distribution(10000);

    benchmark_throughput_scaling({1, 2, 4, 8, 16}, 10000);

    benchmark_batch_sizes({1, 10, 100, 1000}, 100000);

    benchmark_message_sizes(10000);

    benchmark_individual_vs_batch(10000);

    benchmark_contention(8, 10000);

    // Final summary
    std::cout << "\n";
    print_separator();
    std::cout << "All benchmarks completed successfully!\n";
    print_separator();
    std::cout << "\n";

    return 0;
}
