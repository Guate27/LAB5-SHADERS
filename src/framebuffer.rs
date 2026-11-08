// Framebuffer con buffer de color y z-buffer
use std::fs::File;
use std::io::{self, Write};

pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub buffer: Vec<u32>,  // color de cada píxel (0xRRGGBB)
    pub zbuffer: Vec<f32>, // profundidad de cada píxel
    background_color: u32,
    current_color: u32,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Framebuffer {
            width,
            height,
            buffer: vec![0; width * height],
            // empieza en infinito para que cualquier fragmento pase la primera vez
            zbuffer: vec![f32::INFINITY; width * height],
            background_color: 0x000000,
            current_color: 0xFFFFFF,
        }
    }

    // Limpia color y profundidad antes de cada frame
    pub fn clear(&mut self) {
        self.buffer.fill(self.background_color);
        self.zbuffer.fill(f32::INFINITY);
    }

    pub fn set_background_color(&mut self, color: u32) {
        self.background_color = color;
    }

    pub fn set_current_color(&mut self, color: u32) {
        self.current_color = color;
    }

    // Pinta el píxel solo si está dentro de la ventana y más cerca que lo que ya hay
    pub fn point(&mut self, x: usize, y: usize, depth: f32) {
        if x >= self.width || y >= self.height {
            return;
        }
        let index = y * self.width + x; // el buffer es un arreglo de una dimensión
        if depth < self.zbuffer[index] {
            self.zbuffer[index] = depth;
            self.buffer[index] = self.current_color;
        }
    }

    // Revisa la profundidad sin pintar, para no correr el shader en píxeles tapados
    pub fn is_closer(&self, x: usize, y: usize, depth: f32) -> bool {
        depth < self.zbuffer[y * self.width + x]
    }

    // Estrellas de fondo en posiciones fijas
    pub fn draw_stars(&mut self, count: usize) {
        // generador congruencial lineal con semilla fija
        let mut seed: u32 = 12345;
        for _ in 0..count {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            let x = (seed >> 8) as usize % self.width;
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            let y = (seed >> 8) as usize % self.height;
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            let b = 90 + (seed >> 8) % 166; // brillo entre 90 y 255
            // azul con un mínimo de 110 para que las estrellas tenues se vean azuladas
            self.buffer[y * self.width + x] = (b << 16) | (b << 8) | b.max(110);
        }
    }

    // Guarda el frame como .bmp de 24 bits (para las capturas del README)
    pub fn save_bmp(&self, path: &str) -> io::Result<()> {
        let row_size = (self.width * 3 + 3) & !3; // cada fila se rellena a múltiplo de 4 bytes
        let data_size = row_size * self.height;
        let file_size = 54 + data_size;
        let mut f = File::create(path)?;

        // encabezado del archivo (14 bytes) y encabezado DIB (40 bytes)
        f.write_all(b"BM")?;
        f.write_all(&(file_size as u32).to_le_bytes())?;
        f.write_all(&0u32.to_le_bytes())?;
        f.write_all(&54u32.to_le_bytes())?;
        f.write_all(&40u32.to_le_bytes())?;
        f.write_all(&(self.width as i32).to_le_bytes())?;
        f.write_all(&(self.height as i32).to_le_bytes())?;
        f.write_all(&1u16.to_le_bytes())?;
        f.write_all(&24u16.to_le_bytes())?;
        f.write_all(&[0u8; 24])?;

        // BMP guarda las filas de abajo hacia arriba y cada píxel en orden BGR
        let mut row = vec![0u8; row_size];
        for y in (0..self.height).rev() {
            for x in 0..self.width {
                let c = self.buffer[y * self.width + x];
                row[x * 3] = (c & 0xFF) as u8;
                row[x * 3 + 1] = ((c >> 8) & 0xFF) as u8;
                row[x * 3 + 2] = ((c >> 16) & 0xFF) as u8;
            }
            f.write_all(&row)?;
        }
        Ok(())
    }
}
