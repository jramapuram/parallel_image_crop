use std::env;
use std::ffi::{CString};
use libc::{size_t, c_char, c_uchar, c_void, c_double, c_longlong};

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
    fn vips_image_write_to_memory(input: VImage, size: *mut size_t) -> *mut c_void;
    fn vips_image_get_width(image: VImage) -> i32;
    fn vips_image_get_height(image: VImage) -> i32;
    fn vips_resize(input: VImage, out: MutVImage, scale: c_double, ... ) -> i32;
    fn vips_crop(input: VImage, out: MutVImage,
                 left: i32, top: i32, width: i32, height: i32, ... ) -> i32;
    fn vips_init(argv0: *const c_char) -> i32;
    fn vips_shutdown();
}


fn scale_range(val: f32, newmin: f32, newmax: f32) -> f32 {
    // simple helper to scale a value range
    (((val) * (newmax - newmin)) / (1.0)) + newmin
}


#[no_mangle]
pub fn vips_crop_and_resize(path: &str, scale: f32, x_crop: f32, y_crop: f32,
                            max_img_percent: f32, resize_width: u32, resize_height: u32) -> Vec<u8>
{
    assert!(x_crop >= 0f32 && x_crop <= 1f32, "x of crop not bounded in [0, 1]");
    assert!(y_crop >= 0f32 && y_crop <= 1f32, "y of crop not bounded in [0, 1]");

    // get arg0 [i.e. this program's filename] and init vips
    let arg0: String = env::args().take(1).collect();
    let code = unsafe { vips_init(CString::new(arg0).unwrap().into_raw()) };
    assert!(code == 0, "VIPS init error");

    // load the image
    let path_cstr = CString::new(path).unwrap().into_raw();
    let img = unsafe { vips_image_new_from_file(path_cstr, VipsAccess::VIPS_ACCESS_SEQUENTIAL) };

    // grab the image dimensions
    let img_width = unsafe { vips_image_get_width(img) };
    let img_height = unsafe { vips_image_get_height(img) };
    let img_size = (img_width as u32, img_height as u32);

    // scale the x and y co-ordinates to the img_size
    let mut x = scale_range(x_crop, 0f32, img_size.0 as f32) as u32;
    let mut y = scale_range(y_crop, 0f32, img_size.1 as f32) as u32;

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

    // crop the image
    let mut out: i64 = 0;
    let img_crop_code = unsafe { vips_crop(img as VImage, &mut out as MutVImage,
                                           x as i32, y as i32,
                                           crop_size.0 as i32, crop_size.1 as i32) };
    assert!(img_crop_code == 0, "VIPS crop error");

    // resize it
    let img_resize_code = unsafe { vips_resize(out as VImage, &mut out as MutVImage,
                                               resize_width as f64 / img_size.0 as f64,
                                               resize_height as f64 / img_size.1 as f64) };
    assert!(img_resize_code == 0, "VIPS resize error");

    // write to block of memory
    let mut out_size: size_t = 0;
    // let scale_values = unsafe { slice::from_raw_parts(scale_ptr, length as usize) };
    unsafe {
        let mut output_buf = vips_image_write_to_memory(out as VImage, &mut out_size);
        vips_shutdown();
        Vec::from_raw_parts(output_buf as *mut u8, out_size, out_size)
    }

}
