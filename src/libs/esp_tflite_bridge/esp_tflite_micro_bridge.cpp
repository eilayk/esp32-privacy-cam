#include "esp_log.h"
#include "esp_tflite_micro_bridge.h"
#include "tensorflow/lite/micro/micro_interpreter.h"
#include "tensorflow/lite/micro/micro_mutable_op_resolver.h"
#include "tensorflow/lite/schema/schema_generated.h"

namespace {
constexpr const char* kTag = "tflite_bridge";
}

extern "C" TFLiteEngine tflite_create(const uint8_t* model_data, uint8_t* arena, int arena_size) {
    // validate input parameters
    if (model_data == nullptr || arena == nullptr || arena_size <= 0) {
        ESP_LOGE(kTag, "Invalid args: model_data=%p arena=%p arena_size=%d", model_data, arena, arena_size);
        return nullptr;
    }

    // get model from flatbuffer
    auto model = tflite::GetModel(model_data);
    if (model == nullptr) {
        ESP_LOGE(kTag, "Failed to map model FlatBuffer");
        return nullptr;
    }

    // check model schema version
    if (model->version() != TFLITE_SCHEMA_VERSION) {
        ESP_LOGE(
            kTag,
            "Model schema version mismatch: model=%d runtime=%d",
            model->version(),
            TFLITE_SCHEMA_VERSION);
        return nullptr;
    }

    // create op resolver
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

    // create interpreter
    auto* interpreter = new tflite::MicroInterpreter(
        model, resolver, arena, arena_size);
    if (interpreter == nullptr) {
        ESP_LOGE(kTag, "Failed to allocate MicroInterpreter");
        return nullptr;
    }

    // allocate tensors
    if (interpreter->AllocateTensors() != kTfLiteOk) {
        ESP_LOGE(kTag, "AllocateTensors failed. Arena may be too small or op resolver incomplete");
        delete interpreter;
        return nullptr;
    }

    // validate input shape
    auto* input_tensor = interpreter->input(0);
    if (input_tensor == nullptr) {
        ESP_LOGE(kTag, "Failed to get input tensor");
        delete interpreter;
        return nullptr;
    }
    

    // return the interpreter as an opaque pointer
    return static_cast<void*>(interpreter);
}

extern "C" int tflite_invoke(void* engine) {
    auto* interpreter = static_cast<tflite::MicroInterpreter*>(engine);
    if (interpreter == nullptr) {
        return kTfLiteError;
    }
    return interpreter->Invoke();
}

extern "C" void* tflite_get_input_ptr(void* engine, int index) {
    auto* interpreter = static_cast<tflite::MicroInterpreter*>(engine);
    if (interpreter == nullptr) {
        return nullptr;
    }
    auto* tensor = interpreter->input(index);
    if (tensor == nullptr) {
        return nullptr;
    }
    return tensor->data.raw;
}

extern "C" void* tflite_get_output_ptr(void* engine, int index) {
    auto* interpreter = static_cast<tflite::MicroInterpreter*>(engine);
    if (interpreter == nullptr) {
        return nullptr;
    }
    auto* tensor = interpreter->output(index);
    if (tensor == nullptr) {
        return nullptr;
    }
    return tensor->data.raw;
}

extern "C" int tflite_get_input_bytes(void* engine, int index) {
    auto* interpreter = static_cast<tflite::MicroInterpreter*>(engine);
    if (interpreter == nullptr) {
        return -1;
    }
    auto* tensor = interpreter->input(index);
    if (tensor == nullptr) {
        return -1;
    }
    return tensor->bytes;
}

extern "C" int tflite_get_output_bytes(void* engine, int index) {
    auto* interpreter = static_cast<tflite::MicroInterpreter*>(engine);
    if (interpreter == nullptr) {
        return -1;
    }
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