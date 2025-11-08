# Rusty-CPP Documentation

This directory contains comprehensive documentation for the Rusty-CPP static analyzer.

## Directory Structure

### `/features/`
Complete implementation summaries for major features:

- **[template_support.md](features/template_support.md)** - Template function and class analysis
  - Template free functions and methods
  - Generic type analysis (no instantiation needed)
  - Multi-parameter templates (T, U, etc.)

- **[variadic_templates.md](features/variadic_templates.md)** - Variadic template support
  - Parameter pack recognition
  - Pack expansion detection
  - Variadic template classes
  - Pack ownership semantics
  - Template argument pack expansion

- **[unsafe_blocks.md](features/unsafe_blocks.md)** - @unsafe block implementation
  - Block-level safety escapes
  - Scope tracking with depth counter
  - Integration with borrow checker and safety analysis

- **[std_library_annotations.md](features/std_library_annotations.md)** - C++ STL function annotations
  - ~200+ whitelisted safe functions
  - Containers, algorithms, smart pointers, I/O
  - No explicit annotations needed in user code

- **[cast_operations.md](features/cast_operations.md)** - Cast operation safety
  - Why casts require @unsafe context
  - Design rationale for type casting safety

### Root Documentation Files

- **[annotation_reference.md](annotation_reference.md)** - Syntax reference for @safe, @unsafe, @lifetime annotations
- **[annotations.md](annotations.md)** - Detailed annotation system documentation
- **[method_qualifiers.md](method_qualifiers.md)** - C++ method qualifier handling (const, &&, etc.)
- **[control_flow_fix_summary.md](control_flow_fix_summary.md)** - Control flow analysis improvements
- **[fixing_control_flow.md](fixing_control_flow.md)** - Detailed control flow implementation
- **[submodule_integration.md](submodule_integration.md)** - Git submodule integration guide

### Lock-Free MPSC Channel Documentation

- **[mpsc_lockfree_user_guide.md](mpsc_lockfree_user_guide.md)** - ⭐ User guide for the lock-free MPSC channel
  - Quick start and API reference
  - Common patterns and examples
  - Performance guidelines
  - Best practices and troubleshooting

- **[mpsc_lockfree_developer_guide.md](mpsc_lockfree_developer_guide.md)** - Developer guide and implementation details
  - Architecture and design decisions
  - Memory ordering and concurrency
  - Performance characteristics
  - Testing strategy and benchmarks

## Quick Links

- Main project documentation: [../CLAUDE.md](../CLAUDE.md)
- User-facing README: [../README.md](../README.md)
- Source code: [../src/](../src/)
- Tests: [../tests/](../tests/)
- Examples: [../examples/](../examples/)

## Contributing

When adding new features, please:
1. Create a comprehensive implementation summary in `features/`
2. Update this README with a link to the new documentation
3. Update CLAUDE.md with the feature summary
4. Add tests to validate the feature
