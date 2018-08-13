extern crate libc;
extern crate image;
extern crate rayon;
//extern crate time;
 #[macro_use] extern crate itertools;


use std::ptr;
use std::fs::File;
use std::path::Path;
use std::ffi::{CStr, OsStr, CString};
use std::{slice, str, mem};
use rayon::prelude::*;
use libc::{size_t, c_char, c_uchar, c_void, c_double, c_longlong};


mod lazy_load;
mod vips_ffi;
mod piston;


struct CropManager{
    threadpool: rayon::ThreadPool,
    num_threads: usize,
    use_vips: bool
}

// impl Drop for CropManager{
//     fn drop(&mut self) {
//         std::mem:drop(self.threadpool);
//         println!("calling crop drop");
//     }
// }


pub fn scale_range(val: f32, newmin: f32, newmax: f32) -> f32 {
    // simple helper to scale a value range
    (((val) * (newmax - newmin)) / (1.0)) + newmin
}


#[no_mangle]
pub extern "C" fn initialize(num_threads: u64, use_vips: bool) -> *mut c_void
{
    if use_vips {
        vips_ffi::initialize_vips();
    }

    // build the manager that handles the threadpool
    let mut cm = Box::new(Box::new(CropManager {
        threadpool: rayon::ThreadPoolBuilder::new().num_threads(num_threads as usize).build().unwrap(),
        num_threads: num_threads as usize,
        use_vips: use_vips
    }));

    // return just a ptr, but forget to memory release it
    Box::into_raw(cm) as *mut Box<CropManager> as *mut c_void
}


#[no_mangle]
pub extern "C" fn destroy(crop_manager_ptr: *mut c_void)
{
    // drop the threadpool
    let cm: Box<Box<CropManager>> = unsafe { Box::from_raw(crop_manager_ptr as *mut Box<CropManager>) };
    let use_vips = cm.use_vips;
    std::mem::drop(cm);

    // destroy vips after thread-pool
    if use_vips {
        vips_ffi::destroy_vips();
    }
}

pub struct Job {
    image_paths_ptr: *const *const c_char,
    return_ptr: *mut u8,
    scale_ptr: *const f32,
    x_ptr: *const f32,
    y_ptr: *const f32,
    window_size: u32,
    chans: u32,
    max_img_percent: f32,
    length: size_t
}

unsafe impl Send for Job {}
unsafe impl Sync for Job {}

#[no_mangle]
pub extern "C" fn parallel_crop_and_resize(crop_manager_ptr: *const c_void,
                                           image_paths_ptr: *const *const c_char,
                                           return_ptr: *mut u8,
                                           scale_ptr: *const f32,
                                           x_ptr: *const f32,
                                           y_ptr: *const f32,
                                           window_size: u32,
                                           chans: u32,
                                           max_img_percent: f32,
                                           length: size_t)
{
    // unpack the crop manager
    let cm: Box<Box<CropManager>> = unsafe { Box::from_raw(crop_manager_ptr as *mut Box<CropManager>) };

    // build the job
    let job = Job {
        image_paths_ptr: image_paths_ptr,
        return_ptr: return_ptr,
        scale_ptr: scale_ptr,
        x_ptr: x_ptr,
        y_ptr: y_ptr,
        window_size: window_size,
        chans: chans,
        max_img_percent: max_img_percent,
        length: length
    };

    // post to correct impl
    cm.threadpool.install(|| {
        match cm.use_vips {
            true   => vips_ffi::execute_job(&job),
            false => piston::execute_job(&job)
        }
    });

    // prevent the release of the crop-manager
    mem::forget(cm);

    // unpack the c-structures
    // let scale_values = unsafe { slice::from_raw_parts(scale_ptr, length as usize) };
    // let x_values = unsafe { slice::from_raw_parts(x_ptr, length as usize) };
    // let y_values = unsafe { slice::from_raw_parts(y_ptr, length as usize) };
    // let ret_values = unsafe { slice::from_raw_parts(return_ptr, length as usize) };
    // let paths_values = unsafe { slice::from_raw_parts(image_paths_ptr, length as usize) };


    // // iterate over everything and install into the pool
    // let mut idx = 0; // (0..length*win_size).step_by(win_size)
    // let win_size = (window_size * window_size * chans) as usize;
    // for (path, s, x, y) in  izip!(paths_values, scale_values, x_values, y_values)
    // {
    //     let path_str = str::from_utf8(unsafe { CStr::from_ptr(path) }.to_bytes()).unwrap();


    //     let match cm.use_vips {
    //         True  => cm.threadpool.install(|| );
    //         False => cm.threadpool.install(|| );
    //     }

    //     idx += 1;

    // }
}
