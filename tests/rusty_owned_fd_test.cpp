// Tests for rusty::os::fd::OwnedFd and BorrowedFd.
//
// Uses pipe(2) to create real kernel file descriptors so the
// destructor's ::close() is observable (subsequent ::write() to the
// closed fd returns -1 with EBADF).

#include "../include/rusty/os/fd.hpp"

#include <cassert>
#include <cerrno>
#include <cstdio>
#include <cstring>
#include <fcntl.h>
#include <unistd.h>
#include <utility>

using rusty::os::fd::OwnedFd;
using rusty::os::fd::BorrowedFd;
using rusty::os::fd::as_raw_fd;

// Helper: try to close a raw fd and detect "already closed" via EBADF.
// Returns true if the fd was still open (we just closed it).
static bool fd_still_open(int fd) {
    return ::fcntl(fd, F_GETFD) != -1 || errno != EBADF;
}

void test_default_construction() {
    printf("test_default_construction: ");
    OwnedFd fd;
    assert(!fd.is_valid());
    assert(fd.as_raw_fd() == -1);
    printf("PASS\n");
}

void test_from_raw_fd_and_destructor_closes() {
    printf("test_from_raw_fd_and_destructor_closes: ");
    int pipefd[2];
    int rc = ::pipe(pipefd);
    assert(rc == 0);

    // Take ownership of the read end.
    int read_fd = pipefd[0];
    {
        OwnedFd owned = OwnedFd::from_raw_fd(read_fd);
        assert(owned.is_valid());
        assert(owned.as_raw_fd() == read_fd);
        assert(fd_still_open(read_fd));
    }
    // OwnedFd's destructor must have closed the fd.
    assert(!fd_still_open(read_fd));

    // Clean up the write end.
    ::close(pipefd[1]);
    printf("PASS\n");
}

void test_into_raw_fd_releases_without_closing() {
    printf("test_into_raw_fd_releases_without_closing: ");
    int pipefd[2];
    assert(::pipe(pipefd) == 0);

    int released_fd = -1;
    {
        OwnedFd owned = OwnedFd::from_raw_fd(pipefd[0]);
        released_fd = owned.into_raw_fd();
        // After into_raw_fd, owned is moved-from.
        assert(!owned.is_valid());
        assert(owned.as_raw_fd() == -1);
    }
    // Destructor must NOT have closed the released fd.
    assert(fd_still_open(released_fd));

    // Clean up.
    ::close(released_fd);
    ::close(pipefd[1]);
    printf("PASS\n");
}

void test_move_ctor_transfers_ownership() {
    printf("test_move_ctor_transfers_ownership: ");
    int pipefd[2];
    assert(::pipe(pipefd) == 0);

    OwnedFd source = OwnedFd::from_raw_fd(pipefd[0]);
    int raw = source.as_raw_fd();
    {
        OwnedFd moved(std::move(source));
        assert(moved.is_valid());
        assert(moved.as_raw_fd() == raw);
        assert(!source.is_valid());
        assert(source.as_raw_fd() == -1);
        assert(fd_still_open(raw));
    }
    // After scope exit, the moved-into OwnedFd's destructor closed the fd.
    assert(!fd_still_open(raw));

    ::close(pipefd[1]);
    printf("PASS\n");
}

void test_move_assign_closes_prior() {
    printf("test_move_assign_closes_prior: ");
    int pipefd_a[2];
    int pipefd_b[2];
    assert(::pipe(pipefd_a) == 0);
    assert(::pipe(pipefd_b) == 0);

    OwnedFd a = OwnedFd::from_raw_fd(pipefd_a[0]);
    OwnedFd b = OwnedFd::from_raw_fd(pipefd_b[0]);
    int a_raw = a.as_raw_fd();
    int b_raw = b.as_raw_fd();
    assert(fd_still_open(a_raw));
    assert(fd_still_open(b_raw));

    // Move-assign — should close a_raw and transfer b_raw to a.
    a = std::move(b);
    assert(!fd_still_open(a_raw));   // prior fd closed
    assert(a.as_raw_fd() == b_raw);  // new fd owned
    assert(!b.is_valid());           // source moved-from

    // Scope exit closes b_raw via a's destructor.
    ::close(pipefd_a[1]);
    ::close(pipefd_b[1]);
    printf("PASS\n");
}

void test_try_clone_creates_distinct_fd() {
    printf("test_try_clone_creates_distinct_fd: ");
    int pipefd[2];
    assert(::pipe(pipefd) == 0);
    OwnedFd a = OwnedFd::from_raw_fd(pipefd[0]);

    auto clone_result = a.try_clone();
    assert(clone_result.is_ok());
    OwnedFd b = clone_result.unwrap();

    // Both must be valid, with distinct raw fds.
    assert(a.is_valid());
    assert(b.is_valid());
    assert(a.as_raw_fd() != b.as_raw_fd());

    // Both must remain open after the clone.
    assert(fd_still_open(a.as_raw_fd()));
    assert(fd_still_open(b.as_raw_fd()));

    ::close(pipefd[1]);
    printf("PASS\n");
}

void test_try_clone_invalid_fd_is_err() {
    printf("test_try_clone_invalid_fd_is_err: ");
    OwnedFd empty;
    auto result = empty.try_clone();
    assert(result.is_err());
    auto err = result.unwrap_err();
    assert(err.kind() == rusty::io::Error::Kind::InvalidInput);
    printf("PASS\n");
}

void test_borrowed_fd_does_not_close() {
    printf("test_borrowed_fd_does_not_close: ");
    int pipefd[2];
    assert(::pipe(pipefd) == 0);
    int raw = pipefd[0];
    {
        BorrowedFd borrow = BorrowedFd::borrow_raw(raw);
        assert(borrow.as_raw_fd() == raw);
        assert(as_raw_fd(borrow) == raw);
    }
    // BorrowedFd is non-owning; the fd must still be open.
    assert(fd_still_open(raw));

    ::close(raw);
    ::close(pipefd[1]);
    printf("PASS\n");
}

void test_as_fd_returns_non_owning_view() {
    printf("test_as_fd_returns_non_owning_view: ");
    int pipefd[2];
    assert(::pipe(pipefd) == 0);
    OwnedFd owned = OwnedFd::from_raw_fd(pipefd[0]);
    BorrowedFd view = owned.as_fd();
    assert(view.as_raw_fd() == owned.as_raw_fd());

    // After view goes out of scope, owned still owns the fd.
    {
        BorrowedFd inner = owned.as_fd();
        (void)inner;
    }
    assert(fd_still_open(owned.as_raw_fd()));

    ::close(pipefd[1]);
    printf("PASS\n");
}

void test_negative_fd_treated_as_invalid() {
    printf("test_negative_fd_treated_as_invalid: ");
    OwnedFd neg = OwnedFd::from_raw_fd(-1);
    assert(!neg.is_valid());
    // Destructor must be a no-op (no `close(-1)` libc call).
    printf("PASS\n");
}

int main() {
    printf("=== Testing rusty::os::fd::OwnedFd ===\n");

    test_default_construction();
    test_from_raw_fd_and_destructor_closes();
    test_into_raw_fd_releases_without_closing();
    test_move_ctor_transfers_ownership();
    test_move_assign_closes_prior();
    test_try_clone_creates_distinct_fd();
    test_try_clone_invalid_fd_is_err();
    test_borrowed_fd_does_not_close();
    test_as_fd_returns_non_owning_view();
    test_negative_fd_treated_as_invalid();

    printf("\nAll OwnedFd tests passed!\n");
    return 0;
}
