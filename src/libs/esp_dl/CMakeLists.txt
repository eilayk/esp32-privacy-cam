idf_component_register(
    SRCS "esp_dl.cpp"
    INCLUDE_DIRS "."
    # PRIV-REQUIRES esp-dl pedestrian_detect
    REQUIRES esp-dl pedestrian_detect
)
