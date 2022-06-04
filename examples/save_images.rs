use topcodes::scanner::Scanner;

#[cfg(feature = "visualize")]
use image::io::Reader as ImageReader;

fn main() {
    #[cfg(feature = "visualize")]
    {
        let (mut scanner, buffer) = {
            let img = ImageReader::open("assets/photo.png")
                .unwrap()
                .decode()
                .unwrap();
            let (width, height) = (img.width() as usize, img.height() as usize);
            let buffer = img.into_rgb8().into_raw();
            (Scanner::new(width, height), buffer)
        };

        let _res = scanner.scan_rgba_u8(&buffer);
        scanner.write_thresholding_image("target/thresholded.png");
    }

    #[cfg(not(feature = "visualize"))]
    {
        eprintln!("The run target only works with the 'visualize' feature enabled. Use `cargo run --feature visualize` instead.'");
    }
}
