// Funciones de ruido procedural para los patrones de los planetas
use crate::math::Vec3;

// Valor pseudoaleatorio entre 0 y 1 para cada celda entera (x, y, z).
// Siempre da lo mismo para la misma celda, así el patrón no cambia entre frames
fn hash(x: i32, y: i32, z: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374_761_393)
        ^ (y as u32).wrapping_mul(668_265_263)
        ^ (z as u32).wrapping_mul(2_246_822_519);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    (h & 0x00FF_FFFF) as f32 / 16_777_215.0
}

// Punto aleatorio dentro de una celda (se usa en el ruido celular)
fn hash_point(x: i32, y: i32, z: i32) -> Vec3 {
    Vec3::new(
        hash(x, y, z),
        hash(x + 17, y - 31, z + 47),
        hash(x - 59, y + 83, z - 11),
    )
}

// Curva para suavizar la interpolación entre celdas
fn fade(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

// Value noise 3D: interpola los valores de las 8 esquinas de la celda donde cae p
pub fn noise(p: Vec3) -> f32 {
    let (xi, yi, zi) = (p.x.floor(), p.y.floor(), p.z.floor());
    let (x, y, z) = (xi as i32, yi as i32, zi as i32);
    // posición dentro de la celda (0 a 1)
    let (fx, fy, fz) = (fade(p.x - xi), fade(p.y - yi), fade(p.z - zi));

    // interpolación en x, luego en y, luego en z
    let x00 = lerp(hash(x, y, z), hash(x + 1, y, z), fx);
    let x10 = lerp(hash(x, y + 1, z), hash(x + 1, y + 1, z), fx);
    let x01 = lerp(hash(x, y, z + 1), hash(x + 1, y, z + 1), fx);
    let x11 = lerp(hash(x, y + 1, z + 1), hash(x + 1, y + 1, z + 1), fx);
    lerp(lerp(x00, x10, fy), lerp(x01, x11, fy), fz)
}

// fBm: suma varias octavas de ruido, cada una con el doble de frecuencia
// y la mitad de amplitud. Más octavas = más detalle pero más lento
pub fn fbm(p: Vec3, octaves: u32) -> f32 {
    let mut sum = 0.0;
    let mut amp = 0.5;
    let mut norm = 0.0;
    let mut q = p;
    for _ in 0..octaves {
        sum += amp * noise(q);
        norm += amp;
        q = q * 2.03 + Vec3::new(1.7, 9.2, 4.1); // desfase para que las octavas no se alineen
        amp *= 0.5;
    }
    sum / norm // normalizado a 0..1
}

// Ruido de crestas: el valor máximo queda donde el fbm vale 0.5,
// lo que forma líneas delgadas (grietas)
pub fn ridged(p: Vec3, octaves: u32) -> f32 {
    let n = fbm(p, octaves);
    1.0 - (n * 2.0 - 1.0).abs()
}

// Ruido celular: distancia al punto aleatorio más cercano.
// Devuelve (distancia, id aleatorio de esa celda). Se usa para cráteres y tormentas
pub fn cellular(p: Vec3) -> (f32, f32) {
    let (xi, yi, zi) = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
    let mut best = 9.0;
    let mut id = 0.0;
    // revisa la celda actual y las 26 vecinas
    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (cx, cy, cz) = (xi + dx, yi + dy, zi + dz);
                let point = Vec3::new(cx as f32, cy as f32, cz as f32) + hash_point(cx, cy, cz);
                let d = (p - point).length();
                if d < best {
                    best = d;
                    id = hash(cx + 101, cy + 7, cz - 3);
                }
            }
        }
    }
    (best, id)
}
