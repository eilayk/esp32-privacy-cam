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

static void blur_region_rgb888(esp_dl_image_t *image, int left, int top, int right, int bottom)
{
    constexpr int block_size = 12;

    const int width = static_cast<int>(image->width);
    const int height = static_cast<int>(image->height);
    const int channels = 3;

    const int x0 = std::clamp(left, 0, width - 1);
    const int y0 = std::clamp(top, 0, height - 1);
    const int x1 = std::clamp(right + 1, 0, width);
    const int y1 = std::clamp(bottom + 1, 0, height);

    if (x1 <= x0 || y1 <= y0)
    {
        return;
    }

    uint8_t *pixels = image->data;
    const size_t stride = image->stride;

    for (int by = y0; by < y1; by += block_size)
    {
        for (int bx = x0; bx < x1; bx += block_size)
        {
            const int block_end_y = std::min(by + block_size, y1);
            const int block_end_x = std::min(bx + block_size, x1);

            uint32_t sum_r = 0;
            uint32_t sum_g = 0;
            uint32_t sum_b = 0;
            uint32_t count = 0;

            for (int y = by; y < block_end_y; ++y)
            {
                uint8_t *row = pixels + static_cast<size_t>(y) * stride;
                for (int x = bx; x < block_end_x; ++x)
                {
                    uint8_t *px = row + static_cast<size_t>(x) * channels;
                    sum_r += px[0];
                    sum_g += px[1];
                    sum_b += px[2];
                    ++count;
                }
            }

            if (count == 0)
            {
                continue;
            }

            const uint8_t avg_r = static_cast<uint8_t>(sum_r / count);
            const uint8_t avg_g = static_cast<uint8_t>(sum_g / count);
            const uint8_t avg_b = static_cast<uint8_t>(sum_b / count);

            for (int y = by; y < block_end_y; ++y)
            {
                uint8_t *row = pixels + static_cast<size_t>(y) * stride;
                for (int x = bx; x < block_end_x; ++x)
                {
                    uint8_t *px = row + static_cast<size_t>(x) * channels;
                    px[0] = avg_r;
                    px[1] = avg_g;
                    px[2] = avg_b;
                }
            }
        }
    }
}

extern "C" void esp_dl_draw_detections(esp_dl_image_t *image, const esp_dl_detection_list_t *detections)
{
    if (image == nullptr || detections == nullptr || image->data == nullptr || detections->items == nullptr)
    {
        return;
    }

    dl::image::img_t img = {
        .data = image->data,
        .width = image->width,
        .height = image->height,
        .pix_type = static_cast<dl::image::pix_type_t>(image->pix_type),
    };

    const std::vector<uint8_t> box_color = {255, 0, 0};
    constexpr uint8_t line_width = 2;

    for (size_t i = 0; i < detections->len; ++i)
    {
        const auto &result = detections->items[i];
        int left = std::clamp(static_cast<int>(result.left), 0, static_cast<int>(image->width) - 1);
        int top = std::clamp(static_cast<int>(result.top), 0, static_cast<int>(image->height) - 1);
        int right = std::clamp(static_cast<int>(result.right), 0, static_cast<int>(image->width) - 1);
        int bottom = std::clamp(static_cast<int>(result.bottom), 0, static_cast<int>(image->height) - 1);

        if (right > left && bottom > top)
        {
            dl::image::draw_hollow_rectangle(img, left, top, right, bottom, box_color, line_width);
        }
    }
}

extern "C" void esp_dl_blur_detections(esp_dl_image_t *image, const esp_dl_detection_list_t *detections)
{
    if (image == nullptr || detections == nullptr || image->data == nullptr || detections->items == nullptr)
    {
        return;
    }

    if (image->pix_type != ESP_DL_PIX_TYPE_RGB888)
    {
        return;
    }

    for (size_t i = 0; i < detections->len; ++i)
    {
        const auto &result = detections->items[i];
        int left = std::clamp(static_cast<int>(result.left), 0, static_cast<int>(image->width) - 1);
        int top = std::clamp(static_cast<int>(result.top), 0, static_cast<int>(image->height) - 1);
        int right = std::clamp(static_cast<int>(result.right), 0, static_cast<int>(image->width) - 1);
        int bottom = std::clamp(static_cast<int>(result.bottom), 0, static_cast<int>(image->height) - 1);

        if (right > left && bottom > top)
        {
            blur_region_rgb888(image, left, top, right, bottom);
        }
    }
}

extern "C" esp_err_t esp_dl_encode_jpeg(const esp_dl_image_t *image, esp_dl_jpeg_t *out_jpeg)
{
    if (image == nullptr || out_jpeg == nullptr || image->data == nullptr)
    {
        return ESP_ERR_INVALID_ARG;
    }

    dl::image::img_t img = {
        .data = image->data,
        .width = image->width,
        .height = image->height,
        .pix_type = static_cast<dl::image::pix_type_t>(image->pix_type),
    };

    dl::image::jpeg_img_t encoded = dl::image::sw_encode_jpeg(img, 0, 80);
    if (encoded.data == nullptr)
    {
        return ESP_FAIL;
    }

    out_jpeg->data = static_cast<uint8_t *>(encoded.data);
    out_jpeg->data_len = encoded.data_len;
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

    esp_dl_image_t raw_image = {
        .data = static_cast<uint8_t *>(image.data),
        .data_len = dl::image::get_img_byte_size(image),
        .width = image.width,
        .height = image.height,
        .pix_type = static_cast<uint32_t>(image.pix_type),
        .stride = static_cast<size_t>(image.width) * 3,
    };

    esp_err_t err = pedestrian_detection(model, &raw_image, out_result);
    if (err == ESP_OK)
    {
        esp_dl_draw_detections(&raw_image, out_result);
        err = esp_dl_encode_jpeg(&raw_image, out_jpeg);
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

