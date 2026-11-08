// Vectores, matrices y funciones para mezclar colores
use std::ops::{Add, AddAssign, Mul, Neg, Sub};

// Vector de 3 componentes. Se usa para posiciones, direcciones y colores (r, g, b)
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Vec3 { x, y, z }
    }

    // Producto punto. Con vectores normalizados da el coseno del ángulo entre ellos
    pub fn dot(&self, o: Vec3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    // Producto cruz: vector perpendicular a los dos (lo usa la cámara)
    pub fn cross(&self, o: Vec3) -> Vec3 {
        Vec3::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }

    pub fn length(&self) -> f32 {
        self.dot(*self).sqrt()
    }

    // Deja el vector con largo 1 manteniendo la dirección
    pub fn normalize(&self) -> Vec3 {
        let len = self.length();
        if len > 0.0 {
            *self * (1.0 / len)
        } else {
            *self
        }
    }
}

// Operadores para poder escribir a + b, a - b, a * k, a += b y -a

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl AddAssign for Vec3 {
    fn add_assign(&mut self, o: Vec3) {
        self.x += o.x;
        self.y += o.y;
        self.z += o.z;
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, s: f32) -> Vec3 {
        Vec3::new(self.x * s, self.y * s, self.z * s)
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}

// ---------- Funciones de color ----------

pub fn clamp(x: f32, lo: f32, hi: f32) -> f32 {
    x.max(lo).min(hi)
}

// Interpolación lineal entre a y b (t = 0 da a, t = 1 da b)
pub fn mix(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    a + (b - a) * t
}

// Devuelve 0 si x < e0, 1 si x > e1 y una transición suave entre los dos
pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = clamp((x - e0) / (e1 - e0), 0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// Color en hexadecimal (0xRRGGBB) a Vec3 con valores de 0 a 1
pub const fn rgb(hex: u32) -> Vec3 {
    Vec3::new(
        ((hex >> 16) & 0xFF) as f32 / 255.0,
        ((hex >> 8) & 0xFF) as f32 / 255.0,
        (hex & 0xFF) as f32 / 255.0,
    )
}

// Vec3 de 0 a 1 al formato 0xRRGGBB del framebuffer.
// Se recorta primero porque algunas capas suman brillo y pueden pasarse de 1
pub fn to_u32(c: Vec3) -> u32 {
    let r = (clamp(c.x, 0.0, 1.0) * 255.0) as u32;
    let g = (clamp(c.y, 0.0, 1.0) * 255.0) as u32;
    let b = (clamp(c.z, 0.0, 1.0) * 255.0) as u32;
    (r << 16) | (g << 8) | b
}

// ---------- Matrices 4x4 ----------

#[derive(Clone, Copy, Debug)]
pub struct Mat4 {
    pub m: [[f32; 4]; 4], // m[fila][columna]
}

impl Mat4 {
    pub fn identity() -> Self {
        let mut m = [[0.0; 4]; 4];
        for (i, fila) in m.iter_mut().enumerate() {
            fila[i] = 1.0;
        }
        Mat4 { m }
    }

    // La traslación va en la última columna
    pub fn translation(t: Vec3) -> Self {
        let mut r = Mat4::identity();
        r.m[0][3] = t.x;
        r.m[1][3] = t.y;
        r.m[2][3] = t.z;
        r
    }

    // Escala uniforme. La esfera mide 1 de radio, así que la escala es el radio final
    pub fn scale(s: f32) -> Self {
        let mut r = Mat4::identity();
        r.m[0][0] = s;
        r.m[1][1] = s;
        r.m[2][2] = s;
        r
    }

    // Rotación en el eje Y, para que el planeta gire sobre sí mismo
    pub fn rotation_y(a: f32) -> Self {
        let (s, c) = a.sin_cos();
        let mut r = Mat4::identity();
        r.m[0][0] = c;
        r.m[0][2] = s;
        r.m[2][0] = -s;
        r.m[2][2] = c;
        r
    }

    // Rotación en el eje Z, para inclinar el eje del planeta
    pub fn rotation_z(a: f32) -> Self {
        let (s, c) = a.sin_cos();
        let mut r = Mat4::identity();
        r.m[0][0] = c;
        r.m[0][1] = -s;
        r.m[1][0] = s;
        r.m[1][1] = c;
        r
    }

    // Proyección en perspectiva. fov_y es el ángulo de visión vertical,
    // aspect = ancho / alto, near y far son los límites de lo que se ve
    pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        let f = 1.0 / (fov_y / 2.0).tan();
        let mut r = Mat4 { m: [[0.0; 4]; 4] };
        r.m[0][0] = f / aspect;
        r.m[1][1] = f;
        r.m[2][2] = (far + near) / (near - far);
        r.m[2][3] = (2.0 * far * near) / (near - far);
        r.m[3][2] = -1.0; // pasa la profundidad a w para la división de perspectiva
        r
    }

    // Matriz de vista: cámara en eye mirando hacia target
    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let f = (target - eye).normalize(); // adelante
        let s = f.cross(up).normalize(); // derecha
        let u = s.cross(f); // arriba
        let mut r = Mat4::identity();
        r.m[0] = [s.x, s.y, s.z, -s.dot(eye)];
        r.m[1] = [u.x, u.y, u.z, -u.dot(eye)];
        r.m[2] = [-f.x, -f.y, -f.z, f.dot(eye)];
        r
    }

    // Multiplicación de matrices. A.mul(B) aplica primero B y luego A
    pub fn mul(&self, o: &Mat4) -> Mat4 {
        let mut r = Mat4 { m: [[0.0; 4]; 4] };
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    r.m[i][j] += self.m[i][k] * o.m[k][j];
                }
            }
        }
        r
    }

    // Transforma un punto (w = 1). Devuelve x, y, z, w
    pub fn transform_point(&self, p: Vec3) -> [f32; 4] {
        let mut out = [0.0; 4];
        for (i, o) in out.iter_mut().enumerate() {
            *o = self.m[i][0] * p.x + self.m[i][1] * p.y + self.m[i][2] * p.z + self.m[i][3];
        }
        out
    }

    // Transforma una dirección (w = 0), sin aplicar la traslación. Se usa para las normales
    pub fn transform_dir(&self, d: Vec3) -> Vec3 {
        Vec3::new(
            self.m[0][0] * d.x + self.m[0][1] * d.y + self.m[0][2] * d.z,
            self.m[1][0] * d.x + self.m[1][1] * d.y + self.m[1][2] * d.z,
            self.m[2][0] * d.x + self.m[2][1] * d.y + self.m[2][2] * d.z,
        )
    }
}
