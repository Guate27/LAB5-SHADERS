// Carga de modelos .obj (solo posiciones, normales y caras)
use crate::math::Vec3;
use std::fs;

#[derive(Clone, Copy)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
}

pub struct Obj {
    // cada 3 vértices seguidos forman un triángulo
    pub vertices: Vec<Vertex>,
}

impl Obj {
    pub fn load(path: &str) -> Result<Obj, String> {
        let text = fs::read_to_string(path).map_err(|e| format!("No se pudo leer {path}: {e}"))?;

        let mut positions: Vec<Vec3> = Vec::new(); // líneas "v"
        let mut normals: Vec<Vec3> = Vec::new(); // líneas "vn"
        let mut vertices: Vec<Vertex> = Vec::new();

        for line in text.lines() {
            let mut parts = line.split_whitespace();
            match parts.next() {
                Some("v") => positions.push(parse_vec3(&mut parts)),
                Some("vn") => normals.push(parse_vec3(&mut parts)),
                Some("f") => {
                    // formatos posibles por esquina: v, v/vt, v//vn, v/vt/vn
                    // (los índices del .obj empiezan en 1)
                    let corners: Vec<Vertex> = parts
                        .map(|c| {
                            let idx: Vec<&str> = c.split('/').collect();
                            let p = positions[idx[0].parse::<usize>().unwrap() - 1];
                            let n = match idx.get(2) {
                                Some(s) if !s.is_empty() => normals[s.parse::<usize>().unwrap() - 1],
                                // sin normal: en una esfera centrada la normal es la posición normalizada
                                _ => p.normalize(),
                            };
                            Vertex { position: p, normal: n }
                        })
                        .collect();

                    // caras de más de 3 vértices se dividen en triángulos desde el primer vértice
                    for i in 1..corners.len().saturating_sub(1) {
                        vertices.push(corners[0]);
                        vertices.push(corners[i]);
                        vertices.push(corners[i + 1]);
                    }
                }
                _ => {} // se ignoran comentarios, texturas y materiales
            }
        }

        Ok(Obj { vertices })
    }
}

// Lee tres floats de la línea; si alguno falta o no se puede leer, usa 0
fn parse_vec3<'a>(parts: &mut impl Iterator<Item = &'a str>) -> Vec3 {
    let mut v = [0.0f32; 3];
    for x in v.iter_mut() {
        *x = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    }
    Vec3::new(v[0], v[1], v[2])
}
