use std::{slice, str, mem, ptr};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::path::Path;
use std::ffi::{CStr, OsStr, CString};
use std::marker::PhantomData;
use std::error::Error;
use std::os::raw::{c_void, c_int, c_char};
use vips_sys;
use vips_sys::{VipsSize, VipsKernel, VipsBandFormat, VipsCombineMode, VipsDirection};


// most of these are straight up copy-pasta from vips-rs
// the main changes here are to add VipsAccess to read_from_file


pub struct VipsImage<'a> {
    pub c: *mut vips_sys::VipsImage,
    marker: PhantomData<&'a()>,
}


impl<'a> Drop for VipsImage<'a> {
    fn drop(&mut self) {
        unsafe {
            vips_sys::g_object_unref(self.c as *mut c_void);
        }
    }
}


fn current_error() -> String {
    let msg = unsafe {
        CStr::from_ptr(vips_sys::vips_error_buffer())
    };
    msg.to_str().unwrap().to_string()
}


fn result<'a>(ptr: *mut vips_sys::VipsImage) -> Result<VipsImage<'a>, Box<Error>> {
    if ptr.is_null() {
        Err(current_error().into())
    } else {
        Ok(VipsImage { c: ptr, marker: PhantomData })
    }
}


fn result_with_ret<'a>(ptr: *mut vips_sys::VipsImage, ret: c_int) -> Result<VipsImage<'a>, Box<Error>> {
    if ret == 0 {
        Ok(VipsImage { c: ptr, marker: PhantomData })
    } else {
        Err(current_error().into())
    }
}


// callback used by gobjects
pub unsafe extern "C" fn image_postclose(ptr: *mut vips_sys::VipsImage, user_data: *mut c_void) {
    let b:Box<Box<[u8]>> = Box::from_raw(user_data as *mut Box<[u8]>);
    drop(b);
}


impl<'a> VipsImage<'a> {
    pub fn new() -> Result<VipsImage<'a>, Box<Error>> {
        let c = unsafe { vips_sys::vips_image_new() };
        result(c)
    }

    pub fn new_memory() -> Result<VipsImage<'a>, Box<Error>> {
        let c = unsafe { vips_sys::vips_image_new_memory() };
        result(c)
    }

    pub fn from_file<S: Into<Vec<u8>>>(path: S, access: vips_sys::VipsAccess) -> Result<VipsImage<'a>, Box<Error>> {
        let path = CString::new(path)?;
        let access_str = CString::new("access")?;
        let c = unsafe { vips_sys::vips_image_new_from_file(path.as_ptr(),
                                                            access_str.as_ptr(),
                                                            access,
                                                            ptr::null() as *const c_char) };
        result(c)
    }

    pub fn from_memory(buf: Vec<u8>, width: u32, height: u32,
                       bands: u8, format: VipsBandFormat) -> Result<VipsImage<'a>, Box<Error>> {
        let b:Box<[_]> = buf.into_boxed_slice();
        let c = unsafe {
            vips_sys::vips_image_new_from_memory(
                b.as_ptr() as *const c_void,
                b.len(),
                width as i32,
                height as i32,
                bands as i32,
                format,
            )
        };

        let bb:Box<Box<_>> = Box::new(b);
        let raw : *mut c_void = Box::into_raw(bb) as *mut c_void;

        unsafe {
            let callback: unsafe extern "C" fn() = ::std::mem::transmute(image_postclose as *const());
            vips_sys::g_signal_connect_data(
                c as *mut c_void, "postclose\0".as_ptr() as *const c_char,
                Some(callback),
                raw,
                None, vips_sys::GConnectFlags::G_CONNECT_AFTER);
        };

        result(c)
    }

    pub fn from_memory_reference(buf: &'a [u8], width: u32, height: u32,
                                 bands: u8, format: VipsBandFormat) -> Result<VipsImage, Box<Error>> {
        let c = unsafe {
            vips_sys::vips_image_new_from_memory(
                buf.as_ptr() as *const c_void,
                buf.len(),
                width as i32,
                height as i32,
                bands as i32,
                format,
            )
        };

        result(c)
    }

    // formatted
    pub fn from_buffer(buf: &'a [u8]) -> Result<VipsImage, Box<Error>> {
        let c = unsafe {
            vips_sys::vips_image_new_from_buffer(buf.as_ptr() as *const c_void, buf.len(),
                                                 ptr::null(), ptr::null() as *const c_char)
        };

        result(c)
    }

    // default: block shrink + lanczos3
    pub fn resize(&self, scale: f64, vscale: Option<f64>, kernel: Option<VipsKernel>) -> Result<VipsImage, Box<Error>> {
        let mut out_ptr: *mut vips_sys::VipsImage = ptr::null_mut();
        let ret = unsafe {
            vips_sys::vips_resize(self.c as *mut vips_sys::VipsImage,
                                  &mut out_ptr,
                                  scale,
                                  "vscale\0".as_ptr(),
                                  vscale.unwrap_or(scale),
                                  "kernel\0".as_ptr(),
                                  kernel.unwrap_or(VipsKernel::VIPS_KERNEL_LANCZOS3),
                                  ptr::null() as *const c_char,
            )
        };
        result_with_ret(out_ptr, ret)
    }

    pub fn resize_to_size(&self, width: u32, height: Option<u32>,
                      kernel: Option<VipsKernel>) -> Result<VipsImage, Box<Error>> {
        self.resize(
            width as f64 / self.width() as f64,
            height.map(|h| h as f64 / self.height() as f64),
            kernel,
        )
    }

    pub fn crop(&self, x: i32, y: i32, width: i32, height: i32) -> Result<VipsImage, Box<Error>> {
        let mut out_ptr: *mut vips_sys::VipsImage = ptr::null_mut();
        let ret = unsafe {
            vips_sys::vips_crop(self.c as *mut vips_sys::VipsImage,
                                &mut out_ptr,
                                x, y, width, height,
                                ptr::null() as *const c_char)
        };

        result_with_ret(out_ptr, ret)
    }

    pub fn width(&self) -> u32 {
        unsafe { (*self.c).Xsize as u32 }
    }

    pub fn height(&self) -> u32 {
        unsafe { (*self.c).Ysize as u32 }
    }

    pub fn to_vec(&self) -> Vec<u8> {
        unsafe {
            let mut result_size: usize = 0;
            let memory: *mut u8 = vips_sys::vips_image_write_to_memory(self.c as *mut vips_sys::VipsImage,
                                                                       &mut result_size as *mut usize) as *mut u8;
            let slice = ::std::slice::from_raw_parts_mut(memory, result_size);
            let boxed_slice: Box<[u8]> = Box::from_raw(slice);
            let vec = boxed_slice.into_vec();
            vec
        }
    }
}



lazy_static! {
    static ref IS_INSTANCIATED: AtomicBool = AtomicBool::new(false);
}

pub struct VipsInstance { }

impl VipsInstance {
    pub fn new(name:&str, leak_test:bool) -> Result<VipsInstance, Box<Error>> {
        // can return value: prev value
        if IS_INSTANCIATED.compare_and_swap(false, true, Relaxed) {
            Err("You cannot create VipsInstance more than once.".into())
        } else {
            let c = CString::new(name)?;
            unsafe {
                vips_sys::vips_init(c.as_ptr());
                if leak_test {
                    vips_sys::vips_leak_set(leak_test as c_int);
                }

                // set the max cache
                vips_sys::vips_cache_set_max(0);
            }
            Ok(VipsInstance {})
        }
    }
}

impl Drop for VipsInstance {
    fn drop(&mut self) {
        unsafe {
            vips_sys::vips_shutdown();
        }
    }
}
