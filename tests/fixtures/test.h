/* A minimal header file for testing the SID C FFI */
#ifndef TEST_H
#define TEST_H

// Returns the absolute value of x
double fabs(double x);

// Square root
double sqrt(double x);

// Floor
double floor(double x);

// String length (returns a size_t, but we test Int mapping)
// Note: we use int here to keep the return type simple in our bridge
int strlen(const char *s);

// Puts (writes a string + newline to stdout, returns int)
int puts(const char *s);

// Struct declarations should be skipped
struct Foo { int x; };

// Typedef should be skipped  
typedef int MyInt;

// Variadic functions are bridged; extra arg types are inferred at call time
int printf(const char *format, ...);

#endif
