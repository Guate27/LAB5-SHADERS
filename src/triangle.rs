// Rasterización de triángulos con coordenadas baricéntricas
use crate::framebuffer::Framebuffer;
use crate::math::{to_u32, Vec3};

// Salida del vertex shader para cada vértice
#[derive(Clone, Copy)]
pub struct ScreenVertex {
    pub screen: Vec3, // x, y en píxeles y z = profundidad
    pub world: Vec3,  // posición en el mundo (para la luz)
    pub normal: Vec3, // normal en el mundo (para la luz)
    pub local: Vec3,  // posición en el modelo, para que los patrones giren con el planeta
}

// Datos interpolados que recibe el fragment shader
pub struct Fragment {
    pub world: Vec3,
    pub normal: Vec3,
    pub local: Vec3,
}

// Función de arista: el signo indica de qué lado de la recta a-b está el punto
fn edge(a: Vec3, b: Vec3, px: f32, py: f32) -> f32 {
    (px - a.x) * (b.y - a.y) - (py - a.y) * (b.x - a.x)
}

// Dibuja un triángulo. shade es el fragment shader; si devuelve None el píxel no se pinta
pub fn draw_triangle<F>(
    fb: &mut Framebuffer,
    v: [&ScreenVertex; 3],
    cull_backfaces: bool,
    mut shade: F,
) where
    F: FnMut(&Fragment) -> Option<Vec3>,
{
    let (a, b, c) = (v[0].screen, v[1].screen, v[2].screen);

    // área con signo del triángulo en pantalla
    let area = edge(a, b, c.x, c.y);
    if area.abs() < 1e-6 {
        return; // triángulo degenerado
    }
    // área negativa = cara trasera, no se dibuja
    if cull_backfaces && area < 0.0 {
        return;
    }

    // bounding box del triángulo recortado a la ventana
    let min_x = a.x.min(b.x).min(c.x).floor().max(0.0) as usize;
    let min_y = a.y.min(b.y).min(c.y).floor().max(0.0) as usize;
    let max_x = (a.x.max(b.x).max(c.x).ceil() as i64).min(fb.width as i64 - 1);
    let max_y = (a.y.max(b.y).max(c.y).ceil() as i64).min(fb.height as i64 - 1);
    if max_x < 0 || max_y < 0 {
        return;
    }

    for y in min_y..=max_y as usize {
        for x in min_x..=max_x as usize {
            // se evalúa en el centro del píxel
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);

            // coordenadas baricéntricas (suman 1); si alguna es negativa el píxel está afuera
            let w0 = edge(b, c, px, py) / area;
            let w1 = edge(c, a, px, py) / area;
            let w2 = edge(a, b, px, py) / area;
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }

            // profundidad interpolada; se revisa antes de correr el shader
            let depth = a.z * w0 + b.z * w1 + c.z * w2;
            if !fb.is_closer(x, y, depth) {
                continue;
            }

            // interpolación de posición, normal y posición local con los mismos pesos
            let frag = Fragment {
                world: v[0].world * w0 + v[1].world * w1 + v[2].world * w2,
                normal: (v[0].normal * w0 + v[1].normal * w1 + v[2].normal * w2).normalize(),
                local: v[0].local * w0 + v[1].local * w1 + v[2].local * w2,
            };

            if let Some(color) = shade(&frag) {
                fb.set_current_color(to_u32(color));
                fb.point(x, y, depth);
            }
        }
    }
}
