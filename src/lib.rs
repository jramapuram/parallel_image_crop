extern crate libc;
extern crate image;
extern crate rayon;
 #[macro_use] extern crate itertools;

use std::ptr;
use std::fs::File;
use std::path::Path;
use std::ffi::{CStr, OsStr, CString};
use std::{slice, str, mem};
use rayon::prelude::*;
use libc::{size_t, c_char, c_uchar, c_void, c_double, c_longlong};
use image::{GenericImage, ImageBuffer, imageops, FilterType, ColorType, ImageDecoder};

mod lazy_load;
mod vips_ffi;

fn scale_range(val: f32, newmin: f32, newmax: f32) -> f32 {
    // simple helper to scale a value range
    (((val) * (newmax - newmin)) / (1.0)) + newmin
}


#[no_mangle]
pub fn lazy_crop_and_resize(path: &str, scale: f32, x_crop: f32, y_crop: f32,
                            max_img_percent: f32, resize_width: u32, resize_height: u32) -> image::DynamicImage
{
    assert!(x_crop >= 0f32 && x_crop <= 1f32, "x of crop not bounded in [0, 1]");
    assert!(y_crop >= 0f32 && y_crop <= 1f32, "y of crop not bounded in [0, 1]");

    // read the image and grab the size TODO: read using decoder
    let img_size = lazy_load::dimensions(path).unwrap();

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

    // crop the image and resize it
    lazy_load::lazy_crop_to_image(path, x, y, crop_size.0, crop_size.1).unwrap().resize_exact(
        resize_width, resize_height, FilterType::Nearest
    )
}


#[no_mangle]
pub fn crop_and_resize(path: &str, scale: f32, x_crop: f32, y_crop: f32,
                       max_img_percent: f32, resize_width: u32, resize_height: u32) -> image::DynamicImage
{
    assert!(x_crop >= 0f32 && x_crop <= 1f32, "x of crop not bounded in [0, 1]");
    assert!(y_crop >= 0f32 && y_crop <= 1f32, "y of crop not bounded in [0, 1]");

    // read the image and grab the size TODO: read using decoder
    let mut img = image::open(&Path::new(&path)).unwrap();
    let img_size = img.dimensions();

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

    // crop the image and resize it
    img.crop(x, y, crop_size.0, crop_size.1).resize_exact(
        resize_width, resize_height, FilterType::Nearest
    )
}


#[no_mangle]
pub extern "C" fn parallel_crop_and_resize(image_paths_ptr: *const *const c_char,
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
    image_paths_vec.into_par_iter().zip(scale_values)
        .zip(x_values.par_iter()).zip(y_values)
        .map(|(((path, scale), x), y)| {
            crop_and_resize(path,
                            *scale, *x, *y,
                            max_img_percent,
                            window_size,
                            window_size).raw_pixels()
                 // Array::from_vec(crop_and_resize(path,
                 //                                 *scale, *x, *y,
                 //                                 max_img_percent,
                 //                                 window_size,
                 //                                 window_size).raw_pixels()
                 // )
             }).collect_into_vec(&mut resultant_vec);

    // image_paths_vec.par_iter().zip(scale_values)
    //     .zip(x_values.par_iter()).zip(y_values)
    //     .zip(return_vec.par_iter_mut())
    //     .for_each(|((((path, scale), x), y), r)| {
    //         *r = Array::from_vec(crop_and_resize(*path,
    //                                              *scale, *x, *y,
    //                                              max_img_percent,
    //                                              window_size,
    //                                              window_size).raw_pixels()
    //         ).data;
    //     });

    //let = unsafe { slice::from_raw_parts(x_ptr, length as usize) };

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
    fn test_bw_image_crop() {
        // test the center crop and ensure that the crop is also RGB
        let center = crop_and_resize("assets/lena_gray.png",
                                     0.25, 0.5, 0.5, 0.25, 32, 32);
        println!("dims: {:?}", center.dimensions());
        center.save(&Path::new("assets/test_lena_gray_center.png")).unwrap();
        assert!(center.dimensions() == (32, 32));
        let is_color = match center.color() {
            ColorType::Gray(_) => false,
            _ => true
        };
        assert!(is_color == false);
    }

    #[test]
    fn test_image_crops() {
        // test the top left crop
        let top_left = crop_and_resize("assets/lena.png",
                                       0.25, 0.25, 0.25, 0.25, 32, 32);
        println!("dims: {:?}", top_left.dimensions());
        top_left.save(&Path::new("assets/test_lena_top_left.png")).unwrap();
        assert!(top_left.dimensions() == (32, 32), "top left was {:?}", top_left.dimensions());

        // test the top right crop
        let top_right = crop_and_resize("assets/lena.png",
                                        0.25, 0.75, 0.25, 0.25, 32, 32);
        top_right.save(&Path::new("assets/test_lena_top_right.png")).unwrap();
        assert!(top_right.dimensions() == (32, 32), "top right was {:?}", top_right.dimensions());

        // test the bottom left crop
        let bottom_left = crop_and_resize("assets/lena.png",
                                          0.25, 0.25, 0.75, 0.25, 32, 32);
        bottom_left.save(&Path::new("assets/test_lena_bottom_left.png")).unwrap();
        assert!(bottom_left.dimensions() == (32, 32), "bottom left was {:?}", bottom_left.dimensions());

        // test the bottom right crop
        let bottom_right = crop_and_resize("assets/lena.png",
                                           0.25, 0.75, 0.75, 0.25, 32, 32);
        bottom_right.save(&Path::new("assets/test_lena_bottom_right.png")).unwrap();
        assert!(bottom_right.dimensions() == (32, 32), "bottom right was {:?}", bottom_right.dimensions());

        // test the center crop
        let center = crop_and_resize("assets/lena.png",
                                     0.25, 0.5, 0.5, 0.25, 32, 32);
        center.save(&Path::new("assets/test_lena_center.png")).unwrap();
        assert!(center.dimensions() == (32, 32), "center right was {:?}", center.dimensions());
    }
}
