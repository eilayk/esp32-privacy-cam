#pragma once

#include <stddef.h>
#include <stdint.h>

#include "esp_err.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
	uint8_t *data;
	size_t data_len;
	uint16_t width;
	uint16_t height;
	uint32_t pix_type;
	size_t stride;
} esp_dl_image_t;

typedef struct {
	int32_t category;
	float score;
	int32_t left;
	int32_t top;
	int32_t right;
	int32_t bottom;
} esp_dl_detection_t;

typedef struct {
	esp_dl_detection_t *items;
	size_t len;
} esp_dl_detection_list_t;

typedef struct {
	uint8_t *data;
	size_t data_len;
} esp_dl_jpeg_t;

// Matches dl::image::pix_type_t::DL_IMAGE_PIX_TYPE_RGB888
#define ESP_DL_PIX_TYPE_RGB888 0u

// Decode JPEG data to RGB888 format. The caller is responsible for freeing the output image data using `esp_dl_image_free()`.
// Parameters:
// - `jpeg_data`: Pointer to the input JPEG data.
// - `jpeg_len`: Length of the input JPEG data in bytes.
// - `out_image`: Pointer to the output image structure that will be filled with the decoded
//   image data and metadata (width, height, pixel type, etc.).
// Returns:
// - `ESP_OK` on success.
// - `ESP_ERR_INVALID_ARG` if any of the input parameters are invalid (e.g., null pointers, zero length).
// - `ESP_FAIL` if the decoding process fails (e.g., due to invalid JPEG data).
esp_err_t esp_dl_decode_jpeg_rgb888(const uint8_t *jpeg_data, size_t jpeg_len, esp_dl_image_t *out_image);

// Free the memory allocated for the image data in the given `esp_dl_image_t` structure.
// This function should be called to release the memory when the image data is no longer needed.
// Parameters:
// - `image`: Pointer to the `esp_dl_image_t` structure whose image data should be freed. 
//    The structure itself will not be freed, only the memory allocated for the image data. 
//    After calling this function, the `data` pointer in the structure will be set to null
//    and other fields will be reset to zero.
void esp_dl_image_free(esp_dl_image_t *image);

// Creates a pedestrian detection model instance. Destroy it with `destroy_pedestrian_detection_model()`.
// Returns a pointer to the model instance, or null on failure.
void *create_pedestrian_detection_model(void);

// Destroys a pedestrian detection model instance created by `create_pedestrian_detection_model()`.
// Parameters:
// - `model`: Pointer to the model instance to destroy. If null, this function does nothing.
void destroy_pedestrian_detection_model(void *model);

// Runs pedestrian detection on the input image and writes detections to `out_result`.
// The caller must free `out_result` with `esp_dl_detection_list_free()`.
// Parameters:
// - `model`: Pointer to the pedestrian detection model instance.
// - `input_image`: Pointer to the input image structure containing the image data and metadata.
// - `out_result`: Pointer to the output detection list structure that will be filled with the detection results.
esp_err_t pedestrian_detection(void *model, const esp_dl_image_t *input_image, esp_dl_detection_list_t *out_result);

// Creates a human face detection model instance. Destroy it with `destroy_face_detection_model()`.
// Returns a pointer to the model instance, or null on failure.
void *create_face_detection_model(void);

// Destroys a human face detection model instance created by `create_face_detection_model()`.
// Parameters:
// - `model`: Pointer to the model instance to destroy. If null, this function does nothing.
void destroy_face_detection_model(void *model);

// Runs human face detection on the input image and writes detections to `out_result`.
// The caller must free `out_result` with `esp_dl_detection_list_free()`.
// Parameters:
// - `model`: Pointer to the human face detection model instance.
// - `input_image`: Pointer to the input image structure containing the image data and metadata.
// - `out_result`: Pointer to the output detection list structure that will be filled with the detection results.
esp_err_t face_detection(void *model, const esp_dl_image_t *input_image, esp_dl_detection_list_t *out_result);

// Draws hollow rectangles around detections on the given image.
// Parameters:
// - `image`: Pointer to the image structure to draw on.
// - `detections`: Pointer to the detection list structure containing the detections to draw.
void esp_dl_draw_detections(esp_dl_image_t *image, const esp_dl_detection_list_t *detections);

// Encodes the given image to JPEG format.
// The caller must free `out_jpeg` with `esp_dl_jpeg_free()`.
// Parameters:
// - `image`: Pointer to the image structure to encode.
// - `out_jpeg`: Pointer to the output JPEG structure that will be filled with the encoded JPEG data.
esp_err_t esp_dl_encode_jpeg(const esp_dl_image_t *image, esp_dl_jpeg_t *out_jpeg);

// Runs pedestrian detection on JPEG input, draws hollow rectangles around detections,
// and re-encodes the annotated frame to JPEG.
// The caller must free `out_result` with `esp_dl_detection_list_free()` and
// `out_jpeg` with `esp_dl_jpeg_free()`.
// Parameters:
// - `model`: Pointer to the pedestrian detection model instance.
// - `jpeg_data`: Pointer to input JPEG bytes.
// - `jpeg_len`: Length of input JPEG bytes.
// - `out_result`: Detection list output.
// - `out_jpeg`: Annotated JPEG output.
esp_err_t pedestrian_detection_annotate_jpeg(void *model,
											 const uint8_t *jpeg_data,
											 size_t jpeg_len,
											 esp_dl_detection_list_t *out_result,
											 esp_dl_jpeg_t *out_jpeg);

// Runs human face detection on JPEG input, draws hollow rectangles around detections,
// and re-encodes the annotated frame to JPEG.
// The caller must free `out_result` with `esp_dl_detection_list_free()` and
// `out_jpeg` with `esp_dl_jpeg_free()`.
// Parameters:
// - `model`: Pointer to the human face detection model instance.
// - `jpeg_data`: Pointer to input JPEG bytes.
// - `jpeg_len`: Length of input JPEG bytes.
// - `out_result`: Detection list output.
// - `out_jpeg`: Annotated JPEG output.
esp_err_t face_detection_annotate_jpeg(void *model,
										 const uint8_t *jpeg_data,
										 size_t jpeg_len,
										 esp_dl_detection_list_t *out_result,
										 esp_dl_jpeg_t *out_jpeg);

// Frees detection list memory allocated by `pedestrian_detection()`.
// Parameters:
// - `result`: Pointer to the `esp_dl_detection_list_t` structure whose memory should be freed.
void esp_dl_detection_list_free(esp_dl_detection_list_t *result);

// Frees JPEG memory allocated by `pedestrian_detection_annotate_jpeg()`.
// Parameters:
// - `jpeg`: Pointer to the `esp_dl_jpeg_t` structure whose memory should be freed.
void esp_dl_jpeg_free(esp_dl_jpeg_t *jpeg);

#ifdef __cplusplus
}
#endif
