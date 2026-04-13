use image::{GrayAlphaImage, Pixel, imageops::resize};
use log::debug;

pub fn image_to_ascii(
    bytes: &GrayAlphaImage,
    downscale: (u32, u32),
    characters: &Vec<char>,
) -> String {
    //later - characters represent clusters - custom downsizing?

    let mut small = resize(
        bytes,
        downscale.0,
        downscale.1,
        image::imageops::FilterType::Nearest,
    );

    let mut result = "".to_string();
    let mut crop;
    for row in small.enumerate_rows_mut() {
        crop = true;
        let mut r = "".to_string();
        for pixel in row.1.enumerate() {
            let (_, (_, _, data)) = pixel;
            if data.alpha() == 0 {
                r.push(' ');
            } else {
                crop = false;
                let brightness = data.channels().get(0).unwrap_or(&0).to_owned() as f32 / 255.;
                let index: usize = (brightness * (characters.len() - 1) as f32) as usize;
                r.push(characters[index]);
            }
        }
        r.push('\n');
        if !crop {
            result = format!("{} {}", result, r);
        }

        //how to deal with squishing in the middle??
    }

    return result;
}
