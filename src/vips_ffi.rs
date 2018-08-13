use std::{slice, str, mem, env, ptr};
use rayon::prelude::*;
use std::path::Path;
use std::ffi::{CStr, OsStr, CString};
use libc::{size_t, c_char, c_uchar, c_void, c_double, c_longlong};

//use time::PreciseTime;

type MutVImage = *mut c_longlong;
type VImage    = c_longlong;

#[repr(u32)]
enum VipsAccess {
    VIPS_ACCESS_RANDOM = 0,
    VIPS_ACCESS_SEQUENTIAL = 1,
    VIPS_ACCESS_SEQUENTIAL_UNBUFFERED = 2,
    VIPS_ACCESS_LAST = 3
}

#[link(name = "vips")]
extern "C" {
    fn vips_image_new_from_file(name: *const c_char, ... ) -> VImage;
    // fn vips_image_new_from_file(name: *const c_char, access: *const c_char, ...) -> VImage;
    fn vips_image_write_to_memory(input: VImage, size: *mut size_t) -> *mut c_void;
    fn vips_image_get_width(image: VImage) -> i32;
    fn vips_image_get_height(image: VImage) -> i32;
    fn vips_resize(input: VImage, out: MutVImage, scale: c_double,
                   keyword: *const c_char, vscale: c_double, ... ) -> i32;
    fn vips_crop(input: VImage, out: MutVImage, left: i32, top: i32, width: i32, height: i32, ... ) -> i32;
    fn vips_init(argv0: *const c_char) -> i32;
    fn vips_get_argv0() -> *const c_char;
    fn vips_shutdown();
}


fn dimensions(img: &VImage) -> (u32, u32)
{
    let img_width = unsafe { vips_image_get_width(*img) };
    let img_height = unsafe { vips_image_get_height(*img) };
    (img_width as u32, img_height as u32)
}

pub fn initialize_vips()
{
    // let arg0: String = env::args().take(1).collect();
    // println!("arg0 = {:?}", arg0);
    // let code = unsafe { vips_init(CString::new(arg0).unwrap().into_raw()) };
    // let code = unsafe { vips_init(null_ptr as *const i8) };
    let arg0 = unsafe { vips_get_argv0() };
    let code = unsafe { vips_init(arg0) };
    assert!(code == 0, "VIPS init error");
}

pub fn destroy_vips()
{
    unsafe { vips_shutdown() };
}


pub fn vips_crop_and_resize(path: &str, scale: f32, x_crop: f32, y_crop: f32,
                            max_img_percent: f32, resize_width: u32, resize_height: u32) -> Vec<u8>
{
    assert!(x_crop >= 0f32 && x_crop <= 1f32, "x of crop not bounded in [0, 1]");
    assert!(y_crop >= 0f32 && y_crop <= 1f32, "y of crop not bounded in [0, 1]");
    let null_ptr: *const i32 = ptr::null();

    // get arg0 [i.e. this program's filename] and init vips
    // can't init here: causes issues destruction
    // initialize_vips();



    // load the image and grab the image dimensions
    let path_cstr = CString::new(path).unwrap().into_raw();
    let access_cstr = CString::new("access").unwrap().into_raw();
    let img = unsafe { vips_image_new_from_file(path_cstr , access_cstr,
                                                VipsAccess::VIPS_ACCESS_SEQUENTIAL,
                                                // VipsAccess::VIPS_ACCESS_RANDOM,
                                                null_ptr) };
    let img_size = dimensions(&img);

    // scale the x and y co-ordinates to the img_size
    let mut x = super::scale_range(x_crop, 0f32, img_size.0 as f32) as u32;
    let mut y = super::scale_range(y_crop, 0f32, img_size.1 as f32) as u32;

    // calculate the scale of the true crop using the provided scale
    // NOTE: this is different from the return size, i.e. window_size
    let crop_scale = scale.min(max_img_percent);
    let crop_size = ((img_size.0 as f32 * crop_scale).floor().max(2.0) as u32,
                     (img_size.1 as f32 * crop_scale).floor().max(2.0) as u32);
    let max_coords = (img_size.0 - crop_size.0,
                      img_size.1 - crop_size.1);

    // threshold the max x and y
    x = x.min(max_coords.0);
    y = y.min(max_coords.1);
    // println!("x= {} | y= {} | w = {} | h = {} | rw = {} | rh = {} " , x as i32, y as i32,
    //          crop_size.0 as i32, crop_size.1 as i32, resize_width, resize_height);

    // crop the image
    let mut out: i64 = 0;
    let img_crop_code = unsafe { vips_crop(img as VImage, &mut out as MutVImage,
                                           x as i32, y as i32,
                                           crop_size.0 as i32,
                                           crop_size.1 as i32, null_ptr) };
    let crop_img_size = dimensions(&out);
    assert!(img_crop_code == 0, "VIPS crop error");

    // resize it
    let keyword_cstr = CString::new("vscale").unwrap().into_raw();
    let img_resize_code = unsafe { vips_resize(out as VImage, &mut out as MutVImage,
                                               resize_width as f64 / crop_img_size.0 as f64,
                                               keyword_cstr,
                                               resize_height as f64 / crop_img_size.1 as f64,
                                               null_ptr)
    };
    assert!(img_resize_code == 0, "VIPS resize error");

    // write to block of memory
    let mut out_size: size_t = 0;
    //let start = PreciseTime::now();
    let mut output_buf = unsafe { vips_image_write_to_memory(out as VImage , &mut out_size) };
    // let end = PreciseTime::now();
    // println!("{} seconds for write[internal].", start.to(end));

    // destroy_vips(); //XXX: causes issues if destroyed here

    let v = unsafe { Vec::from_raw_parts(output_buf as *mut u8, out_size, out_size) };

    // cleanup mem
    // mem::drop(img);
    // mem::drop(output_buf);

    // return the vec
    v
}


pub fn execute_job(job: &super::Job){
    parallel_crop_and_resize(job.image_paths_ptr,
                             job.return_ptr,
                             job.scale_ptr,
                             job.x_ptr,
                             job.y_ptr,
                             job.window_size,
                             job.chans,
                             job.max_img_percent,
                             job.length)
}

pub fn parallel_crop_and_resize(image_paths_ptr: *const *const c_char,
                                return_ptr: *mut u8,
                                scale_ptr: *const f32,
                                x_ptr: *const f32,
                                y_ptr: *const f32,
                                window_size: u32,
                                chans: u32,
                                max_img_percent: f32,
                                length: size_t)
{
    // accepts list of image-paths (str), a vector (np) of z's
    // and the size of the arrays (i.e. batch dim)
    // and returns crops of all the images
    assert!(!scale_ptr.is_null(), "can't operate over null scale vector");
    assert!(!x_ptr.is_null(), "can't operate over null x vector");
    assert!(!y_ptr.is_null(), "can't operate over null y vector");
    assert!(!return_ptr.is_null(), "can't operate over null result vector");
    assert!(!image_paths_ptr.is_null(), "can't operate over null list of image paths");

    // gather the paths into a vector
    let image_paths_vec: Vec<&str> = unsafe { slice::from_raw_parts(image_paths_ptr, length as usize) }.iter()
        .map(|&p| unsafe { CStr::from_ptr(p) })  // iterator of &CStr
        .map(|cs| cs.to_bytes())                 // iterator of &[u8]
        .map(|bs| str::from_utf8(bs).unwrap())   // iterator of &str
        .collect();

    // gather the z into arrays of [scale, x, y]
    let scale_values = unsafe { slice::from_raw_parts(scale_ptr, length as usize) };
    let x_values = unsafe { slice::from_raw_parts(x_ptr, length as usize) };
    let y_values = unsafe { slice::from_raw_parts(y_ptr, length as usize) };

    // working!
    let mut resultant_vec = vec![];
    // let start = PreciseTime::now();
    // image_paths_vec.into_par_iter().zip(scale_values)
    //     .zip(x_values.par_iter()).zip(y_values)
    image_paths_vec.into_par_iter().zip(scale_values)
        .zip(x_values).zip(y_values)
        .map(|(((path, scale), x), y)| {
            vips_crop_and_resize(path,
                                 *scale, *x , *y,
                                 max_img_percent,
                                 window_size,
                                 window_size)
        }).collect_into_vec(&mut resultant_vec);

    // let end = PreciseTime::now();
    // println!("{} seconds for write[external].", start.to(end));

    // copy the buffer into the return array
    let win_size = (window_size * window_size * chans) as usize;
    for (begin, rvec) in izip!((0..length*win_size).step_by(win_size), resultant_vec)
    {
        assert!(rvec.len() == win_size, "rvec [{:?}] != window_size [{:?}]",
                rvec.len(), win_size);
        unsafe { ptr::copy(rvec.as_ptr() as *const u8, return_ptr.offset(begin as isize),
                           win_size) };
    }
}
