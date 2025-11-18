#include <iostream>
#include <cassert>
#include "../include/rusty/rusty.hpp"

using namespace rusty;

struct Animal {
    std::string name;
    virtual ~Animal() = default;
    virtual void speak() const = 0;
};

struct Dog : public Animal {
    int age;

    Dog(std::string n, int a) : age(a) {
        name = std::move(n);
        std::cout << "Dog(" << name << ") created\n";
    }

    ~Dog() override {
        std::cout << "Dog(" << name << ") destroyed\n";
    }

    void speak() const override {
        std::cout << name << " says: Woof! (age " << age << ")\n";
    }
};

// Simulate passing Arc across FFI boundary
void test_ffi_round_trip() {
    std::cout << "\n=== Test FFI Round Trip with into_raw_parts/from_raw_parts ===\n\n";

    // Create Arc
    Arc<Dog> dog = Arc<Dog>::make("Buddy", 5);
    std::cout << "Created Arc<Dog>, strong_count=" << dog.strong_count() << "\n";

    // Convert to raw parts (like passing to C code)
    auto parts = std::move(dog).into_raw_parts();
    std::cout << "Converted to raw parts, dog is now invalid\n";
    std::cout << "Dog still alive (not destroyed yet)\n";

    // Simulate some C code using the raw pointer
    parts.ptr->speak();

    // Reconstruct Arc from raw parts (like returning from C code)
    Arc<Dog> dog2 = Arc<Dog>::from_raw_parts(parts.ptr, parts.control);
    std::cout << "Reconstructed Arc<Dog>, strong_count=" << dog2.strong_count() << "\n";

    // Use the reconstructed Arc
    dog2->speak();

    std::cout << "\n=== Test Complete ===\n";
    // Dog should be destroyed here when dog2 goes out of scope
}

// Test adopt() for taking ownership of existing raw pointer
void test_adopt() {
    std::cout << "\n=== Test Arc::adopt() ===\n\n";

    // Simulate legacy code that returns raw pointer
    Dog* raw_dog = new Dog("Charlie", 3);
    std::cout << "Created raw Dog*\n";

    // Adopt the raw pointer into Arc
    Arc<Dog> dog = Arc<Dog>::adopt(raw_dog);
    std::cout << "Adopted into Arc<Dog>, strong_count=" << dog.strong_count() << "\n";

    dog->speak();

    std::cout << "\n=== Test Complete ===\n";
    // Dog will be properly destroyed when Arc goes out of scope
}

// Test polymorphic FFI
void test_polymorphic_ffi() {
    std::cout << "\n=== Test Polymorphic FFI ===\n\n";

    // Create Arc<Dog>
    Arc<Dog> dog = Arc<Dog>::make("Max", 7);
    std::cout << "Created Arc<Dog>\n";

    // Convert to Arc<Animal> (polymorphic)
    Arc<Animal> animal = dog;
    std::cout << "Converted to Arc<Animal>, strong_count=" << animal.strong_count() << "\n";

    // Export polymorphic Arc to FFI
    auto parts = std::move(animal).into_raw_parts();
    std::cout << "Exported Arc<Animal> to raw parts\n";

    // Use through base pointer
    parts.ptr->speak();

    // Import back as base type
    Arc<Animal> animal2 = Arc<Animal>::from_raw_parts(parts.ptr, parts.control);
    std::cout << "Imported back as Arc<Animal>, strong_count=" << animal2.strong_count() << "\n";

    animal2->speak();

    std::cout << "\n=== Test Complete ===\n";
}

int main() {
    test_ffi_round_trip();
    test_adopt();
    test_polymorphic_ffi();

    std::cout << "\n=== All Tests Passed ===\n";
    return 0;
}
