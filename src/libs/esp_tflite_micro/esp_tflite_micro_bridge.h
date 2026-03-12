#ifndef TFLITE_BRIDGE_H
#define TFLITE_BRIDGE_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>

// Opaque pointer to the C++ Interpreter
typedef void* TFLiteEngine;

// Creates a TensorFlow Lite Micro interpreter for a model stored in flash.
//
// Parameters:
// - model_data: Pointer to a FlatBuffer model in memory.
// - arena: Preallocated tensor arena buffer used by TFLM for all runtime memory.
// - arena_size: Size of arena in bytes.
//
// Returns:
// - Non-null opaque engine handle on success.
// - nullptr if tensor allocation fails.
TFLiteEngine tflite_create(const uint8_t* model_data, uint8_t* arena, int arena_size);

// Runs one inference.
//
// Current implementation expects:
// - one float input value (written to input tensor index 0, element 0)
// - one float output value (read from output tensor index 0, element 0)
//
// Parameters:
// - engine: Opaque interpreter handle returned by tflite_create.
// - input: Pointer to the input scalar.
// - output: Pointer to where the output scalar will be written.
void tflite_run(TFLiteEngine engine, const float* input, float* output);

// Destroys an engine created by tflite_create. Safe to call with nullptr.
void tflite_destroy(TFLiteEngine engine);

#ifdef __cplusplus
}
#endif

#endif