# RustyCpp TODO
<!--
This comment block is the prompt content in case you forget.

Work on tasks defined in TODO.md. Repeat the following steps, don’t stop until interrupted. Don’t ask me for advice, just pick the best option you think that is honest, complete, and not corner-cutting: 

1. Pick the top undone task with highest priority (high-medium-low), choose its first leaf task.  If there are no undone TODO items left, sleep a minute and git pull and restart step 1 (so this step is a dead loop until you find a todo item).
2. Analyze the task, check if this can be done with not too many LOC (i.e., smaller than 500 lines code give or take). If not, try to analyze this task and break it down into several smaller tasks, expanding it in the TODO.md. The breakdown can be nested and hierarchical. Try to make each leaf task small enough (<500 lines LOC). You can document your analysis in the doc folder for future reference. 
3. Try to execute the first leaf task. Make a plan for the task before execute, put the plan in the docs folder, and add the file name in the item in TODO.md for reference. You can all write your key findings as a few sentences in the TODO item. 
4. Make sure to add comprehensive test for the task executed. Run the whole test suites to make sure no regression happens. If tests fail, fix them using the best, honest, complete approach, run test suites again to verify fixes work. Repeat this step until no tests fail. 
5. Prepare for git commit, remove all temporary files, especially not to commit any binary files. For plan files, extract from implementation plan the design rational and user manual and put it in the docs folder.
6. Git commit the changes. First do git pull --rebase, and fix conflicts if any. Then do git push.
7. Go back to step 1. (The TODO.md file is possibly updated, so make sure you read the updated TODO.)

-->

- [ ] Bring Rust's memory safety guarantees to C++ through static analysis and safe type wrappers
  - [ ] Static borrow checking - analyze C++ code to detect use-after-free, dangling references, and double-free at compile time
    - [x] *done* Detect returning a struct whose reference member points to a local variable that will be destroyed
    - [x] *done* Track which parameter's lifetime flows to return value when function has multiple reference parameters with different lifetimes
    - [x] *done* Detect iterator use after container modification (e.g., using iterator after push_back which may reallocate)
    - [x] *done* Detect reference use after container modification (e.g., holding ref to vec[0] then calling push_back)
    - [x] *done* Detect use of reference obtained from unique_ptr after calling reset() or release()
    - [x] *done* Detect returning ptr.get() from a function where the unique_ptr is a local variable
    - [x] *done* Fix use-after-move detection for STL types like std::string in non-template code (now detects use-after-move when passing moved variables to function calls)
    - [ ] *low* Track field-level borrows through method calls using MoveField/UseField/BorrowField IR statements
      - [x] Phase 1+2 partial: Generate UseField for method calls on fields - When `field.method()` is called, generate UseField to check for borrow conflicts
      - [ ] Phase 3: Track return value borrows from field - When method returns reference, track that return value borrows from the field (requires type info for method return types)
  - [ ] Rust std library equivalents - C++ types in rusty:: namespace that mirror Rust's safe APIs
    - [x] *done* rusty::Box<T> - heap-allocated single-owner pointer, like unique_ptr but with Rust semantics
    - [x] *done* rusty::Arc<T> - atomic reference-counted pointer for thread-safe shared ownership
    - [x] *done* rusty::Rc<T> - reference-counted pointer for single-threaded shared ownership
    - [x] *done* rusty::Cell<T> - interior mutability for Copy types, allows mutation through const reference
    - [x] *done* rusty::RefCell<T> - interior mutability with runtime borrow checking, panics on violation
    - [x] *done* rusty::Option<T> - explicit optional value, forces handling of None case
    - [x] *done* rusty::Vec<T> - growable array with bounds checking and safe iterator invalidation
    - [x] *done* rusty::SafeFn / rusty::UnsafeFn - type-safe function pointer wrappers distinguishing safe vs unsafe callables
    - [x] *done* rusty::Result<T, E> - error handling type that forces explicit error handling, no exceptions
    - [x] *done* rusty::String - owned UTF-8 string with safe mutation and no null terminator assumptions
    - [x] *done* rusty::HashMap<K, V> - hash map with safe iteration and no iterator invalidation on lookup
    - [x] *done* rusty::HashSet<T> - hash set with safe iteration and no iterator invalidation on lookup
