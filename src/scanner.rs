#[cfg(feature = "visualize")]
use image::{GrayImage, ImageBuffer};

use crate::{candidate::Candidate, topcode::TopCode};

/// Default maximum width of a TopCode unit/ring in pixels. This is equivalent to 640 pixels.
const DEFAULT_MAX_UNIT: usize = 80;

#[repr(u8)]
enum UnitLevel {
    WhiteRegion = 0,
    BlackRegion = 1,
    WhiteRegionSecond = 2,
    BlackRegionSecond = 3,
}

/// Loads and scans images for TopCodes.  The algorithm does a single sweep of an image (scanning
/// one horizontal line at a time) looking for TopCode bullseye patterns.  If the pattern matches
/// and the black and white regions meet certain ratio constraints, then the pixel is tested as the
/// center of a candidate TopCode.
#[derive(Clone)]
pub struct Scanner {
    /// Expected image width
    width: usize,
    /// Expected image height
    height: usize,
    /// Holds processed binary pixel data as a single u32 in the ARGB format.
    data: Vec<u32>,
    /// Maximum width of a TopCode unit in pixels
    max_unit: usize,
}

impl Scanner {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            data: vec![0; width * height],
            max_unit: DEFAULT_MAX_UNIT,
        }
    }

    pub fn image_width(&self) -> usize {
        self.width
    }

    pub fn image_height(&self) -> usize {
        self.height
    }

    /// Scan the image and return a list of all TopCodes found in it.
    pub fn scan<T: ?Sized>(
        &mut self,
        image_buffer: &T,
        decode_rgb: impl Fn(&T, usize) -> (u32, u32, u32),
    ) -> Vec<TopCode> {
        let candidates = self.threshold(image_buffer, decode_rgb);
        self.find_codes(&candidates)
    }

    /// Sets the maximum allowable diameter (in pixels) for a TopCode identified by the scanner.
    /// Setting this to a reasonable value for your application will reduce false positives
    /// (recognizing codes that aren't actually there) and improve performance (because fewer
    /// candidate codes will be tested). Setting this value to as low as 50 or 60 pixels could be
    /// advisable for some applications. However, setting the maximum diameter too low will prevent
    /// valid codes from being recognized.
    pub fn set_max_code_diameter(&mut self, diameter: usize) {
        let f = diameter as f64 / 8.0;
        self.max_unit = f.ceil() as usize;
    }

    /// Average of thresholded pixels in a 3x3 region around (x, y). Returned value is between 0
    /// (black) and 255 (white).
    pub(crate) fn get_sample_3x3(&self, x: usize, y: usize) -> usize {
        if x < 1 || x >= self.width - 1 || y < 1 || y >= self.height - 1 {
            return 0;
        }

        let mut sum = 0;
        for j in y - 1..=y + 1 {
            for i in x - 1..=x + 1 {
                let pixel = self.data[j * self.width + i];
                sum += 0xff * (pixel >> 24 & 0x01);
            }
        }

        (sum / 9) as usize
    }

    /// Average of thresholded pixels in a 3x3 region around (x, y). Returned value is either 0
    /// (black) or 1 (white).
    pub(crate) fn get_bw_3x3(&self, x: usize, y: usize) -> u32 {
        if x < 1 || x >= self.width - 1 || y < 1 || y >= self.height - 1 {
            return 0;
        }

        let mut sum = 0;
        for j in y - 1..=y + 1 {
            for i in x - 1..=x + 1 {
                let pixel = self.data[j * self.width + i];
                sum += pixel >> 24 & 0x01;
            }
        }

        if sum >= 5 {
            1
        } else {
            0
        }
    }

    /// Perform Wellner adaptive thresholding to produce binary pixel data. Also mark candidate
    /// SpotCode locations.
    ///
    /// "Adaptive Thresholding for the DigitalDesk"
    /// EuroPARC Technical Report EPC-93-110
    fn threshold<T: ?Sized>(
        &mut self,
        image_buffer: &T,
        decode_rgb: impl Fn(&T, usize) -> (u32, u32, u32),
    ) -> Vec<Candidate> {
        let mut candidates = Vec::with_capacity(50);
        let mut sum = 128;
        let s = 32;

        for j in 0..self.height {
            let mut level = UnitLevel::WhiteRegion;
            let mut b1: isize = 0;
            let mut b2: isize = 0;
            let mut w1: isize = 0;

            let mut k = if j % 2 == 0 { 0 } else { self.width - 1 };
            k += j * self.width;

            for _i in 0..self.width {
                // Calculate pixel intensity (0-255)
                let (r, g, b) = decode_rgb(image_buffer, k);
                let mut a: isize = (r + g + b) as isize / 3;

                // Calculate the average sum as an approximate sum of the last s pixels
                sum += a - (sum / s);

                // Factor in sum from the previous row
                let threshold = if k >= self.width {
                    (sum + (self.data[k - self.width] as isize & 0xffffff)) / (2 * s)
                } else {
                    sum / s
                };

                // Compare the average sum to current pixel to decide black or white
                a = if (a as f64) < (threshold as f64 * 0.975) {
                    0
                } else {
                    1
                };

                // Repack pixel data with binary data in the alpha channel, and the running some
                // for this pixel in the RGB channels.
                self.data[k] = ((a << 24) + (sum & 0xffffff)) as u32;

                match level {
                    UnitLevel::WhiteRegion => {
                        if a == 0 {
                            // First black pixel encountered
                            level = UnitLevel::BlackRegion;
                            b1 = 1;
                            w1 = 0;
                            b2 = 0;
                        }
                    }
                    UnitLevel::BlackRegion => {
                        if a == 0 {
                            b1 += 1;
                        } else {
                            level = UnitLevel::WhiteRegionSecond;
                            w1 = 1;
                        }
                    }
                    UnitLevel::WhiteRegionSecond => {
                        if a == 0 {
                            level = UnitLevel::BlackRegionSecond;
                            b2 = 1;
                        } else {
                            w1 += 1;
                        }
                    }
                    UnitLevel::BlackRegionSecond => {
                        let max_u = self.max_unit as isize;
                        if a == 0 {
                            b2 += 1;
                        } else {
                            if b1 >= 2
                                && b2 >= 2
                                && b1 <= max_u
                                && b2 <= max_u
                                && w1 <= (max_u + max_u)
                                && (b1 + b2 - w1).abs() <= (b1 + b2)
                                && (b1 + b2 - w1).abs() <= w1
                                && (b1 - b2).abs() <= b1
                                && (b1 - b2).abs() <= b2
                            {
                                let mut dk: usize = 1 + b2 as usize + (w1 as usize >> 1);
                                dk = if j % 2 == 0 { k - dk } else { k + dk };

                                candidates.push(Candidate::new(dk % self.width, j));
                            }
                            b1 = b2;
                            w1 = 1;
                            b2 = 0;
                            level = UnitLevel::WhiteRegionSecond;
                        }
                    }
                }
                if j % 2 == 0 {
                    k += 1
                } else {
                    k -= 1
                };
            }
        }

        candidates
    }

    /// Scan the image line by line looking for TopCodes.
    fn find_codes(&self, candidates: &Vec<Candidate>) -> Vec<TopCode> {
        let mut spots = Vec::with_capacity(candidates.len());

        for c in candidates {
            if !self.overlaps(&spots, c.x, c.y) {
                let mut spot = TopCode::default();
                spot.decode(self, c.x, c.y);
                if spot.is_valid() {
                    spots.push(spot);
                }
            }
        }

        spots
    }

    fn overlaps(&self, spots: &Vec<TopCode>, x: usize, y: usize) -> bool {
        for top in spots {
            if top.in_bullseye(x as f64, y as f64) {
                return true;
            }
        }

        false
    }

    /// Counts the number of pixels from (x, y) until a color change is perceived.
    pub(crate) fn dist(&self, x: usize, y: usize, dx: isize, dy: isize) -> isize {
        let start = self.get_bw_3x3(x, y);

        let mut i = x as isize + dx;
        let mut j = y as isize + dy;

        loop {
            if i <= 1 || i >= self.width as isize - 1 || j <= 1 || j >= self.height as isize {
                break;
            }

            let sample = self.get_bw_3x3(i as usize, j as usize);
            if start + sample == 1 {
                let x_dist = (i - x as isize).abs();
                let y_dist = (j - y as isize).abs();
                return x_dist + y_dist;
            }

            i += dx;
            j += dy;
        }

        -1
    }

    #[cfg(feature = "visualize")]
    pub fn write_thresholding_image(&self, path: &str) {
        let img = GrayImage::from_fn(self.width as u32, self.height as u32, |x, y| {
            let index = (y * self.width as u32 + x) as usize;
            let pixel = self.data[index];
            let a = ((pixel >> 24) * 0xff) as u8;
            image::Luma([a])
        });
        img.save(path).expect("Failed to save png image");
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use image::io::Reader as ImageReader;

    fn setup(asset_name: &str) -> (Scanner, Vec<u8>) {
        let img = ImageReader::open(format!("assets/{}.png", asset_name))
            .unwrap()
            .decode()
            .unwrap();
        let (width, height) = (img.width() as usize, img.height() as usize);
        let image_raw = img.into_rgb8().into_raw();
        (Scanner::new(width, height), image_raw)
    }

    #[test]
    fn it_can_scan_a_source_image_accurately() {
        let (mut scanner, buffer) = setup("source");
        let topcodes = scanner.scan(&buffer, |buffer, index| {
            (
                buffer[index * 3] as u32,
                buffer[index * 3 + 1] as u32,
                buffer[index * 3 + 2] as u32,
            )
        });

        assert_eq!(
            topcodes,
            vec![
                TopCode {
                    code: Some(55),
                    unit: 48.8125,
                    orientation: -0.07249829200591831,
                    x: 1803.0,
                    y: 878.0,
                    core: [0, 255, 0, 255, 255, 0, 255, 255]
                },
                TopCode {
                    code: Some(31),
                    unit: 48.675,
                    orientation: -0.07249829200591831,
                    x: 618.0,
                    y: 923.0,
                    core: [0, 255, 0, 255, 255, 0, 255, 255]
                },
                TopCode {
                    code: Some(93),
                    unit: 39.825,
                    orientation: -0.07249829200591831,
                    x: 1275.3333333333333,
                    y: 1704.0,
                    core: [56, 255, 0, 255, 255, 0, 255, 255]
                }
            ]
        );
    }

    #[test]
    fn it_can_scan_a_photo_accurately() {
        let (mut scanner, buffer) = setup("photo");
        let topcodes = scanner.scan(&buffer, |buffer, index| {
            (
                buffer[index * 3] as u32,
                buffer[index * 3 + 1] as u32,
                buffer[index * 3 + 2] as u32,
            )
        });

        assert_eq!(
            topcodes,
            vec![
                TopCode {
                    code: Some(55),
                    unit: 22.44375,
                    orientation: -0.07249829200591831,
                    x: 996.8333333333334,
                    y: 493.5,
                    core: [0, 255, 0, 255, 255, 0, 255, 255]
                },
                TopCode {
                    code: Some(31),
                    unit: 22.91875,
                    orientation: 0.024166097335306114,
                    x: 366.5,
                    y: 510.0,
                    core: [0, 255, 0, 255, 255, 0, 255, 255]
                },
                TopCode {
                    code: Some(93),
                    unit: 21.15,
                    orientation: -0.07249829200591831,
                    x: 718.8333333333334,
                    y: 929.5,
                    core: [113, 255, 0, 255, 255, 0, 255, 255]
                }
            ]
        );
    }
}
