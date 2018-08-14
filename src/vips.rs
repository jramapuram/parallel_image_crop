use std::{slice, str, mem, ptr};
use rayon::prelude::*;
use std::path::Path;
use std::ffi::{CStr, OsStr, CString};
use std::marker::PhantomData;
use std::error::Error;
use libc::{size_t, c_char, c_uchar, c_void, c_double, c_longlong};
use vips_ffi::{VipsInstance, VipsImage};
use vips_sys::VipsAccess;


pub fn vips_crop_and_resize(path: &str, scale: f32, x_crop: f32, y_crop: f32,
                            max_img_percent: f32, resize_width: u32, resize_height: u32) -> Vec<u8>
{
    assert!(x_crop >= 0f32 && x_crop <= 1f32, "x of crop not bounded in [0, 1]");
    assert!(y_crop >= 0f32 && y_crop <= 1f32, "y of crop not bounded in [0, 1]");
    let null_ptr: *const i32 = ptr::null();

    // load the image and grab the image dimensions
    let mut img = VipsImage::from_file(path, VipsAccess::VIPS_ACCESS_SEQUENTIAL).unwrap();
    let img_size = (img.width(), img.height());

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

    // truncate the image, resize it and return a Vec<u8>
    let crop = img.crop(x as i32, y as i32, crop_size.0 as i32, crop_size.1 as i32).unwrap();
    let resized = crop.resize_to_size(resize_width, Some(resize_height), None).unwrap();
    resized.to_vec()
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
    // let start = PreciseTime::now();
    let mut resultant_vec = vec![];
    image_paths_vec.into_par_iter().zip(scale_values)
        .zip(x_values).zip(y_values)
        .map(|(((path, scale), x), y)| {
            vips_crop_and_resize(path,
                                 *scale, *x , *y,
                                 max_img_percent,
                                 window_size,
                                 window_size)
        }).collect_into_vec(&mut resultant_vec);

    // single threaded
    // let resultant_vec: Vec<Vec<u8>> = image_paths_vec.iter().zip(scale_values)
    //     .zip(x_values).zip(y_values)
    //     .map(|(((path, scale), x), y)| {
    //         vips_crop_and_resize(path,
    //                              *scale, *x , *y,
    //                              max_img_percent,
    //                              window_size,
    //                              window_size)
    //     }).collect();


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



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vips_bw_image_crop() {
        // test the center crop and ensure that the crop is also RGB
        // initialize_vips();
        let center = vips_crop_and_resize("assets/lena_gray.png",
                                          0.25, 0.5, 0.5, 0.25, 32, 32);
        assert!(center.len() == 32*32);
        // destroy_vips();
    }

    // #[test]
    // fn test_vips_image_crops() {
    //     initialize_vips();

    //     // test the top left crop
    //     let top_left = vips_crop_and_resize("assets/lena.png",
    //                                         0.25, 0.25, 0.25, 0.25, 32, 32);
    //     assert!(top_left.len() == 32*32*3, "top left was {:?}", top_left.len());

    //     // // test the top right crop
    //     // let top_right = vips_crop_and_resize("assets/lena.png",
    //     //                                      0.25, 0.75, 0.25, 0.25, 32, 32);
    //     // assert!(top_right.len() == 32*32*3, "top right was {:?}", top_right.len());

    //     // // test the bottom left crop
    //     // let bottom_left = vips_crop_and_resize("assets/lena.png",
    //     //                                        0.25, 0.25, 0.75, 0.25, 32, 32);
    //     // assert!(bottom_left.len() == 32*32*3, "bottom left was {:?}", bottom_left.len());

    //     // // test the bottom right crop
    //     // let bottom_right = vips_crop_and_resize("assets/lena.png",
    //     //                                         0.25, 0.75, 0.75, 0.25, 32, 32);
    //     // assert!(bottom_right.len() == 32*32*3, "bottom right was {:?}", bottom_right.len());

    //     // // test the center crop
    //     // let center = vips_crop_and_resize("assets/lena.png",
    //     //                                   0.25, 0.5, 0.5, 0.25, 32, 32);
    //     // assert!(center.len() == 32*32*3, "center was {:?}", center.len());

    //     destroy_vips();
    // }
}
