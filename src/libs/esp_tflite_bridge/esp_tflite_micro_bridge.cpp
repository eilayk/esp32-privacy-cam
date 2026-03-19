#include "esp_tflite_micro_bridge.h"
#include "tensorflow/lite/micro/micro_interpreter.h"
#include "tensorflow/lite/micro/micro_mutable_op_resolver.h"
#include "tensorflow/lite/micro/micro_log.h"

extern "C" TFLiteEngine tflite_create(const uint8_t* model_data, uint8_t* arena, int arena_size) {
    // map the model into a usable data structure
    // does not involve any copying or parsing
    // lightweight operation
    auto model = tflite::GetModel(model_data);
    
    // add the needed operators
    static tflite::MicroMutableOpResolver<14> resolver;
    resolver.AddConv2D();
    resolver.AddDepthwiseConv2D();
    resolver.AddReshape();
    resolver.AddAdd();
    resolver.AddMul();
    resolver.AddConcatenation();
    resolver.AddQuantize();
    resolver.AddDequantize();
    resolver.AddPad();
    resolver.AddLogistic();
    resolver.AddRelu();
    resolver.AddMaxPool2D();
    resolver.AddLeakyRelu();
    resolver.AddSlice();

    // create the interpreter
    auto* interpreter = new tflite::MicroInterpreter(
        model, resolver, arena, arena_size);

    // allocate memory from the tensor arena
    if (interpreter->AllocateTensors() != kTfLiteOk) {
        delete interpreter;
        return nullptr;
    }

    // return the interpreter as an opaque pointer
    return (void*)interpreter;
}

extern "C" int tflite_invoke(void* engine) {
    auto* interpreter = static_cast<tflite::MicroInterpreter*>(engine);
    return interpreter->Invoke();
}

extern "C" void* tflite_get_input_ptr(void* engine, int index) {
    auto* interpreter = static_cast<tflite::MicroInterpreter*>(engine);
    return interpreter->input(index)->data.raw;
}

extern "C" void* tflite_get_output_ptr(void* engine, int index) {
    auto* interpreter = static_cast<tflite::MicroInterpreter*>(engine);
    return interpreter->output(index)->data.raw;
}

extern "C" int tflite_get_input_bytes(void* engine, int index) {
    auto* interpreter = static_cast<tflite::MicroInterpreter*>(engine);
    auto* tensor = interpreter->input(index);
    if (tensor == nullptr) {
        return -1;
    }
    return tensor->bytes;
}

extern "C" int tflite_get_output_bytes(void* engine, int index) {
    auto* interpreter = static_cast<tflite::MicroInterpreter*>(engine);
    auto* tensor = interpreter->output(index);
    if (tensor == nullptr) {
        return -1;
    }
    return tensor->bytes;
}

extern "C" void tflite_destroy(TFLiteEngine engine) {
    auto* interpreter = static_cast<tflite::MicroInterpreter*>(engine);
    delete interpreter;
}