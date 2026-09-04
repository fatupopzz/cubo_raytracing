mod cube;
mod framebuffer;
mod material;
mod ray_intersect;
mod texture;

use cube::Cube;
use framebuffer::Framebuffer;
use material::Material;
use nalgebra_glm::{cross, dot, normalize, Vec3};
use ray_intersect::{Intersect, Ray, RayIntersect};
use raylib::prelude::*;
use std::f32::consts::PI;

const ANCHO: usize = 800;
const ALTO: usize = 600;

/// Campo de vision vertical.
const FOV: f32 = PI / 3.0;

/// Piso de luz que le llega a toda cara, mire a donde mire. Sin esto las
/// caras que no ven la luz quedan negras y el cubo pierde el volumen.
const AMBIENTE: f32 = 0.15;

/// Colores del degradado de fondo, de arriba hacia abajo. Un fondo plano
/// negro se comeria la silueta del cubo.
const FONDO_ARRIBA: Vec3 = Vec3::new(0.05, 0.07, 0.15);
const FONDO_ABAJO: Vec3 = Vec3::new(0.45, 0.52, 0.62);

// ---------- Luz ----------

/// Luz puntual: irradia desde un punto en todas direcciones.
struct Luz {
    posicion: Vec3,
    color: Vec3,
    intensidad: f32,
}

// ---------- Camara ----------

/// El cubo es un AABB y no rota, asi que la que se mueve es la camara:
/// orbita alrededor del objetivo con dos angulos y un radio.
struct Camara {
    objetivo: Vec3,
    yaw: f32,
    pitch: f32,
    radio: f32,
}

impl Camara {
    fn new(objetivo: Vec3, yaw: f32, pitch: f32, radio: f32) -> Self {
        Camara {
            objetivo,
            yaw,
            pitch,
            radio,
        }
    }

    fn posicion(&self) -> Vec3 {
        Vec3::new(
            self.objetivo.x + self.radio * self.pitch.cos() * self.yaw.sin(),
            self.objetivo.y + self.radio * self.pitch.sin(),
            self.objetivo.z + self.radio * self.pitch.cos() * self.yaw.cos(),
        )
    }

    /// Base ortonormal de la camara: (derecha, arriba, adelante). Se
    /// recalcula cada cuadro porque los controles mueven la orbita.
    fn base(&self) -> (Vec3, Vec3, Vec3) {
        let adelante = normalize(&(self.objetivo - self.posicion()));
        // El pitch esta topado antes de llegar al polo, asi que este cross
        // nunca degenera.
        let derecha = normalize(&cross(&adelante, &Vec3::new(0.0, 1.0, 0.0)));
        let arriba = cross(&derecha, &adelante);

        (derecha, arriba, adelante)
    }

    /// Lee el teclado y mueve la orbita. Devuelve true si algo cambio, para
    /// no volver a trazar la escena entera cuando nadie toco nada.
    fn actualizar(&mut self, rl: &RaylibHandle, delta: f32) -> bool {
        let velocidad_giro = 1.4 * delta;
        let velocidad_zoom = 4.0 * delta;
        let mut movio = false;

        if rl.is_key_down(KeyboardKey::KEY_LEFT) {
            self.yaw -= velocidad_giro;
            movio = true;
        }
        if rl.is_key_down(KeyboardKey::KEY_RIGHT) {
            self.yaw += velocidad_giro;
            movio = true;
        }
        if rl.is_key_down(KeyboardKey::KEY_UP) {
            self.pitch += velocidad_giro;
            movio = true;
        }
        if rl.is_key_down(KeyboardKey::KEY_DOWN) {
            self.pitch -= velocidad_giro;
            movio = true;
        }
        if rl.is_key_down(KeyboardKey::KEY_W) {
            self.radio -= velocidad_zoom;
            movio = true;
        }
        if rl.is_key_down(KeyboardKey::KEY_S) {
            self.radio += velocidad_zoom;
            movio = true;
        }

        // Topes: el pitch antes de los polos y el radio afuera del cubo.
        let limite = PI / 2.0 - 0.05;
        self.pitch = self.pitch.clamp(-limite, limite);
        self.radio = self.radio.clamp(2.0, 25.0);

        movio
    }
}

// ---------- Utilidades ----------

fn a_color(v: Vec3) -> Color {
    Color::new(
        (v.x.clamp(0.0, 1.0) * 255.0) as u8,
        (v.y.clamp(0.0, 1.0) * 255.0) as u8,
        (v.z.clamp(0.0, 1.0) * 255.0) as u8,
        255,
    )
}

/// Refleja `incidente` respecto de `normal`. Las dos entran normalizadas.
fn reflejar(incidente: Vec3, normal: Vec3) -> Vec3 {
    incidente - normal * 2.0 * dot(&incidente, &normal)
}

/// Degradado vertical del fondo. `t` va de 0 arriba a 1 abajo.
fn color_de_fondo(t: f32) -> Vec3 {
    FONDO_ARRIBA * (1.0 - t) + FONDO_ABAJO * t
}

// ---------- Trazado ----------

/// Lanza un rayo contra la escena y devuelve el color que ve.
///
/// Solo habla con el trait `RayIntersect`, nunca con una primitiva
/// concreta: agregar otra figura no obliga a tocar esta funcion.
fn cast_ray(rayo: &Ray, objetos: &[&dyn RayIntersect], luz: &Luz, t_fondo: f32) -> Vec3 {
    let mut impacto = Intersect::empty();

    for objeto in objetos {
        let choque = objeto.ray_intersect(rayo);
        if choque.is_intersecting && choque.distance < impacto.distance {
            impacto = choque;
        }
    }

    if !impacto.is_intersecting {
        return color_de_fondo(t_fondo);
    }

    // El color difuso lo pone la textura, evaluada en las uv que dejo la
    // primitiva. El material solo aporta especular, brillo y albedo.
    let difuso = (impacto.material.textura)(impacto.u, impacto.v);

    let hacia_luz = normalize(&(luz.posicion - impacto.point));
    let hacia_camara = normalize(&(-rayo.direction));

    // Lambert: cuanto se inclina la cara respecto de la luz.
    let intensidad_difusa = dot(&impacto.normal, &hacia_luz).max(0.0);
    let termino_difuso =
        difuso * intensidad_difusa * luz.intensidad * impacto.material.albedo[0];

    // Phong: el reflejo de la luz apuntando al ojo.
    let reflejo = reflejar(-hacia_luz, impacto.normal);
    let intensidad_especular = dot(&reflejo, &hacia_camara)
        .max(0.0)
        .powf(impacto.material.brillo);
    let termino_especular = impacto.material.especular
        * intensidad_especular
        * luz.intensidad
        * impacto.material.albedo[1];

    let ambiente = difuso * AMBIENTE;

    ambiente + termino_difuso.component_mul(&luz.color) + termino_especular.component_mul(&luz.color)
}

/// Genera los rayos primarios y llena el framebuffer.
fn render(fb: &mut Framebuffer, objetos: &[&dyn RayIntersect], luz: &Luz, camara: &Camara) {
    let ancho = fb.width as f32;
    let alto = fb.height as f32;

    // Sin corregir el aspecto la imagen sale estirada, porque la ventana no
    // es cuadrada pero el espacio normalizado si.
    let aspecto = ancho / alto;
    let escala = (FOV * 0.5).tan();

    let origen = camara.posicion();
    let (derecha, arriba, adelante) = camara.base();

    for y in 0..fb.height {
        // La Y de pantalla crece hacia abajo y la del espacio normalizado
        // hacia arriba: sin este signo la escena sale de cabeza.
        let sy = (1.0 - 2.0 * (y as f32 + 0.5) / alto) * escala;
        let t_fondo = y as f32 / alto;

        for x in 0..fb.width {
            let sx = (2.0 * (x as f32 + 0.5) / ancho - 1.0) * aspecto * escala;

            // El pixel se arma en la base de la camara de este cuadro.
            let direccion = derecha * sx + arriba * sy + adelante;
            let rayo = Ray::new(origen, direccion);

            let color = cast_ray(&rayo, objetos, luz, t_fondo);

            fb.set_current_color(a_color(color));
            fb.point(x, y);
        }
    }
}

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(ANCHO as i32, ALTO as i32)
        .title("Cubo con raytracing")
        .build();

    rl.set_target_fps(60);

    let mut framebuffer = Framebuffer::new(ANCHO, ALTO);
    framebuffer.set_background_color(a_color(FONDO_ARRIBA));
    framebuffer.clear();

    let material = Material::new(
        Vec3::new(1.0, 1.0, 1.0),
        50.0,
        [0.9, 0.35],
        texture::damero,
    );

    let cubo = Cube::new(Vec3::new(0.0, 0.0, 0.0), 2.0, material);
    let objetos: Vec<&dyn RayIntersect> = vec![&cubo];

    let luz = Luz {
        posicion: Vec3::new(5.0, 6.0, 6.0),
        color: Vec3::new(1.0, 0.97, 0.92),
        intensidad: 1.3,
    };

    let mut camara = Camara::new(Vec3::new(0.0, 0.0, 0.0), 0.6, 0.45, 7.0);

    // Primer trazado antes de abrir el bucle: la textura de la ventana
    // tiene que nacer con la escena ya dibujada.
    render(&mut framebuffer, &objetos, &luz, &camara);
    let mut textura = rl
        .load_texture_from_image(&thread, &framebuffer.to_image())
        .expect("no se pudo crear la textura de la ventana");

    while !rl.window_should_close() {
        let delta = rl.get_frame_time();

        // Solo se vuelve a trazar si la camara se movio: el raytracing es
        // caro y la imagen no cambia sola.
        if camara.actualizar(&rl, delta) {
            render(&mut framebuffer, &objetos, &luz, &camara);
            textura = rl
                .load_texture_from_image(&thread, &framebuffer.to_image())
                .expect("no se pudo actualizar la textura de la ventana");
        }

        let mut d = rl.begin_drawing(&thread);
        d.draw_texture(&textura, 0, 0, Color::WHITE);
        d.draw_text(
            "Flechas: orbitar    W/S: acercar y alejar",
            10,
            10,
            18,
            Color::RAYWHITE,
        );
    }
}
