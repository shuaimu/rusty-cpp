# btreemap

Minimal end-to-end fixture for the `std::collections::BTreeMap` `insert`/`get`
write-path.

The goal is the path the team-todo TODO-001 milestone calls out: the C++
emitted by `rusty-cpp-transpiler` for these three functions must pass
`rusty-cpp-checker` with no `@unsafe` escapes when the functions are
annotated `// @safe`.

See `../run_btreemap_check.sh` for the driver that performs:

1. Transpile this crate to a `.cppm` module.
2. Inject `// @safe` annotations on the three exported fixture functions.
3. Run `rusty-cpp-checker` and require a clean checker exit.
