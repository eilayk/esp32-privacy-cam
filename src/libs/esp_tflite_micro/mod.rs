use core::ffi::c_void;
use anyhow::Result;
extern "C" {
    fn tflite_create(model_data: *const u8, arena: *mut u8, arena_size: i32) -> *mut c_void;
fn tflite_invoke(engine: *mut c_void) -> i32;
    fn tflite_get_input_ptr(engine: *mut c_void, index: i32) -> *mut c_void;
    fn tflite_get_output_ptr(engine: *mut c_void, index: i32) -> *mut c_void;
    fn tflite_destroy(engine: *mut c_void);
}

pub struct TFLiteEngine {
    engine_ptr: *mut c_void,
    // keep the arena alive
    _arena: Vec<u8>, 
}

impl TFLiteEngine {
    pub fn new(model_data: &[u8], arena_size: usize) -> Result<Self> {
        let mut arena = vec![0u8; arena_size];
        let engine_ptr = unsafe { 
            tflite_create(model_data.as_ptr(), arena.as_mut_ptr(), arena_size as i32)
        };
        if engine_ptr.is_null() {
            anyhow::bail!("Failed to create TFLite engine");
        }
        Ok(Self { engine_ptr, _arena: arena })
    }

    pub fn invoke(&self) -> Result<()> {
        let status = unsafe { tflite_invoke(self.engine_ptr) };
        if status != 0 {
            anyhow::bail!("TFLite Invoke failed with status: {}", status);
        }
        Ok(())
    }

    // get pointer to input tensor
    // allows writing directly to the input buffer
    pub unsafe fn input_tensor_mut<T>(&mut self, index: i32) -> Result<&mut [T]> {
        let ptr = tflite_get_input_ptr(self.engine_ptr, index) as *mut T;
        if ptr.is_null() {
            anyhow::bail!("Input tensor index {} not found", index);
        }
        Ok(core::slice::from_raw_parts_mut(ptr, 128 * 128 * 3))
    }

    // get pointer to output tensor
    // immutable, read only
    pub unsafe fn output_tensor<T>(&self, index: i32, len: usize) -> Result<&[T]> {
        let ptr = tflite_get_output_ptr(self.engine_ptr, index) as *const T;
        if ptr.is_null() {
            anyhow::bail!("Output tensor index {} not found", index);
        }
        Ok(core::slice::from_raw_parts(ptr, len))
    }
}

impl Drop for TFLiteEngine {
    fn drop(&mut self) {
        unsafe { tflite_destroy(self.engine_ptr); }
    }
}