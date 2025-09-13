# External Annotations for Third-Party Code

RustyCpp provides a powerful system for annotating third-party functions and libraries with safety information without modifying their source code. This allows gradual adoption of safety checking even when using legacy code or external dependencies.

## Quick Start

To use external annotations in your project:

```cpp
#include <external_annotations.hpp>  // RustyCpp external annotations
#include <third_party_library.h>      // Your third-party headers

// @safe
void my_function() {
    // Third-party functions are checked according to annotations
    safe_third_party_func();     // OK if marked safe
    unsafe_third_party_func();   // ERROR: requires @unsafe context
}
```

## Annotation Syntax

### Function-Level Safety Annotations

Mark specific functions as safe or unsafe:

```cpp
// @external_safety: {
//   third_party::process: safe
//   third_party::allocate: unsafe
//   legacy::old_api: unsafe
//   vendor::trusted_func: safe
// }
```

### Pattern-Based Annotations

Use wildcards to annotate groups of functions:

```cpp
// @external_whitelist: {
//   patterns: [
//     "mylib::*",           // All functions in mylib namespace
//     "*::size",            // All size() methods
//     "*::length",          // All length() methods
//     "safe_*",             // Functions starting with safe_
//   ]
// }

// @external_blacklist: {
//   patterns: [
//     "*::internal_*",      // Internal functions
//     "*::unsafe_*",        // Explicitly unsafe functions
//     "*::operator new*",   // Memory allocation
//     "*::operator delete*", // Memory deallocation
//   ]
// }
```

### Library Profiles

Define reusable profiles for common libraries:

```cpp
// @external_profile: qt {
//   safe: [
//     "Q*::*",              // Most Qt classes
//     "qt::*",              // Qt namespace
//   ]
//   unsafe: [
//     "*::connect",         // Signal/slot connections
//     "*::eventFilter",     // Event handling
//   ]
// }

// Activate a profile in your code
// RustyCpp will use 'qt' profile for this file
```

## Built-in Annotations

### C Standard Library

Common C functions are pre-annotated:

**Safe functions:**
- `printf`, `fprintf`, `snprintf` - Formatted output
- `strcmp`, `strncmp`, `strlen` - String operations
- `atoi`, `atof`, `strtol` - String conversions
- `exit`, `abort` - Program termination

**Unsafe functions:**
- `malloc`, `calloc`, `realloc`, `free` - Memory management
- `memcpy`, `memmove`, `memset` - Raw memory operations
- `strcpy`, `strcat` - Buffer overflow risks
- `gets`, `sprintf` - Unsafe I/O

### POSIX System Calls

**Safe functions:**
- `mkdir`, `rmdir`, `unlink`, `rename` - File operations
- `stat`, `fstat`, `lstat` - File information
- `getpid`, `getuid`, `getgid` - Process information

**Unsafe functions:**
- `open`, `close`, `read`, `write` - File descriptors
- `fork`, `exec*` - Process management
- `socket`, `bind`, `connect` - Networking
- `pthread_*` - Threading operations
- `mmap`, `munmap` - Memory mapping

### Common Libraries

**Boost:**
```cpp
// Safe: algorithm, format, filesystem paths
// Unsafe: asio, thread, interprocess
```

**JSON Libraries:**
```cpp
// Safe: nlohmann::json parse/dump operations
// Unsafe: Raw buffer manipulations
```

**Database Libraries:**
```cpp
// Safe: Prepared statements, bind operations
// Unsafe: Direct SQL execution, resource management
```

## Usage Examples

### Basic External Annotations

```cpp
// Define annotations for a third-party library
// @external_safety: {
//   libfoo::initialize: safe
//   libfoo::process: safe
//   libfoo::cleanup: unsafe  // Manual resource management
// }

#include <libfoo.h>

// @safe
void use_library() {
    libfoo::initialize();  // OK - marked safe
    libfoo::process();     // OK - marked safe
    // libfoo::cleanup();  // ERROR: unsafe function in safe context
}

// @unsafe
void cleanup() {
    libfoo::cleanup();     // OK in unsafe context
}
```

### Pattern Matching

```cpp
// @external_whitelist: {
//   patterns: ["helper::*", "util::*"]
// }

// @safe
void test() {
    helper::any_function();  // OK - matches helper::*
    util::another();        // OK - matches util::*
    unknown::function();    // ERROR: not whitelisted
}
```

### Using Profiles

```cpp
// @external_profile: embedded {
//   unsafe: ["*"]  // Everything unsafe by default
//   safe: [
//     "*::read_register",
//     "*::write_register",
//     "hal::gpio::*"
//   ]
// }

// Embedded code with selective safe functions
// @safe
void embedded_code() {
    hal::gpio::set_pin(5);     // OK - marked safe
    read_register(0x1000);      // OK - marked safe
    // raw_memory_access(addr);  // ERROR: not marked safe
}
```

### Mixing with STL Annotations

```cpp
#include <external_annotations.hpp>
#include <stl_lifetimes.hpp>
#include <vector>
#include <third_party.h>

// @safe
void combined() {
    // STL is handled by stl_lifetimes.hpp
    std::vector<int> vec = {1, 2, 3};
    int& ref = vec[0];  // Lifetime tracked
    
    // Third-party handled by external_annotations.hpp
    third_party::safe_function();   // Checked against annotations
}
```

## Best Practices

### 1. Start Conservative

Mark most third-party functions as unsafe initially:

```cpp
// @external_safety: {
//   third_party::*: unsafe  // Start with everything unsafe
// }
```

Then selectively mark proven-safe functions:

```cpp
// @external_safety: {
//   third_party::get_version: safe
//   third_party::get_name: safe
//   // Rest remains unsafe
// }
```

### 2. Use Profiles for Large Libraries

Create profiles for commonly used libraries:

```cpp
// @external_profile: opencv {
//   safe: [
//     "cv::imread",
//     "cv::imwrite",
//     "cv::resize",
//     "cv::Mat::*"
//   ]
//   unsafe: [
//     "cv::*alloc*",
//     "cv::*release*"
//   ]
// }
```

### 3. Document Assumptions

Add comments explaining why functions are marked safe/unsafe:

```cpp
// @external_safety: {
//   // Safe: Pure computation, no side effects
//   math_lib::calculate: safe
//   
//   // Unsafe: Allocates memory that must be freed
//   math_lib::create_matrix: unsafe
//   
//   // Unsafe: Global state modification
//   math_lib::set_precision: unsafe
// }
```

### 4. Regular Audits

Periodically review external annotations:

- Remove annotations for libraries no longer used
- Update annotations when library APIs change
- Refine safety classifications based on experience

## Integration with Build Systems

### CMake Integration

```cmake
# Add RustyCpp external annotations to include path
target_include_directories(myproject PRIVATE
    ${RUSTYCPP_INCLUDE_DIR}
)

# Define which external annotation profile to use
target_compile_definitions(myproject PRIVATE
    RUSTYCPP_PROFILE=myproject
)
```

### Compiler Flags

```bash
# Include RustyCpp headers
g++ -I/path/to/rustycpp/include \
    -DRUSTYCPP_PROFILE=embedded \
    myfile.cpp
```

## Limitations

1. **No Deep Analysis**: External annotations don't analyze the actual implementation
2. **Trust-Based**: Incorrectly marking unsafe functions as safe can hide bugs
3. **Maintenance**: Annotations must be kept in sync with library versions
4. **Granularity**: Can't annotate specific overloads differently

## Troubleshooting

### Function Not Recognized

If a function isn't being checked properly:

1. Check the exact name (including namespace)
2. Verify pattern matching rules
3. Check profile is activated
4. Look for conflicting annotations

### Too Many False Positives

If safe functions are marked unsafe:

1. Add explicit safe annotations
2. Use whitelist patterns
3. Create a project-specific profile
4. Consider wrapping in safe interfaces

### Performance Impact

External annotations have minimal runtime impact:

- Checked at compile-time only
- No runtime overhead
- Pattern matching cached

## Future Improvements

- Automatic annotation generation from library docs
- Machine learning-based safety inference
- Integration with package managers
- Shared annotation databases
- Overload-specific annotations