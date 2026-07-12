// Runtime validation of the io-cursor std slice: io::cursor::Cursor over
// std::span<uint8_t> (Rust Cursor<&mut [u8]>) — Read/Seek/BufRead/Write
// surface, error paths included. Instantiates the template bodies that
// --precompile skipped.
import rusty;
#include <cassert>
#include <cstdint>
#include <cstdio>
#include <span>

int main() {
    uint8_t data[5] = {1, 2, 3, 4, 5};
    auto c = io::cursor::Cursor<std::span<uint8_t>>::new_(std::span<uint8_t>(data, 5));
    assert(c.position() == 0);

    // Read: partial read advances pos
    uint8_t rb[2] = {0, 0};
    auto r = c.read(std::span<uint8_t>(rb, 2));
    assert(r.is_ok() && r.unwrap() == 2 && rb[0] == 1 && rb[1] == 2);
    assert(c.position() == 2);

    // read_exact: consumes the remaining 3
    uint8_t rb3[3] = {0, 0, 0};
    auto re = c.read_exact(std::span<uint8_t>(rb3, 3));
    assert(re.is_ok() && rb3[0] == 3 && rb3[2] == 5 && c.position() == 5);

    // read at EOF -> Ok(0)
    auto r0 = c.read(std::span<uint8_t>(rb, 2));
    assert(r0.is_ok() && r0.unwrap() == 0);

    // read_exact past EOF -> Err
    auto rerr = c.read_exact(std::span<uint8_t>(rb, 2));
    assert(rerr.is_err());

    // Seek: Start / Current / End / overflow error
    auto s1 = c.seek(io::SeekFrom::Start(1));
    assert(s1.is_ok() && s1.unwrap() == 1 && c.position() == 1);
    auto s2 = c.seek(io::SeekFrom::Current(2));
    assert(s2.is_ok() && s2.unwrap() == 3);
    auto s3 = c.seek(io::SeekFrom::End(-1));
    assert(s3.is_ok() && s3.unwrap() == 4);
    auto s4 = c.seek(io::SeekFrom::Current(-10));
    assert(s4.is_err());
    assert(c.position() == 4);  // failed seek must not move pos

    assert(c.stream_len().unwrap() == 5);
    assert(c.stream_position().unwrap() == 4);

    // BufRead: fill_buf window + consume
    c.set_position(2);
    auto fb = c.fill_buf();
    assert(fb.is_ok());
    auto win = fb.unwrap();
    assert(win.size() == 3 && win[0] == 3 && win[2] == 5);
    c.consume(2);
    assert(c.position() == 4);

    // split at pos
    c.set_position(2);
    auto [lo, hi] = c.split();
    assert(lo.size() == 2 && hi.size() == 3 && lo[0] == 1 && hi[0] == 3);

    // Write (Cursor<&mut [u8]>): overwrite from pos, no growth
    c.set_position(0);
    uint8_t wsrc[3] = {9, 8, 7};
    auto w = c.write_(std::span<const uint8_t>(wsrc, 3));
    assert(w.is_ok() && w.unwrap() == 3);
    assert(data[0] == 9 && data[1] == 8 && data[2] == 7 && data[3] == 4);
    assert(c.position() == 3);

    // write_all past the end -> Err (WriteZero), flush -> Ok
    uint8_t big[4] = {1, 1, 1, 1};
    auto wa = c.write_all(std::span<const uint8_t>(big, 4));
    assert(wa.is_err());
    assert(c.flush().is_ok());

    std::printf("io-cursor runtime OK: Read/Seek/BufRead/Write + error paths\n");
    return 0;
}
