extern crate image;

use std::io::BufReader;
use std::path::Path;
use std::fs::File;

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
            DynamicImage};


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
