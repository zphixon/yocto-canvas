pub struct Pixel {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

pub struct ImageData {
    pub data: Vec<f32>,
}

impl IntoIterator for ImageData {
    type Item = f32;
    type IntoIter = <Vec<f32> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

pub struct Image {
    data: ImageData,
    width: u32,
    height: u32,
}

impl Image {
    pub fn pixel_at(&self, x: usize, y: usize) -> Pixel {
        Pixel {
            r: self.data.data[(self.width as usize * y + x) * 4],
            g: self.data.data[(self.width as usize * y + x) * 4 + 1],
            b: self.data.data[(self.width as usize * y + x) * 4 + 2],
            a: self.data.data[(self.width as usize * y + x) * 4 + 3],
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, pixel: Pixel) {
        self.data.data[(self.width as usize * y + x) * 4] = pixel.r;
        self.data.data[(self.width as usize * y + x) * 4 + 1] = pixel.g;
        self.data.data[(self.width as usize * y + x) * 4 + 2] = pixel.b;
        self.data.data[(self.width as usize * y + x) * 4 + 3] = pixel.a;
    }

    pub fn set_rgba(&mut self, x: usize, y: usize, r: f32, g: f32, b: f32, a: f32) {
        self.data.data[(self.width as usize * y + x) * 4] = r;
        self.data.data[(self.width as usize * y + x) * 4 + 1] = g;
        self.data.data[(self.width as usize * y + x) * 4 + 2] = b;
        self.data.data[(self.width as usize * y + x) * 4 + 3] = a;
    }

    pub fn as_raw(&self) -> Vec<u8> {
        self.data
            .data
            .iter()
            .map(|float| (float * 256.).floor() as u8)
            .collect()
    }

    pub fn as_mut(&mut self) -> &mut [f32] {
        &mut self.data.data
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}

impl From<image_library::RgbaImage> for Image {
    fn from(image: image_library::RgbaImage) -> Image {
        Image {
            width: image.width(),
            height: image.height(),
            data: ImageData {
                data: image
                    .into_vec()
                    .into_iter()
                    .map(|byte| byte as f32 / 256.0)
                    .collect(),
            },
        }
    }
}
