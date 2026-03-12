use core::ffi::c_void;
// import the generated bindings
extern "C" {
    fn tflite_create(model_data: *const u8, arena: *mut u8, arena_size: i32) -> *mut c_void;
    fn tflite_run(engine: *mut c_void, input : *const f32, output: *mut f32);
}

pub struct TFLiteEngine {
    ptr: *mut c_void,
    // keep the arena alive
    _arena: Vec<u8>, 
}

impl TFLiteEngine {
    pub fn new(model_data: &[u8], arena_size: usize) -> Self {
        let mut arena = vec![0u8; arena_size];
        let ptr = unsafe { 
            tflite_create(model_data.as_ptr(), arena.as_mut_ptr(), arena_size as i32) 
        };
        Self { ptr, _arena: arena }
    }

    pub fn predict(&self, input: f32) -> f32 {
        let mut output = 0.0f32;
        unsafe { tflite_run(self.ptr, &input, &mut output) };
        output
    }
}