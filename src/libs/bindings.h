#if defined(ESP_IDF_COMP_ESPRESSIF__ESP32_CAMERA_ENABLED)
#include "esp_camera.h"
#endif

#if defined(ESP_IDF_COMP_ESPRESSIF__ESP_TFLITE_MICRO_ENABLED)
#include "tensorflow/lite/c/c_api_types.h"
#include "tensorflow/lite/c/common.h"
#include "tensorflow/lite/c/builtin_op_data.h"
#include "esp_nn.h"
#endif