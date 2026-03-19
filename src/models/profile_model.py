from mltk.core.tflite_micro import TfliteMicro

# pip install --upgrade silabs-mltk

profiling_results = TfliteMicro.profile_model("face_detection_front_128_integer_quant.tflite")

print(profiling_results)

