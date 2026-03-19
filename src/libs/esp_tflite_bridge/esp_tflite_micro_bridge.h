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

int tflite_invoke(void* engine);
void* tflite_get_input_ptr(void* engine, int index);
void* tflite_get_output_ptr(void* engine, int index);
int tflite_get_input_bytes(void* engine, int index);
int tflite_get_output_bytes(void* engine, int index);

// Destroys an engine created by tflite_create. Safe to call with nullptr.
void tflite_destroy(TFLiteEngine engine);

#ifdef __cplusplus
}
#endif

#endif