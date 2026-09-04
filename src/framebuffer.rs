use raylib::prelude::*;

/// Buffer de color en memoria. Aca se dibuja todo el render y recien al
/// final se sube a una textura de raylib para mostrarlo en la ventana.
pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    buffer: Vec<Color>,
    /// Color con el que `clear` deja el buffer.
    background_color: Color,
    /// Color con el que pinta `point`. Se setea antes de dibujar.
    current_color: Color,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Framebuffer {
            width,
            height,
            buffer: vec![Color::BLACK; width * height],
            background_color: Color::BLACK,
            current_color: Color::WHITE,
        }
    }

    pub fn set_background_color(&mut self, color: Color) {
        self.background_color = color;
    }

    pub fn set_current_color(&mut self, color: Color) {
        self.current_color = color;
    }

    pub fn clear(&mut self) {
        for px in self.buffer.iter_mut() {
            *px = self.background_color;
        }
    }

    /// El unico punto de escritura del buffer. No recibe color: pinta con
    /// el color actual, que se define antes con `set_current_color`.
    pub fn point(&mut self, x: usize, y: usize) {
        if x < self.width && y < self.height {
            self.buffer[y * self.width + x] = self.current_color;
        }
    }

    /// Convierte el buffer en una Image de raylib para poder subirlo a la
    /// GPU. Es la unica salida del framebuffer hacia la ventana.
    pub fn to_image(&self) -> Image {
        let mut image =
            Image::gen_image_color(self.width as i32, self.height as i32, self.background_color);

        for y in 0..self.height {
            for x in 0..self.width {
                image.draw_pixel(x as i32, y as i32, self.buffer[y * self.width + x]);
            }
        }

        image
    }
}
