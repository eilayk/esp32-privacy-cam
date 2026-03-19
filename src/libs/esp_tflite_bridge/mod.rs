use core::ffi::c_void;
use anyhow::Result;
extern "C" {
    fn tflite_create(model_data: *const u8, arena: *mut u8, arena_size: i32) -> *mut c_void;
    fn tflite_invoke(engine: *mut c_void) -> i32;
    fn tflite_get_input_ptr(engine: *mut c_void, index: i32) -> *mut c_void;
    fn tflite_get_output_ptr(engine: *mut c_void, index: i32) -> *mut c_void;
    fn tflite_get_input_bytes(engine: *mut c_void, index: i32) -> i32;
    fn tflite_get_output_bytes(engine: *mut c_void, index: i32) -> i32;
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
            anyhow::bail!("Failed to create TFLite engine (arena_size={} bytes)", arena_size);
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

    fn input_tensor_mut_impl<T>(&mut self, index: i32) -> Result<&mut [T]> {
        let ptr = unsafe { tflite_get_input_ptr(self.engine_ptr, index) } as *mut T;
        if ptr.is_null() {
            anyhow::bail!("Input tensor index {} not found", index);
        }

        let bytes = unsafe { tflite_get_input_bytes(self.engine_ptr, index) };
        if bytes < 0 {
            anyhow::bail!("Failed to read input tensor {} size", index);
        }

        let elem_size = core::mem::size_of::<T>();
        if elem_size == 0 {
            anyhow::bail!("Zero-sized tensor element type is not supported");
        }

        let bytes = bytes as usize;
        if bytes % elem_size != 0 {
            anyhow::bail!(
                "Input tensor {} size {} is not aligned for element size {}",
                index,
                bytes,
                elem_size
            );
        }

        let len = bytes / elem_size;
        Ok(unsafe { core::slice::from_raw_parts_mut(ptr, len) })
    }

    pub fn input_tensor_i8_mut(&mut self, index: i32) -> Result<&mut [i8]> {
        self.input_tensor_mut_impl::<i8>(index)
    }

    // get pointer to output tensor
    // immutable, read only
    fn output_tensor_impl<T>(&self, index: i32) -> Result<&[T]> {
        let ptr = unsafe { tflite_get_output_ptr(self.engine_ptr, index) } as *const T;
        if ptr.is_null() {
            anyhow::bail!("Output tensor index {} not found", index);
        }

        let bytes = unsafe { tflite_get_output_bytes(self.engine_ptr, index) };
        if bytes < 0 {
            anyhow::bail!("Failed to read output tensor {} size", index);
        }

        let elem_size = core::mem::size_of::<T>();
        if elem_size == 0 {
            anyhow::bail!("Zero-sized tensor element type is not supported");
        }

        let bytes = bytes as usize;
        if bytes % elem_size != 0 {
            anyhow::bail!(
                "Output tensor {} size {} is not aligned for element size {}",
                index,
                bytes,
                elem_size
            );
        }

        let len = bytes / elem_size;
        Ok(unsafe { core::slice::from_raw_parts(ptr, len) })
    }

    pub fn output_tensor_f32(&self, index: i32) -> Result<&[f32]> {
        self.output_tensor_impl::<f32>(index)
    }
}

impl Drop for TFLiteEngine {
    fn drop(&mut self) {
        unsafe { tflite_destroy(self.engine_ptr); }
    }
}