#include "esp_dl.h"

#include <algorithm>
#include <string.h>
#include <vector>

#include "dl_image_draw.hpp"
#include "dl_image_define.hpp"
#include "dl_image_jpeg.hpp"
#include "esp_heap_caps.h"
#include "esp_err.h"
#include "pedestrian_detect.hpp"

extern "C" esp_err_t esp_dl_decode_jpeg_rgb888(const uint8_t *jpeg_data, size_t jpeg_len, esp_dl_image_t *out_image)
{
    if (jpeg_data == nullptr || jpeg_len == 0 || out_image == nullptr)
    {
        return ESP_ERR_INVALID_ARG;
    }

    dl::image::jpeg_img_t input = {
        .data = const_cast<uint8_t *>(jpeg_data),
        .data_len = jpeg_len,
    };

    dl::image::img_t decoded =
        dl::image::sw_decode_jpeg(input, dl::image::DL_IMAGE_PIX_TYPE_RGB888, 0);
    if (decoded.data == nullptr)
    {
        return ESP_FAIL;
    }

    out_image->data = static_cast<uint8_t *>(decoded.data);
    out_image->data_len = dl::image::get_img_byte_size(decoded);
    out_image->width = decoded.width;
    out_image->height = decoded.height;
    out_image->pix_type = static_cast<uint32_t>(decoded.pix_type);
    out_image->stride = static_cast<size_t>(decoded.width) * 3;

    return ESP_OK;
}

extern "C" void esp_dl_image_free(esp_dl_image_t *image)
{
    if (image == nullptr)
    {
        return;
    }

    if (image->data != nullptr)
    {
        heap_caps_free(image->data);
    }

    memset(image, 0, sizeof(*image));
}

extern "C" void* create_pedestrian_detection_model()
{
    PedestrianDetect *detect = new PedestrianDetect();
    return detect;
}

extern "C" void destroy_pedestrian_detection_model(void *model)
{
    if (model != nullptr)
    {
        PedestrianDetect *detect = static_cast<PedestrianDetect *>(model);
        delete detect;
    }
}

extern "C" esp_err_t pedestrian_detection(void *model, const esp_dl_image_t *input_image, esp_dl_detection_list_t *out_result)
{
    if (model == nullptr || input_image == nullptr || out_result == nullptr || input_image->data == nullptr)
    {
        return ESP_ERR_INVALID_ARG;
    }

    out_result->items = nullptr;
    out_result->len = 0;

    if (input_image->pix_type != ESP_DL_PIX_TYPE_RGB888)
    {
        return ESP_ERR_INVALID_ARG;
    }

    dl::image::img_t image = {
        .data = input_image->data,
        .width = input_image->width,
        .height = input_image->height,
        .pix_type = static_cast<dl::image::pix_type_t>(input_image->pix_type),
    };

    PedestrianDetect *detect = static_cast<PedestrianDetect *>(model);
    auto &detect_results = detect->run(image);

    const size_t count = detect_results.size();
    if (count == 0)
    {
        return ESP_OK;
    }

    auto *items = static_cast<esp_dl_detection_t *>(heap_caps_calloc(count, sizeof(esp_dl_detection_t), MALLOC_CAP_DEFAULT));
    if (items == nullptr)
    {
        return ESP_ERR_NO_MEM;
    }

    size_t i = 0;
    for (const auto &result : detect_results)
    {
        auto &dst = items[i++];
        dst.category = result.category;
        dst.score = result.score;
        if (result.box.size() >= 4)
        {
            dst.left = result.box[0];
            dst.top = result.box[1];
            dst.right = result.box[2];
            dst.bottom = result.box[3];
        }
    }

    out_result->items = items;
    out_result->len = count;
    return ESP_OK;
}

extern "C" void esp_dl_detection_list_free(esp_dl_detection_list_t *result)
{
    if (result == nullptr)
    {
        return;
    }

    if (result->items != nullptr)
    {
        heap_caps_free(result->items);
    }

    memset(result, 0, sizeof(*result));
}

extern "C" esp_err_t pedestrian_detection_annotate_jpeg(void *model,
                                                         const uint8_t *jpeg_data,
                                                         size_t jpeg_len,
                                                         esp_dl_detection_list_t *out_result,
                                                         esp_dl_jpeg_t *out_jpeg)
{
    if (model == nullptr || jpeg_data == nullptr || jpeg_len == 0 || out_result == nullptr || out_jpeg == nullptr)
    {
        return ESP_ERR_INVALID_ARG;
    }

    out_result->items = nullptr;
    out_result->len = 0;
    out_jpeg->data = nullptr;
    out_jpeg->data_len = 0;

    dl::image::jpeg_img_t input = {
        .data = const_cast<uint8_t *>(jpeg_data),
        .data_len = jpeg_len,
    };

    dl::image::img_t image = dl::image::sw_decode_jpeg(input, dl::image::DL_IMAGE_PIX_TYPE_RGB888, 0);
    if (image.data == nullptr)
    {
        return ESP_FAIL;
    }

    esp_err_t err = ESP_OK;

    PedestrianDetect *detect = static_cast<PedestrianDetect *>(model);
    auto &detect_results = detect->run(image);

    const size_t count = detect_results.size();
    if (count > 0)
    {
        auto *items = static_cast<esp_dl_detection_t *>(heap_caps_calloc(count, sizeof(esp_dl_detection_t), MALLOC_CAP_DEFAULT));
        if (items == nullptr)
        {
            heap_caps_free(image.data);
            return ESP_ERR_NO_MEM;
        }

        const std::vector<uint8_t> box_color = {255, 0, 0};
        constexpr uint8_t line_width = 2;

        size_t i = 0;
        for (const auto &result : detect_results)
        {
            auto &dst = items[i++];
            dst.category = result.category;
            dst.score = result.score;
            if (result.box.size() >= 4)
            {
                int left = std::clamp(result.box[0], 0, static_cast<int>(image.width) - 1);
                int top = std::clamp(result.box[1], 0, static_cast<int>(image.height) - 1);
                int right = std::clamp(result.box[2], 0, static_cast<int>(image.width) - 1);
                int bottom = std::clamp(result.box[3], 0, static_cast<int>(image.height) - 1);

                dst.left = left;
                dst.top = top;
                dst.right = right;
                dst.bottom = bottom;

                if (right > left && bottom > top)
                {
                    dl::image::draw_hollow_rectangle(image, left, top, right, bottom, box_color, line_width);
                }
            }
        }

        out_result->items = items;
        out_result->len = count;
    }

    dl::image::jpeg_img_t encoded = dl::image::sw_encode_jpeg(image, 0, 80);
    if (encoded.data == nullptr)
    {
        err = ESP_FAIL;
    }
    else
    {
        out_jpeg->data = static_cast<uint8_t *>(encoded.data);
        out_jpeg->data_len = encoded.data_len;
    }

    heap_caps_free(image.data);
    return err;
}

extern "C" void esp_dl_jpeg_free(esp_dl_jpeg_t *jpeg)
{
    if (jpeg == nullptr)
    {
        return;
    }

    if (jpeg->data != nullptr)
    {
        heap_caps_free(jpeg->data);
    }

    memset(jpeg, 0, sizeof(*jpeg));
}

