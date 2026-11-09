// Vertex shader y fragment shaders de cada cuerpo celeste.
// Todo el color se calcula aquí, sin texturas ni materiales.
use crate::math::{clamp, mix, rgb, smoothstep, Mat4, Vec3};
use crate::noise::{cellular, fbm, noise, ridged};
use crate::obj::Vertex;
use crate::triangle::{Fragment, ScreenVertex};
use std::f32::consts::PI;

// Valores que comparten todos los vértices y fragmentos de un objeto en un frame
pub struct Uniforms {
    pub model: Mat4,
    pub view_projection: Mat4,
    pub width: f32,
    pub height: f32,
    pub time: f32,
    pub sun_pos: Vec3,     // posición de la luz
    pub camera_pos: Vec3,
    pub body_center: Vec3, // centro del planeta (para la sombra sobre los anillos)
    pub body_radius: f32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ShaderKind {
    Sol,
    Rocoso,
    Gaseoso,
    Anillos,
    Luna,
    Lava,
    Hielo,
    Alienigena,
    AnillosAlien,
}

// ===================== VERTEX SHADER =====================
pub fn vertex_shader(v: &Vertex, u: &Uniforms) -> Option<ScreenVertex> {
    // modelo -> mundo
    let w = u.model.transform_point(v.position);
    let world = Vec3::new(w[0], w[1], w[2]);
    // la normal se rota con el modelo y se vuelve a normalizar por la escala
    let normal = u.model.transform_dir(v.normal).normalize();

    // mundo -> clip
    let clip = u.view_projection.transform_point(world);
    if clip[3] < 0.1 {
        return None; // detrás de la cámara
    }
    // división de perspectiva -> NDC (-1 a 1)
    let ndc = Vec3::new(clip[0] / clip[3], clip[1] / clip[3], clip[2] / clip[3]);

    // NDC -> pantalla (y invertida porque en pantalla crece hacia abajo)
    let screen = Vec3::new(
        (ndc.x + 1.0) * 0.5 * u.width,
        (1.0 - ndc.y) * 0.5 * u.height,
        ndc.z,
    );

    Some(ScreenVertex { screen, world, normal, local: v.position })
}

// ===================== FRAGMENT SHADER =====================
pub fn fragment_shader(kind: ShaderKind, f: &Fragment, u: &Uniforms) -> Option<Vec3> {
    match kind {
        ShaderKind::Sol => Some(sol(f, u)),
        ShaderKind::Rocoso => Some(rocoso(f, u)),
        ShaderKind::Gaseoso => Some(gaseoso(f, u)),
        ShaderKind::Luna => Some(luna(f, u)),
        ShaderKind::Lava => Some(lava(f, u)),
        ShaderKind::Hielo => Some(hielo(f, u)),
        ShaderKind::Alienigena => Some(alienigena(f, u)),
        // mismo shader de anillos con dos paletas (interior, exterior, bandas claras)
        ShaderKind::Anillos => anillos(f, u, rgb(0x8A765A), rgb(0xE9D6AA), rgb(0xC8A97A)),
        ShaderKind::AnillosAlien => anillos(f, u, rgb(0x2B1B5A), rgb(0x7FE6C8), rgb(0xA77CF2)),
    }
}

// ---------- Funciones de iluminación compartidas ----------

struct Luz {
    ndl: f32,  // normal · luz (negativo = lado nocturno)
    diff: f32, // difusa, ndl recortado a 0
    ndv: f32,  // normal · vista (1 en el centro, 0 en el borde)
    n: Vec3,
    l: Vec3,
    v: Vec3,
}

fn luz(f: &Fragment, u: &Uniforms) -> Luz {
    let n = f.normal;
    let l = (u.sun_pos - f.world).normalize();
    let v = (u.camera_pos - f.world).normalize();
    let ndl = n.dot(l);
    Luz { ndl, diff: ndl.max(0.0), ndv: n.dot(v).max(0.0), n, l, v }
}

// Especular Blinn-Phong, apagado en el lado nocturno
fn especular(lz: &Luz, dureza: f32) -> f32 {
    let h = (lz.l + lz.v).normalize();
    lz.n.dot(h).max(0.0).powf(dureza) * smoothstep(0.0, 0.1, lz.ndl)
}

// Fresnel: 0 en el centro y 1 en el borde, se usa para la atmósfera
fn fresnel(lz: &Luz, potencia: f32) -> f32 {
    (1.0 - lz.ndv).powf(potencia)
}

// Rota un punto en Y (para mover nubes y gases a otra velocidad que la superficie)
fn rot_y(p: Vec3, a: f32) -> Vec3 {
    let (s, c) = a.sin_cos();
    Vec3::new(p.x * c + p.z * s, p.y, -p.x * s + p.z * c)
}

// =============================================================
// 1) ESTRELLA "HELIOS"
// No usa luz difusa porque es la fuente de luz
// =============================================================
fn sol(f: &Fragment, u: &Uniforms) -> Vec3 {
    let p = f.local.normalize();
    let t = u.time;
    let lz = luz(f, u);

    // Capa 1: color base naranja/amarillo con fbm animado
    let temp = fbm(p * 2.2 + Vec3::new(t * 0.06, 0.0, t * 0.04), 4);
    let mut col = mix(rgb(0xD9420B), rgb(0xFFB423), smoothstep(0.3, 0.7, temp));

    // Capa 2: granulación con ruido celular (centro de la celda claro, borde oscuro)
    let (d, _) = cellular(p * 16.0 + Vec3::new(0.0, t * 0.25, 0.0));
    col = col * (1.18 - 0.4 * smoothstep(0.1, 0.75, d));

    // Capa 3: manchas solares con penumbra y umbra
    let s = fbm(p * 3.0 + Vec3::new(40.0, t * 0.01, 0.0), 4);
    let penumbra = smoothstep(0.64, 0.68, s);
    let umbra = smoothstep(0.69, 0.72, s);
    col = mix(col, rgb(0x8A3208), penumbra * 0.7);
    col = mix(col, rgb(0x2E0D02), umbra * 0.85);

    // Capa 4: filamentos brillantes con ruido de crestas (no sobre las manchas)
    let r = ridged(p * 4.0 + Vec3::new(t * 0.08, -t * 0.05, 0.0), 4);
    col += rgb(0xFFE9A8) * (smoothstep(0.86, 0.98, r) * 0.55 * (1.0 - umbra));

    // Capa 5: borde más oscuro y rojizo, centro más claro
    let mu = lz.ndv;
    col = mix(rgb(0x9E2A05) * 0.8, col, smoothstep(0.0, 0.45, mu));
    col = col * (0.55 + 0.45 * mu.sqrt()) + rgb(0xFFF6D8) * (mu.powi(8) * 0.4);
    col
}

// =============================================================
// 2) PLANETA ROCOSO "GAIA-7" (basado en la Tierra)
// =============================================================
fn rocoso(f: &Fragment, u: &Uniforms) -> Vec3 {
    let p = f.local.normalize();
    let t = u.time;
    let lz = luz(f, u);
    let lat = p.y.abs(); // 0 en el ecuador, 1 en los polos

    // Capa 1: océano, más oscuro en zonas profundas
    let prof = fbm(p * 3.0 + Vec3::new(5.0, 1.0, 2.0), 4);
    let oceano = mix(rgb(0x061C52), rgb(0x1C6DB0), smoothstep(0.35, 0.65, prof));

    // Capa 2: continentes. El fbm funciona como mapa de alturas
    let h = fbm(p * 1.7 + Vec3::new(11.0, 3.0, 7.0), 6);
    let nivel_mar = 0.52;
    let es_tierra = smoothstep(nivel_mar, nivel_mar + 0.008, h);
    let elev = clamp((h - nivel_mar) / 0.2, 0.0, 1.0); // altura sobre el mar (0 a 1)
    let verde = mix(rgb(0x2C6324), rgb(0x6A9440), fbm(p * 9.0, 3));
    // desiertos solo cerca del ecuador
    let desierto_t = smoothstep(0.4, 0.15, lat) * smoothstep(0.48, 0.58, fbm(p * 3.5 + Vec3::new(20.0, 0.0, 0.0), 4));
    let mut tierra = mix(verde, rgb(0xC8A46A), desierto_t);
    tierra = mix(rgb(0xD9C690), tierra, smoothstep(0.0, 0.06, elev)); // playa
    tierra = mix(tierra, rgb(0x6E5A48), smoothstep(0.35, 0.65, elev)); // montañas
    tierra = mix(tierra, rgb(0xF0F0F0), smoothstep(0.75, 0.9, elev)); // nieve
    let mut col = mix(oceano, tierra, es_tierra);

    // Capa 3: casquetes polares, con ruido para que el borde sea irregular
    let hielo = smoothstep(0.78, 0.84, lat + (fbm(p * 6.0, 4) - 0.5) * 0.25);
    col = mix(col, rgb(0xEEF5FA), hielo);

    // Capa 4: luz difusa + reflejo especular solo en el agua
    let agua = (1.0 - es_tierra) * (1.0 - hielo);
    let mut lit = col * (0.03 + 0.97 * lz.diff) + Vec3::new(1.0, 0.95, 0.85) * (especular(&lz, 60.0) * 0.6 * agua);

    // Capa 5: luces de ciudades en el lado nocturno (solo tierra baja sin hielo)
    let noche = smoothstep(0.05, -0.2, lz.ndl);
    let ciudades = es_tierra * (1.0 - hielo) * (1.0 - smoothstep(0.3, 0.5, elev))
        * smoothstep(0.6, 0.68, fbm(p * 28.0, 3));
    lit += rgb(0xFFC25A) * (ciudades * noche * 0.9);

    // Capa 6: nubes. Se rotan con el tiempo y se deforman con otro fbm
    let cp = rot_y(p, t * 0.08);
    let warp = fbm(cp * 2.0, 3);
    let nubes = smoothstep(0.52, 0.72, fbm(cp * 3.2 + Vec3::new(warp * 2.0, 0.0, 0.0), 5)) * 0.9;
    let luz_nube = 0.05 + 0.95 * smoothstep(-0.1, 0.4, lz.ndl);
    lit = mix(lit, Vec3::new(1.0, 1.0, 1.0) * luz_nube, nubes);

    // Capa 7: atmósfera azul en el borde, del lado iluminado
    lit += rgb(0x4F9CFF) * (fresnel(&lz, 2.5) * smoothstep(-0.3, 0.4, lz.ndl) * 0.85);
    lit
}

// =============================================================
// 3) GIGANTE GASEOSO "ZEPHYROS" (basado en Júpiter)
// =============================================================
fn gaseoso(f: &Fragment, u: &Uniforms) -> Vec3 {
    let p = f.local.normalize();
    let t = u.time;
    let lz = luz(f, u);

    // Capa 1: bandas con seno de la latitud, deformada con fbm para que se ondulen
    let warp = fbm(p * 3.0 + Vec3::new(t * 0.02, 0.0, 0.0), 4);
    let lat = p.y + (warp - 0.5) * 0.18;
    let b_fina = 0.5 + 0.5 * (lat * 22.0).sin();
    let b_ancha = 0.5 + 0.5 * (lat * 7.0 + 1.3).sin();
    let mut col = mix(rgb(0x8F4E30), rgb(0xC9976A), b_ancha);
    col = mix(col, rgb(0xF0E2C4), smoothstep(0.35, 0.8, b_fina) * 0.85);

    // Capa 2: turbulencia, ruido estirado en x/z para que siga las bandas
    let turb = fbm(Vec3::new(p.x * 3.0 + t * 0.04, p.y * 32.0, p.z * 3.0), 4);
    col = col * (0.82 + 0.36 * turb);

    // Capa 3: Gran Mancha (elipse con espiral animada)
    let lon = p.z.atan2(p.x);
    let mut dlon = lon + 2.6;
    // mantener la diferencia de longitud entre -PI y PI
    if dlon > PI { dlon -= 2.0 * PI; }
    if dlon < -PI { dlon += 2.0 * PI; }
    let (dx, dy) = (dlon * 0.94 / 0.34, (p.y - 0.22) / 0.12);
    let d = (dx * dx + dy * dy).sqrt(); // 0 en el centro, 1 en el borde de la elipse
    let giro = 0.5 + 0.5 * (d * 9.0 - dy.atan2(dx) - t * 1.5).sin();
    let mancha = mix(rgb(0xA8361E), rgb(0xE07A4E), giro * (1.0 - d));
    let halo = smoothstep(0.75, 0.95, d) - smoothstep(1.0, 1.3, d);
    col = mix(col, rgb(0xF5E6CC), halo * 0.55);
    col = mix(col, mancha, 1.0 - smoothstep(0.8, 1.0, d));

    // Capa 4: óvalos blancos en algunas celdas, lejos de los polos
    let (dc, id) = cellular(Vec3::new(p.x * 4.0, p.y * 14.0, p.z * 4.0));
    let ovalo = (1.0 - smoothstep(0.12, 0.2, dc)) * smoothstep(0.82, 0.86, id) * smoothstep(0.85, 0.6, p.y.abs());
    col = mix(col, rgb(0xFAF4E6), ovalo * 0.9);

    // Capa 5: luz difusa, oscurecimiento del borde y bruma en la orilla
    let limbo = 0.65 + 0.35 * lz.ndv.sqrt();
    let mut lit = col * ((0.03 + 0.97 * lz.diff) * limbo);
    lit += rgb(0xE8C9A0) * (fresnel(&lz, 3.0) * smoothstep(-0.2, 0.5, lz.ndl) * 0.35);
    lit
}

// =============================================================
// ANILLOS (modelo separado: disco plano en XZ)
// =============================================================
fn anillos(f: &Fragment, u: &Uniforms, c_int: Vec3, c_ext: Vec3, c_brillo: Vec3) -> Option<Vec3> {
    let p = f.local;
    let r = (p.x * p.x + p.z * p.z).sqrt();
    let rn = clamp((r - 1.35) / (2.4 - 1.35), 0.0, 1.0); // 0 = borde interno, 1 = borde externo

    // Capa 1: divisiones vacías, el fragmento se descarta
    let cassini = rn > 0.60 && rn < 0.665;
    let encke = rn > 0.90 && rn < 0.915;
    if cassini || encke {
        return None;
    }

    // Capa 2: color según el radio + bandas finas con ruido 1D sobre el radio
    let mut col = mix(c_int, c_ext, smoothstep(0.0, 0.55, rn));
    let bandas = fbm(Vec3::new(r * 38.0, 0.5, 0.5), 4);
    col = col * (0.6 + 0.7 * bandas);
    col = mix(col, c_brillo, smoothstep(0.62, 0.75, bandas) * 0.5);

    // Capa 3: densidad (interior tenue, borde externo se desvanece) y granulado
    let densidad = smoothstep(0.0, 0.18, rn) * (1.0 - smoothstep(0.96, 1.0, rn));
    col = col * (0.35 + 0.65 * densidad);
    col = col * (0.85 + 0.3 * noise(p * 90.0));

    // Capa 4: sombra del planeta. Intersección rayo-esfera desde el punto hacia el sol
    let dir = (u.sun_pos - f.world).normalize();
    let oc = f.world - u.body_center;
    let b = oc.dot(dir);
    let c = oc.dot(oc) - u.body_radius * u.body_radius;
    let disc = b * b - c;
    let sombra = if disc > 0.0 && -b - disc.sqrt() > 0.0 { 0.15 } else { 1.0 };

    // Capa 5: luz por ambas caras del disco (valor absoluto)
    let ndl = f.normal.dot(dir).abs();
    Some(col * ((0.08 + 0.92 * (0.3 + 0.7 * ndl)) * sombra))
}

// =============================================================
// LUNA "SELENE"
// =============================================================
fn luna(f: &Fragment, u: &Uniforms) -> Vec3 {
    let p = f.local.normalize();
    let lz = luz(f, u);

    // Capa 1: superficie gris con variación
    let mut col = mix(rgb(0x6A6661), rgb(0xBDB8B0), fbm(p * 4.0, 5));

    // Capa 2: mares (zonas oscuras grandes)
    let mares = smoothstep(0.55, 0.62, fbm(p * 1.6 + Vec3::new(3.0, 8.0, 1.0), 4));
    col = mix(col, rgb(0x46433F), mares * 0.8);

    // Capa 3: cráteres grandes y pequeños con ruido celular (fondo oscuro, borde claro)
    for (escala, fuerza) in [(5.0f32, 1.0f32), (13.0, 0.7)] {
        let (d, id) = cellular(p * escala);
        let radio = 0.18 + 0.22 * id;
        let existe = smoothstep(0.3, 0.35, id); // no todas las celdas tienen cráter
        let fondo = 1.0 - smoothstep(radio * 0.6, radio, d);
        let borde = smoothstep(radio * 0.8, radio, d) - smoothstep(radio, radio * 1.25, d);
        col = mix(col, col * 0.62, fondo * existe * fuerza);
        col = mix(col, rgb(0xDEDAD2), borde * existe * fuerza * 0.6);
    }

    // Capa 4: luz sin atmósfera, la noche queda casi negra
    col * (0.015 + 0.985 * lz.diff.powf(0.8))
}

// =============================================================
// EXTRA 1: PLANETA DE LAVA "VULKARA"
// =============================================================
fn lava(f: &Fragment, u: &Uniforms) -> Vec3 {
    let p = f.local.normalize();
    let t = u.time;
    let lz = luz(f, u);

    // Capa 1: corteza oscura
    let corteza = mix(rgb(0x17110F), rgb(0x4D3B33), fbm(p * 5.0, 5));

    // Capa 2: grietas (ruido de crestas deformado) y lagos de magma
    let w = fbm(p * 2.0 + Vec3::new(0.0, t * 0.03, 0.0), 3);
    let r = ridged(p * 2.6 + Vec3::new(w, w, w), 5);
    let grieta = smoothstep(0.86, 0.96, r);
    let lago = smoothstep(0.64, 0.70, fbm(p * 1.4 + Vec3::new(9.0, 2.0, 5.0), 4));
    let magma = grieta.max(lago);

    // Capa 3: la temperatura de la lava sube y baja con el tiempo
    let pulso = 0.7 + 0.3 * (t * 2.2 + fbm(p * 4.0, 3) * 12.0).sin();
    let col_lava = mix(rgb(0x9E1500), rgb(0xFFD447), magma * pulso);

    // Capa 4: la corteza usa la luz del sol, la lava es emisiva (se ve de noche)
    let mut col = corteza * (0.04 + 0.96 * lz.diff);
    col += rgb(0xFF3C00) * (smoothstep(0.7, 0.9, r) * 0.22); // brillo alrededor de las grietas
    col = mix(col, col_lava, magma);

    // Capa 5: brillo naranja en el borde
    col += rgb(0xFF5A1A) * (fresnel(&lz, 2.2) * 0.55);
    col
}

// =============================================================
// EXTRA 2: PLANETA HELADO "GLACIUS" (basado en Europa)
// =============================================================
fn hielo(f: &Fragment, u: &Uniforms) -> Vec3 {
    let p = f.local.normalize();
    let lz = luz(f, u);

    // Capa 1: hielo base blanco/celeste
    let mut col = mix(rgb(0xB9D3E4), rgb(0xF5FBFF), fbm(p * 3.0, 5));

    // Capa 2: zonas de hielo azul más oscuro
    let caos = smoothstep(0.6, 0.67, fbm(p * 2.2 + Vec3::new(4.0, 7.0, 1.0), 4));
    col = mix(col, rgb(0x4A7BA3) * (0.8 + 0.4 * fbm(p * 12.0, 3)), caos * 0.75);

    // Capa 3: grietas largas rojizas y grietas finas azules
    let l1 = smoothstep(0.92, 0.985, ridged(p * 2.5 + Vec3::new(1.0, 0.0, 3.0), 4));
    let l2 = smoothstep(0.94, 0.99, ridged(p * 6.5 + Vec3::new(8.0, 2.0, 0.0), 3));
    col = mix(col, rgb(0x8C4F3A), l1 * 0.85);
    col = mix(col, rgb(0x3E77A8), l2 * 0.6);

    // Capa 4: luz difusa + especular con ruido fino para que brille en puntos
    let destello = especular(&lz, 25.0) * (0.4 + 0.6 * noise(p * 60.0));
    let mut lit = col * (0.03 + 0.97 * lz.diff) + Vec3::new(0.85, 0.95, 1.0) * (destello * 0.5);

    // Capa 5: atmósfera celeste en el borde
    lit += rgb(0x9FE7FF) * (fresnel(&lz, 3.0) * smoothstep(-0.2, 0.5, lz.ndl) * 0.4);
    lit
}

// =============================================================
// EXTRA 3: GIGANTE TÓXICO "XYLOTH" (inventado)
// =============================================================
fn alienigena(f: &Fragment, u: &Uniforms) -> Vec3 {
    let p = f.local.normalize();
    let t = u.time;
    let lz = luz(f, u);

    // Capa 1: bandas violeta/turquesa con más deformación que Zephyros y animadas
    let q = rot_y(p, t * 0.05);
    let warp = fbm(q * 2.5, 4);
    let lat = p.y + (warp - 0.5) * 0.35;
    let b = 0.5 + 0.5 * (lat * 12.0).sin();
    let b2 = 0.5 + 0.5 * (lat * 31.0 + warp * 4.0).sin();
    let mut col = mix(rgb(0x2A1257), rgb(0x179C86), smoothstep(0.2, 0.8, b));
    col = mix(col, rgb(0xB6F25A), smoothstep(0.75, 0.95, b2) * 0.6);

    // Capa 2: hexágono en el polo norte (como el de Saturno)
    if p.y > 0.0 {
        let ang = p.z.atan2(p.x) + t * 0.1;
        let rr = (p.x * p.x + p.z * p.z).sqrt(); // distancia al eje
        // distancia hexagonal: se divide el ángulo en 6 sectores de 60°
        let sector = ang.rem_euclid(PI / 3.0) - PI / 6.0;
        let hex = rr * sector.cos() / (PI / 6.0).cos();
        let dentro = 1.0 - smoothstep(0.42, 0.44, hex);
        let borde = smoothstep(0.40, 0.43, hex) - smoothstep(0.44, 0.47, hex);
        col = mix(col, rgb(0x1B0B3A), dentro * 0.8);
        col = mix(col, rgb(0xC9FF7A), borde * 0.9);
    }

    // Capa 3: luz difusa
    let mut lit = col * (0.04 + 0.96 * lz.diff);

    // Capa 4: relámpagos en el lado nocturno; cada celda parpadea en un momento distinto
    let (d, id) = cellular(p * 7.0);
    let parpadeo = smoothstep(0.85, 1.0, (t * 3.0 + id * 40.0).sin());
    let rayo = (1.0 - smoothstep(0.0, 0.3, d)) * parpadeo;
    let noche = smoothstep(0.1, -0.2, lz.ndl);
    lit += rgb(0x7CFFB2) * (rayo * noche * 0.9);

    // Capa 5: atmósfera verde en el borde
    lit += rgb(0x6BFF9E) * (fresnel(&lz, 2.5) * (0.25 + 0.75 * smoothstep(-0.3, 0.4, lz.ndl)) * 0.6);
    lit
}
