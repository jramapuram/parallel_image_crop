extern crate image;

use std::io::BufReader;
use std::path::Path;
use std::fs::File;
use rayon::prelude::*;

use image::jpeg;
use image::png;
use image::tiff;
use image::bmp;
use image::tga;
use image::gif;
use image::hdr;
use image::ico;
use image::pnm;
#[allow(unused)]
use image::{ImageDecoder, ImageFormat, ImageResult,
            DynamicImage, FilterType};


#[allow(dead_code)]
pub fn get_image_format(path: &Path) -> ImageResult<ImageFormat>
{
    let ext = path.extension().and_then(|s| s.to_str())
                  .map_or("".to_string(), |s| s.to_ascii_lowercase());

    match &ext[..] {
        "jpg" |
        "jpeg" => Ok(image::ImageFormat::JPEG),
        "png"  => Ok(image::ImageFormat::PNG),
        "gif"  => Ok(image::ImageFormat::GIF),
        "webp" => Ok(image::ImageFormat::WEBP),
        "tif" |
        "tiff" => Ok(image::ImageFormat::TIFF),
        "tga" =>  Ok(image::ImageFormat::TGA),
        "bmp" =>  Ok(image::ImageFormat::BMP),
        "ico" =>  Ok(image::ImageFormat::ICO),
        "hdr" =>  Ok(image::ImageFormat::HDR),
        "pbm" |
        "pam" |
        "ppm" |
        "pgm" =>  Ok(image::ImageFormat::PNM),
        format => return Err(image::ImageError::UnsupportedError(format!(
            "Image format image/{:?} is not supported.",
            format
        )))
    }
}

// Create a new image from a Reader
#[allow(dead_code)]
pub fn dimensions(path_str: &str) -> ImageResult<(u32, u32)>
{
    let path = Path::new(&path_str);
    let format = get_image_format(path).unwrap();

    // the file input reader
    let fin = match File::open(path) {
        Ok(f)  => f,
        Err(err) => return Err(image::ImageError::IoError(err))
    };
    let fin = BufReader::new(fin);

    match format {
        image::ImageFormat::PNG  => Ok(png::PNGDecoder::new(fin).dimensions().unwrap()),
        image::ImageFormat::GIF  => Ok(gif::Decoder::new(fin).dimensions().unwrap()),
        image::ImageFormat::JPEG => Ok(jpeg::JPEGDecoder::new(fin).dimensions().unwrap()),
        image::ImageFormat::TIFF => Ok(try!(tiff::TIFFDecoder::new(fin)).dimensions().unwrap()),
        image::ImageFormat::TGA => Ok(tga::TGADecoder::new(fin).dimensions().unwrap()),
        image::ImageFormat::BMP => Ok(bmp::BMPDecoder::new(fin).dimensions().unwrap()),
        image::ImageFormat::ICO => Ok(try!(ico::ICODecoder::new(fin)).dimensions().unwrap()),
        image::ImageFormat::HDR => Ok(try!(hdr::HDRAdapter::new(BufReader::new(fin))).dimensions().unwrap()),
        image::ImageFormat::PNM => Ok(try!(pnm::PNMDecoder::new(BufReader::new(fin))).dimensions().unwrap()),
        _ => Err(image::ImageError::UnsupportedError(format!("A decoder for {:?} is not available.", format))),
    }
}


#[allow(dead_code)]
pub fn vec_to_image(v: &Vec<u8>) -> ImageResult<DynamicImage>
{
    image::load_from_memory(&v)
}


#[allow(dead_code)]
pub fn lazy_crop_to_image(path: &str, x: u32, y: u32, width: u32, length: u32) -> ImageResult<DynamicImage>
{
    let buf = lazy_crop_to_vec(path, x, y, width, length).unwrap();
    vec_to_image(&buf)
}


#[allow(dead_code)]
pub fn lazy_crop_to_vec(path_str: &str, x: u32, y: u32, width: u32, length: u32) -> ImageResult<Vec<u8>>
{
    let path = Path::new(&path_str);
    let format = get_image_format(path).unwrap();

    // the file input reader
    let fin = match File::open(path) {
        Ok(f)  => f,
        Err(err) => return Err(image::ImageError::IoError(err))
    };
    let fin = BufReader::new(fin);

    println!("img_dims = {:?}, x = {:?} | y = {:?} | width = {:?} | len = {:?}", dimensions(&path_str).unwrap(), x, y, width, length);
    match format {
        image::ImageFormat::PNG  => png::PNGDecoder::new(fin).load_rect(x, y, length, width),
        image::ImageFormat::GIF  => gif::Decoder::new(fin).load_rect(x, y, length, width),
        image::ImageFormat::JPEG => jpeg::JPEGDecoder::new(fin).load_rect(x, y, length, width),
        image::ImageFormat::TIFF => try!(tiff::TIFFDecoder::new(fin)).load_rect(x, y, length, width),
        image::ImageFormat::TGA => tga::TGADecoder::new(fin).load_rect(x, y, length, width),
        image::ImageFormat::BMP => bmp::BMPDecoder::new(fin).load_rect(x, y, length, width),
        image::ImageFormat::ICO => try!(ico::ICODecoder::new(fin)).load_rect(x, y, length, width),
        image::ImageFormat::HDR => try!(hdr::HDRAdapter::new(BufReader::new(fin))).load_rect(x, y, length, width),
        image::ImageFormat::PNM => try!(pnm::PNMDecoder::new(BufReader::new(fin))).load_rect(x, y, length, width),
        _ => Err(image::ImageError::UnsupportedError(format!("A decoder for {:?} is not available.", format))),
    }
}

#[allow(dead_code)]
pub fn lazy_crop_and_resize(path: &str, scale: f32, x_crop: f32, y_crop: f32,
                            max_img_percent: f32, resize_width: u32, resize_height: u32) -> image::DynamicImage
{
    assert!(x_crop >= 0f32 && x_crop <= 1f32, "x of crop not bounded in [0, 1]");
    assert!(y_crop >= 0f32 && y_crop <= 1f32, "y of crop not bounded in [0, 1]");

    // read the image and grab the size TODO: read using decoder
    let img_size = dimensions(path).unwrap();

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

    // crop the image and resize it
    lazy_crop_to_image(path, x, y, crop_size.0, crop_size.1).unwrap().resize_exact(
        resize_width, resize_height, FilterType::Nearest
    )
}
