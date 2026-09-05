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

/// Muestras por eje dentro de cada pixel. Con 2 salen 4 rayos por pixel y
/// los bordes del cubo dejan de ser la escalera dura de un solo rayo.
const MUESTRAS: usize = 2;

/// Peso del ambiente. Sin un piso de luz, toda cara que no ve la luz queda
/// negra y el cubo pierde el volumen.
const AMBIENTE: f32 = 0.70;

/// El ambiente no es plano: llega mas frio desde arriba y mas calido desde
/// abajo, como si el entorno rebotara luz. Es barato y le saca el aire de
/// maqueta al render.
const AMBIENTE_CIELO: Vec3 = Vec3::new(0.34, 0.40, 0.52);
const AMBIENTE_SUELO: Vec3 = Vec3::new(0.30, 0.25, 0.20);

/// Colores del degradado de fondo, de arriba hacia abajo. Un fondo plano
/// negro se comeria la silueta del cubo.
const FONDO_ARRIBA: Vec3 = Vec3::new(0.05, 0.07, 0.13);
const FONDO_ABAJO: Vec3 = Vec3::new(0.42, 0.47, 0.56);

// ---------- Luz ----------

/// Luz puntual: irradia desde un punto en todas direcciones.
struct Luz {
    posicion: Vec3,
    color: Vec3,
    intensidad: f32,
}

impl Luz {
    fn new(posicion: Vec3, color: Vec3, intensidad: f32) -> Self {
        Luz {
            posicion,
            color,
            intensidad,
        }
    }
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
        self.radio = self.radio.clamp(2.2, 25.0);

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

/// Inclina la normal segun la pendiente de la altura del material.
///
/// Es lo que hace que las grietas se vean hundidas y no pintadas: donde la
/// altura cae, la normal se ladea y esa parte deja de mirar a la luz.
/// Trabaja con la tangente y la bitangente que dejo la primitiva, asi que
/// sirve para cualquier figura que las llene.
fn normal_con_relieve(impacto: &Intersect) -> Vec3 {
    if impacto.material.relieve <= 0.0 {
        return impacto.normal;
    }

    let altura = impacto.material.altura;
    let paso = 1.0 / 512.0;

    let pendiente_u = altura(impacto.u + paso, impacto.v) - altura(impacto.u - paso, impacto.v);
    let pendiente_v = altura(impacto.u, impacto.v + paso) - altura(impacto.u, impacto.v - paso);

    let desvio = impacto.tangente * pendiente_u + impacto.bitangente * pendiente_v;

    normalize(&(impacto.normal - desvio * impacto.material.relieve))
}

// ---------- Trazado ----------

/// Lanza un rayo contra la escena y devuelve el color que ve.
///
/// Solo habla con el trait `RayIntersect`, nunca con una primitiva
/// concreta: agregar otra figura no obliga a tocar esta funcion.
fn cast_ray(rayo: &Ray, objetos: &[&dyn RayIntersect], luces: &[Luz], t_fondo: f32) -> Vec3 {
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
    // primitiva. El material solo aporta especular, brillo, albedo y
    // relieve.
    let difuso = (impacto.material.textura)(impacto.u, impacto.v);
    let normal = normal_con_relieve(&impacto);

    let hacia_camara = normalize(&(-rayo.direction));

    // Al fondo de una grieta le entra menos luz que a la cara expuesta. Es
    // una oclusion de a mentiras, sacada de la misma altura que el relieve,
    // y es lo que le da profundidad a los surcos.
    let oclusion = 0.35 + 0.65 * (impacto.material.altura)(impacto.u, impacto.v);

    // Cada luz aporta su Lambert y su Phong, y las contribuciones se suman.
    // Sumar es lo correcto y no promediar: dos luces iluminan mas que una.
    // Como no hay sombras, ninguna luz se tapa contra el propio cubo, pero
    // tampoco hace falta: es un solo objeto convexo y la cara que no mira a
    // una luz ya recibe cero por el coseno de Lambert.
    let mut acumulado = Vec3::zeros();

    for luz in luces {
        let hacia_luz = normalize(&(luz.posicion - impacto.point));

        // Lambert: cuanto se inclina la cara respecto de esta luz.
        let intensidad_difusa = dot(&normal, &hacia_luz).max(0.0);
        if intensidad_difusa <= 0.0 {
            continue;
        }

        let termino_difuso =
            difuso * intensidad_difusa * luz.intensidad * impacto.material.albedo[0];

        // Phong: el reflejo de esta luz apuntando al ojo.
        let reflejo = reflejar(-hacia_luz, normal);
        let intensidad_especular = dot(&reflejo, &hacia_camara)
            .max(0.0)
            .powf(impacto.material.brillo);
        let termino_especular = impacto.material.especular
            * intensidad_especular
            * luz.intensidad
            * impacto.material.albedo[1];

        acumulado += termino_difuso.component_mul(&luz.color) * oclusion
            + termino_especular.component_mul(&luz.color);
    }

    let cielo = 0.5 + 0.5 * normal.y;
    let luz_de_entorno = AMBIENTE_CIELO * cielo + AMBIENTE_SUELO * (1.0 - cielo);
    let ambiente = difuso.component_mul(&luz_de_entorno) * AMBIENTE * oclusion;

    ambiente + acumulado
}

/// Genera los rayos primarios y llena el framebuffer.
fn render(fb: &mut Framebuffer, objetos: &[&dyn RayIntersect], luces: &[Luz], camara: &Camara) {
    let ancho = fb.width as f32;
    let alto = fb.height as f32;

    // Sin corregir el aspecto la imagen sale estirada, porque la ventana no
    // es cuadrada pero el espacio normalizado si.
    let aspecto = ancho / alto;
    let escala = (FOV * 0.5).tan();

    let origen = camara.posicion();
    let (derecha, arriba, adelante) = camara.base();
    let muestras_por_pixel = (MUESTRAS * MUESTRAS) as f32;

    for y in 0..fb.height {
        for x in 0..fb.width {
            let mut acumulado = Vec3::zeros();

            // Varias muestras repartidas dentro del pixel, promediadas al
            // final: eso es el antialiasing.
            for sub_y in 0..MUESTRAS {
                for sub_x in 0..MUESTRAS {
                    let px = x as f32 + (sub_x as f32 + 0.5) / MUESTRAS as f32;
                    let py = y as f32 + (sub_y as f32 + 0.5) / MUESTRAS as f32;

                    let sx = (2.0 * px / ancho - 1.0) * aspecto * escala;
                    // La Y de pantalla crece hacia abajo y la del espacio
                    // normalizado hacia arriba: sin este signo la escena
                    // sale de cabeza.
                    let sy = (1.0 - 2.0 * py / alto) * escala;

                    // El pixel se arma en la base de la camara de este
                    // cuadro.
                    let direccion = derecha * sx + arriba * sy + adelante;
                    let rayo = Ray::new(origen, direccion);

                    acumulado += cast_ray(&rayo, objetos, luces, py / alto);
                }
            }

            fb.set_current_color(a_color(acumulado / muestras_por_pixel));
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

    // Piedra: casi nada de especular y un brillo ancho, porque la roca no
    // es un espejo; el relieve es el que hace el trabajo pesado.
    let material = Material::new(
        Vec3::new(1.0, 0.98, 0.94),
        22.0,
        [0.95, 0.16],
        texture::piedra,
        texture::altura_piedra,
        1.1,
    );

    let cubo = Cube::new(Vec3::new(0.0, 0.0, 0.0), 2.0, material);
    let objetos: Vec<&dyn RayIntersect> = vec![&cubo];

    // Tres luces puntuales, cada una con su color y su trabajo. El esquema
    // es el de estudio: una manda, otra rellena la sombra y la tercera
    // recorta el contorno desde atras.
    let luces = vec![
        // Clave: la fuerte y calida, arriba a la derecha y adelante.
        Luz::new(Vec3::new(5.0, 6.0, 6.0), Vec3::new(1.0, 0.94, 0.85), 1.15),
        // Relleno: fria y suave, del lado opuesto, para que la cara en
        // sombra muestre su textura en vez de quedar negra.
        Luz::new(Vec3::new(-7.0, 1.5, 4.0), Vec3::new(0.42, 0.58, 1.0), 0.68),
        // Contra: calida y baja, desde atras. Es la que despega el cubo
        // del fondo dibujandole un canto encendido.
        Luz::new(Vec3::new(-3.5, -4.0, -7.0), Vec3::new(1.0, 0.62, 0.38), 0.62),
    ];

    let mut camara = Camara::new(Vec3::new(0.0, 0.0, 0.0), 0.6, 0.45, 6.5);

    // Primer trazado antes de abrir el bucle: la textura de la ventana
    // tiene que nacer con la escena ya dibujada.
    render(&mut framebuffer, &objetos, &luces, &camara);
    let mut textura = rl
        .load_texture_from_image(&thread, &framebuffer.to_image())
        .expect("no se pudo crear la textura de la ventana");

    while !rl.window_should_close() {
        let delta = rl.get_frame_time();

        // Solo se vuelve a trazar si la camara se movio: el raytracing es
        // caro y la imagen no cambia sola.
        if camara.actualizar(&rl, delta) {
            render(&mut framebuffer, &objetos, &luces, &camara);
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
