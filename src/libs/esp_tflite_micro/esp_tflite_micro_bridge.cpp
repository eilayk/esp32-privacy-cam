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
    static tflite::MicroMutableOpResolver<2> resolver;
    // resolver.AddFullyConnected();
    // resolver.AddSoftmax();
    // resolver.AddConv2D(); // Add only what you need!

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

extern "C" void tflite_run(TFLiteEngine engine, const float* input, float* output) {
    auto* interpreter = static_cast<tflite::MicroInterpreter*>(engine);
    
    // copy input data into the model's input tensor
    float* input_tensor = interpreter->input(0)->data.f;
    input_tensor[0] = *input; 

    interpreter->Invoke();

    // copy result to output pointer
    float* output_tensor = interpreter->output(0)->data.f;
    *output = output_tensor[0];
}

extern "C" void tflite_destroy(TFLiteEngine engine) {
    auto* interpreter = static_cast<tflite::MicroInterpreter*>(engine);
    delete interpreter;
}