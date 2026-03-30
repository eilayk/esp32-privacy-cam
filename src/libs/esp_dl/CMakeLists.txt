idf_component_register(
    SRCS "esp_dl.cpp"
    INCLUDE_DIRS "."
    REQUIRES esp-dl pedestrian_detect human_face_detect
)
