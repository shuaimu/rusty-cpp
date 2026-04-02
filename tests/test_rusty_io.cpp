// Tests for rusty::io module
// Build: g++ -std=c++20 -I include -o test_rusty_io tests/test_rusty_io.cpp
// Run:   ./test_rusty_io

#include <rusty/io.hpp>
#include <cassert>
#include <iostream>
#include <cstring>
#include <array>

using namespace rusty::io;

// ── Error tests ────────────────────────────────────────

void test_error_creation() {
    Error e1(Error::Kind::NotFound, "file not found");
    assert(e1.kind() == Error::Kind::NotFound);
    assert(e1.to_string() == "file not found");

    Error e2("generic error");
    assert(e2.kind() == Error::Kind::Other);

    std::cout << "  test_error_creation PASSED" << std::endl;
}

// ── Result tests ───────────────────────────────────────

void test_result_ok() {
    auto r = Result<int>::ok(42);
    assert(r.is_ok());
    assert(!r.is_err());
    assert(r.unwrap() == 42);

    std::cout << "  test_result_ok PASSED" << std::endl;
}

void test_result_err() {
    auto r = Result<int>::err(Error("bad"));
    assert(!r.is_ok());
    assert(r.is_err());
    assert(r.unwrap_err().to_string() == "bad");

    std::cout << "  test_result_err PASSED" << std::endl;
}

void test_result_void() {
    auto ok = Result<void>::ok();
    assert(ok.is_ok());

    auto err = Result<void>::err(Error("failed"));
    assert(err.is_err());

    std::cout << "  test_result_void PASSED" << std::endl;
}

// ── SeekFrom tests ─────────────────────────────────────

void test_seekfrom() {
    auto start = SeekFrom::Start(100);
    assert(start.tag() == SeekFrom::StartTag);
    assert(start.offset() == 100);

    auto end = SeekFrom::End(-10);
    assert(end.tag() == SeekFrom::EndTag);
    assert(end.offset() == -10);

    auto current = SeekFrom::Current(5);
    assert(current.tag() == SeekFrom::CurrentTag);
    assert(current.offset() == 5);

    std::cout << "  test_seekfrom PASSED" << std::endl;
}

// ── Cursor tests ───────────────────────────────────────

void test_cursor_read() {
    std::vector<uint8_t> data = {0, 1, 2, 3, 4, 5, 6, 7};
    auto cursor = Cursor<std::vector<uint8_t>>::new_(data);

    uint8_t buf[4];
    auto result = cursor.read(std::span<uint8_t>(buf, 4));
    assert(result.is_ok());
    assert(result.unwrap() == 4);
    assert(buf[0] == 0 && buf[1] == 1 && buf[2] == 2 && buf[3] == 3);

    // Read remaining
    result = cursor.read(std::span<uint8_t>(buf, 4));
    assert(result.is_ok());
    assert(result.unwrap() == 4);
    assert(buf[0] == 4 && buf[1] == 5 && buf[2] == 6 && buf[3] == 7);

    // Read at end
    result = cursor.read(std::span<uint8_t>(buf, 4));
    assert(result.is_ok());
    assert(result.unwrap() == 0);

    std::cout << "  test_cursor_read PASSED" << std::endl;
}

void test_cursor_write() {
    std::vector<uint8_t> data(8, 0);
    auto cursor = Cursor<std::vector<uint8_t>>::new_(std::move(data));

    uint8_t write_data[] = {10, 20, 30};
    auto result = cursor.write(std::span<const uint8_t>(write_data, 3));
    assert(result.is_ok());
    assert(result.unwrap() == 3);

    // Verify written data
    assert(cursor.get_ref()[0] == 10);
    assert(cursor.get_ref()[1] == 20);
    assert(cursor.get_ref()[2] == 30);
    assert(cursor.get_ref()[3] == 0); // unchanged

    std::cout << "  test_cursor_write PASSED" << std::endl;
}

void test_cursor_seek() {
    std::vector<uint8_t> data = {0, 1, 2, 3, 4, 5, 6, 7};
    auto cursor = Cursor<std::vector<uint8_t>>::new_(data);

    // Read first 4 bytes
    uint8_t buf[4];
    cursor.read(std::span<uint8_t>(buf, 4));
    assert(cursor.position() == 4);

    // Seek back to start
    auto result = cursor.seek(SeekFrom::Start(0));
    assert(result.is_ok());
    assert(result.unwrap() == 0);
    assert(cursor.position() == 0);

    // Re-read from start
    cursor.read(std::span<uint8_t>(buf, 4));
    assert(buf[0] == 0 && buf[1] == 1);

    // Seek from current
    cursor.seek(SeekFrom::Current(-2));
    assert(cursor.position() == 2);

    // Seek from end
    cursor.seek(SeekFrom::End(-3));
    assert(cursor.position() == 5);

    // Read from position 5
    cursor.read(std::span<uint8_t>(buf, 3));
    assert(buf[0] == 5 && buf[1] == 6 && buf[2] == 7);

    std::cout << "  test_cursor_seek PASSED" << std::endl;
}

void test_cursor_read_write_seek_combined() {
    // Simulates the either crate's seek test pattern
    std::vector<uint8_t> mockdata(256);
    for (int i = 0; i < 256; i++) {
        mockdata[i] = static_cast<uint8_t>(i);
    }

    auto cursor = Cursor<std::vector<uint8_t>>::new_(mockdata);

    // Read first 16 bytes
    uint8_t buf[16];
    auto r = cursor.read(std::span<uint8_t>(buf, 16));
    assert(r.unwrap() == 16);
    assert(buf[0] == 0 && buf[15] == 15);

    // Read next 16 bytes
    r = cursor.read(std::span<uint8_t>(buf, 16));
    assert(r.unwrap() == 16);
    assert(buf[0] == 16 && buf[15] == 31);

    // Seek back to start
    cursor.seek(SeekFrom::Start(0));

    // Re-read should give first 16 bytes again
    r = cursor.read(std::span<uint8_t>(buf, 16));
    assert(r.unwrap() == 16);
    assert(buf[0] == 0 && buf[15] == 15);

    std::cout << "  test_cursor_read_write_seek_combined PASSED" << std::endl;
}

void test_cursor_position() {
    std::vector<uint8_t> data(10, 0);
    auto cursor = Cursor<std::vector<uint8_t>>::new_(data);

    assert(cursor.position() == 0);

    cursor.set_position(5);
    assert(cursor.position() == 5);

    std::cout << "  test_cursor_position PASSED" << std::endl;
}

void test_cursor_into_inner() {
    std::vector<uint8_t> data = {1, 2, 3};
    auto cursor = Cursor<std::vector<uint8_t>>::new_(data);

    auto inner = cursor.into_inner();
    assert(inner.size() == 3);
    assert(inner[0] == 1);

    std::cout << "  test_cursor_into_inner PASSED" << std::endl;
}

// ── Copy test ──────────────────────────────────────────

void test_io_copy() {
    std::vector<uint8_t> src_data = {10, 20, 30, 40, 50};
    auto reader = Cursor<std::vector<uint8_t>>::new_(src_data);

    std::vector<uint8_t> dst_data(10, 0);
    auto writer = Cursor<std::vector<uint8_t>>::new_(std::move(dst_data));

    auto result = copy(reader, writer);
    assert(result.is_ok());
    assert(result.unwrap() == 5);

    // Verify destination
    assert(writer.get_ref()[0] == 10);
    assert(writer.get_ref()[4] == 50);

    std::cout << "  test_io_copy PASSED" << std::endl;
}

// ── Seek negative position test ────────────────────────

void test_seek_negative_error() {
    std::vector<uint8_t> data(10, 0);
    auto cursor = Cursor<std::vector<uint8_t>>::new_(data);

    auto result = cursor.seek(SeekFrom::Current(-1));
    assert(result.is_err());
    assert(result.unwrap_err().kind() == Error::Kind::InvalidInput);

    std::cout << "  test_seek_negative_error PASSED" << std::endl;
}

void test_read_dispatch_for_integral_span() {
    std::array<int, 4> data = {255, 2, 3, 4};
    auto reader = std::span<const int>(data.data(), data.size());
    uint8_t out[3] = {0, 0, 0};

    auto result = read(reader, std::span<uint8_t>(out, 3));
    assert(result.is_ok());
    assert(result.unwrap() == 3);
    assert(out[0] == 255);
    assert(out[1] == 2);
    assert(out[2] == 3);
    assert(reader.size() == 1); // dynamic span advances like Rust &[u8] Read impl

    std::cout << "  test_read_dispatch_for_integral_span PASSED" << std::endl;
}

void test_write_dispatch_for_integral_span() {
    std::array<uint8_t, 4> storage = {0, 0, 0, 0};
    auto writer = std::span<uint8_t>(storage.data(), storage.size());
    const uint8_t input[] = {9, 8, 7};

    auto result = write(writer, std::span<const uint8_t>(input, 3));
    assert(result.is_ok());
    assert(result.unwrap() == 3);
    assert(storage[0] == 9);
    assert(storage[1] == 8);
    assert(storage[2] == 7);
    assert(writer.size() == 1); // dynamic span advances like Rust &mut [u8] Write impl

    std::cout << "  test_write_dispatch_for_integral_span PASSED" << std::endl;
}

void test_write_dispatch_rejects_read_only_span() {
    const std::array<uint8_t, 4> storage = {0, 0, 0, 0};
    auto writer = std::span<const uint8_t>(storage.data(), storage.size());
    const uint8_t input[] = {1, 2};

    auto result = write(writer, std::span<const uint8_t>(input, 2));
    assert(result.is_err());
    assert(result.unwrap_err().kind() == Error::Kind::Unsupported);

    std::cout << "  test_write_dispatch_rejects_read_only_span PASSED" << std::endl;
}

// ── Main ───────────────────────────────────────────────

int main() {
    std::cout << "Running rusty::io tests..." << std::endl;

    test_error_creation();
    test_result_ok();
    test_result_err();
    test_result_void();
    test_seekfrom();
    test_cursor_read();
    test_cursor_write();
    test_cursor_seek();
    test_cursor_read_write_seek_combined();
    test_cursor_position();
    test_cursor_into_inner();
    test_io_copy();
    test_seek_negative_error();
    test_read_dispatch_for_integral_span();
    test_write_dispatch_for_integral_span();
    test_write_dispatch_rejects_read_only_span();

    std::cout << "\nAll 16 rusty::io tests PASSED" << std::endl;
    return 0;
}
