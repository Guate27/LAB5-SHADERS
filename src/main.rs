// Lab 5 - Shaders: sistema solar renderizado solo con shaders
//
// Controles:
//   1 Helios   2 Gaia-7 + Selene   3 Zephyros + anillos
//   4 Vulkara  5 Glacius           6 Xyloth + anillos
//   0 Sistema solar con rotación y traslación
//   Espacio: pausa   Arriba/Abajo: zoom   P: captura   Esc: salir
//
// Capturas del README sin abrir ventana:
//   cargo run --release -- --capturas
mod framebuffer;
mod math;
mod noise;
mod obj;
mod shaders;
mod triangle;

use framebuffer::Framebuffer;
use math::{Mat4, Vec3};
use minifb::{Key, KeyRepeat, Window, WindowOptions};
use obj::Obj;
use shaders::{fragment_shader, vertex_shader, ShaderKind, Uniforms};
use std::time::Instant;
use triangle::draw_triangle;

const WIDTH: usize = 800;
const HEIGHT: usize = 600;

#[derive(Clone, Copy, PartialEq)]
enum Malla {
    Esfera, // sphere.obj
    Anillo, // ring.obj
}

// Objeto de la escena: modelo, shader y transformación
struct Cuerpo {
    malla: Malla,
    shader: ShaderKind,
    posicion: Vec3,
    escala: f32,
    giro: f32,        // rotación sobre su eje (radianes)
    inclinacion: f32, // inclinación del eje (radianes)
}

struct Escena {
    nombre: &'static str,
    cuerpos: Vec<Cuerpo>,
    camara: Vec3, // siempre mira al origen
    sol: Vec3,    // posición de la luz
}

fn esfera(shader: ShaderKind, posicion: Vec3, escala: f32, giro: f32, inclinacion: f32) -> Cuerpo {
    Cuerpo { malla: Malla::Esfera, shader, posicion, escala, giro, inclinacion }
}

// Anillos: modelo aparte con la misma posición, escala e inclinación del planeta
fn anillos(shader: ShaderKind, planeta: &Cuerpo) -> Cuerpo {
    Cuerpo {
        malla: Malla::Anillo,
        shader,
        posicion: planeta.posicion,
        escala: planeta.escala,
        giro: 0.0,
        inclinacion: planeta.inclinacion,
    }
}

// Posición en una órbita circular; altura inclina un poco la órbita
fn orbita(centro: Vec3, radio: f32, angulo: f32, altura: f32) -> Vec3 {
    centro + Vec3::new(angulo.cos() * radio, angulo.sin() * altura, angulo.sin() * radio)
}

// Arma la escena n en el tiempo t (segundos)
fn escena(n: u8, t: f32) -> Escena {
    // luz fija para las vistas de un solo planeta
    let sol_lejano = Vec3::new(-40.0, 15.0, 30.0);
    let origen = Vec3::new(0.0, 0.0, 0.0);
    match n {
        1 => Escena {
            nombre: "Helios (estrella)",
            cuerpos: vec![esfera(ShaderKind::Sol, origen, 1.6, t * 0.05, 0.0)],
            camara: Vec3::new(0.0, 0.0, 5.2),
            sol: Vec3::new(0.0, 0.0, 0.0),
        },
        2 => {
            let planeta = esfera(ShaderKind::Rocoso, origen, 1.3, t * 0.2, 0.41);
            let luna = esfera(ShaderKind::Luna, orbita(origen, 2.5, t * 0.45, 0.4), 0.32, t * 0.45, 0.0);
            Escena {
                nombre: "Gaia-7 (rocoso) + luna Selene",
                cuerpos: vec![planeta, luna],
                camara: Vec3::new(0.0, 0.6, 6.5),
                sol: sol_lejano,
            }
        }
        3 => {
            let planeta = esfera(ShaderKind::Gaseoso, origen, 1.1, t * 0.3, 0.45);
            let aro = anillos(ShaderKind::Anillos, &planeta);
            Escena {
                nombre: "Zephyros (gigante gaseoso) + anillos",
                cuerpos: vec![planeta, aro],
                camara: Vec3::new(0.0, 1.8, 6.8),
                sol: sol_lejano,
            }
        }
        4 => Escena {
            nombre: "Vulkara (planeta de lava)",
            cuerpos: vec![esfera(ShaderKind::Lava, origen, 1.4, t * 0.15, 0.2)],
            camara: Vec3::new(0.0, 0.3, 5.2),
            sol: sol_lejano,
        },
        5 => Escena {
            nombre: "Glacius (planeta helado)",
            cuerpos: vec![esfera(ShaderKind::Hielo, origen, 1.4, t * 0.15, 0.15)],
            camara: Vec3::new(0.0, 0.3, 5.2),
            sol: sol_lejano,
        },
        6 => {
            let planeta = esfera(ShaderKind::Alienigena, origen, 1.1, t * 0.25, -0.15);
            let aro = anillos(ShaderKind::AnillosAlien, &planeta);
            Escena {
                nombre: "Xyloth (gigante toxico) + anillos",
                cuerpos: vec![planeta, aro],
                camara: Vec3::new(0.0, 3.6, 5.8), // desde arriba para ver el hexágono
                sol: sol_lejano,
            }
        }
        _ => {
            // Sistema solar: rotación (giro) y traslación (orbita); los lejanos orbitan más lento
            let vulkara = esfera(ShaderKind::Lava, orbita(origen, 2.7, t * 0.5 + 1.0, 0.0), 0.35, t * 0.6, 0.2);
            let gaia = esfera(ShaderKind::Rocoso, orbita(origen, 4.0, t * 0.32 + 4.0, 0.0), 0.5, t * 0.8, 0.41);
            // la luna orbita alrededor de Gaia
            let selene = esfera(ShaderKind::Luna, orbita(gaia.posicion, 0.9, t * 1.6, 0.2), 0.13, t, 0.0);
            let glacius = esfera(ShaderKind::Hielo, orbita(origen, 5.4, t * 0.22 + 2.5, 0.0), 0.4, t * 0.5, 0.1);
            let zephyros = esfera(ShaderKind::Gaseoso, orbita(origen, 7.2, t * 0.14 + 5.5, 0.0), 0.85, t * 0.7, 0.45);
            let xyloth = esfera(ShaderKind::Alienigena, orbita(origen, 9.6, t * 0.1 + 3.6, 0.0), 0.7, t * 0.6, -0.35);
            let aro_z = anillos(ShaderKind::Anillos, &zephyros);
            let aro_x = anillos(ShaderKind::AnillosAlien, &xyloth);
            Escena {
                nombre: "Sistema solar",
                cuerpos: vec![
                    esfera(ShaderKind::Sol, origen, 1.5, t * 0.05, 0.0),
                    vulkara, gaia, selene, glacius, zephyros, aro_z, xyloth, aro_x,
                ],
                camara: Vec3::new(0.0, 9.0, 17.0),
                sol: origen, // la luz sale del sol
            }
        }
    }
}

// Pipeline de un cuerpo: vertex shader -> rasterización -> fragment shader
fn render_cuerpo(fb: &mut Framebuffer, modelo: &Obj, c: &Cuerpo, view_projection: &Mat4, camara: Vec3, sol: Vec3, t: f32) {
    // matriz de modelo: escala -> giro -> inclinación -> traslación (se aplica de derecha a izquierda)
    let model = Mat4::translation(c.posicion)
        .mul(&Mat4::rotation_z(c.inclinacion))
        .mul(&Mat4::rotation_y(c.giro))
        .mul(&Mat4::scale(c.escala));

    let u = Uniforms {
        model,
        view_projection: *view_projection,
        width: fb.width as f32,
        height: fb.height as f32,
        time: t,
        sun_pos: sol,
        camera_pos: camara,
        body_center: c.posicion,
        body_radius: c.escala, // la esfera tiene radio 1
    };

    // vertex shader
    let transformados: Vec<_> = modelo.vertices.iter().map(|v| vertex_shader(v, &u)).collect();

    // backface culling solo en esferas; los anillos se ven por las dos caras
    let culling = c.malla == Malla::Esfera;

    // rasterización y fragment shader por triángulo
    for tri in transformados.chunks(3) {
        // se omite el triángulo si algún vértice quedó detrás de la cámara
        if let [Some(a), Some(b), Some(cc)] = tri {
            draw_triangle(fb, [a, b, cc], culling, |frag| fragment_shader(c.shader, frag, &u));
        }
    }
}

fn render_escena(fb: &mut Framebuffer, esfera: &Obj, anillo: &Obj, e: &Escena, zoom: f32, t: f32) {
    fb.clear();
    fb.draw_stars(450);

    let camara = e.camara * zoom;
    let view = Mat4::look_at(camara, Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
    let projection = Mat4::perspective(45f32.to_radians(), fb.width as f32 / fb.height as f32, 0.1, 100.0);
    // se multiplican una vez por frame en lugar de por vértice
    let view_projection = projection.mul(&view);

    // el orden no importa, el z-buffer resuelve qué queda al frente
    for c in &e.cuerpos {
        let modelo = match c.malla {
            Malla::Esfera => esfera,
            Malla::Anillo => anillo,
        };
        render_cuerpo(fb, modelo, c, &view_projection, camara, e.sol, t);
    }
}

fn main() {
    let esfera = Obj::load("assets/sphere.obj").expect("Falta assets/sphere.obj");
    let anillo = Obj::load("assets/ring.obj").expect("Falta assets/ring.obj");

    let mut fb = Framebuffer::new(WIDTH, HEIGHT);
    fb.set_background_color(0x05060B);

    // modo --capturas: guarda una imagen de cada escena y termina
    if std::env::args().any(|a| a == "--capturas") {
        std::fs::create_dir_all("capturas").expect("No se pudo crear la carpeta capturas");
        let nombres = ["0_sistema_solar", "1_helios", "2_gaia7_selene", "3_zephyros_anillos", "4_vulkara", "5_glacius", "6_xyloth_anillos"];
        for (i, nombre) in nombres.iter().enumerate() {
            // tiempo fijo para que las capturas sean siempre iguales
            let e = escena(i as u8, 6.0);
            render_escena(&mut fb, &esfera, &anillo, &e, 1.0, 6.0);
            let ruta = format!("capturas/{nombre}.bmp");
            fb.save_bmp(&ruta).expect("No se pudo guardar la captura");
            println!("Guardada {ruta}");
        }
        return;
    }

    let mut window = Window::new("Lab 5 - Shaders", WIDTH, HEIGHT, WindowOptions::default())
        .expect("No se pudo crear la ventana");
    window.set_target_fps(60);

    let mut actual: u8 = 0; // escena actual
    let mut zoom = 1.0f32;
    let mut pausa = false;
    let mut t = 0.0f32; // tiempo de la animación
    let mut ultimo = Instant::now();
    let teclas = [Key::Key0, Key::Key1, Key::Key2, Key::Key3, Key::Key4, Key::Key5, Key::Key6];

    while window.is_open() && !window.is_key_down(Key::Escape) {
        // dt en segundos, para que la animación no dependa de los FPS
        let dt = ultimo.elapsed().as_secs_f32();
        ultimo = Instant::now();
        if !pausa {
            t += dt;
        }

        // teclas 0-6 cambian de escena
        for (i, k) in teclas.iter().enumerate() {
            if window.is_key_pressed(*k, KeyRepeat::No) {
                actual = i as u8;
            }
        }
        if window.is_key_pressed(Key::Space, KeyRepeat::No) {
            pausa = !pausa;
        }
        // zoom con límites
        if window.is_key_down(Key::Up) {
            zoom = (zoom - 0.6 * dt).max(0.4);
        }
        if window.is_key_down(Key::Down) {
            zoom = (zoom + 0.6 * dt).min(2.5);
        }

        let e = escena(actual, t);
        render_escena(&mut fb, &esfera, &anillo, &e, zoom, t);

        // P guarda una captura
        if window.is_key_pressed(Key::P, KeyRepeat::No) {
            let ruta = format!("captura_{actual}.bmp");
            if fb.save_bmp(&ruta).is_ok() {
                println!("Captura guardada en {ruta}");
            }
        }

        let fps = if dt > 0.0 { 1.0 / dt } else { 0.0 };
        window.set_title(&format!("Lab 5 - Shaders | {} | {:.0} FPS | teclas 0-6", e.nombre, fps));
        window
            .update_with_buffer(&fb.buffer, WIDTH, HEIGHT)
            .expect("No se pudo actualizar la ventana");
    }
}
