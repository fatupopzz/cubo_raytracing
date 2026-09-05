use nalgebra_glm::Vec3;
use std::sync::OnceLock;

/// Resolucion del mosaico que se genera una sola vez al arrancar. Todo el
/// ruido es caro de evaluar, y hacerlo por rayo se sentia en cada giro de
/// camara; generarlo una vez y muestrear cuesta lo mismo que leer memoria.
const LADO: usize = 1024;

/// Celdas de piedra por lado del mosaico. Pocas y grandes: la referencia
/// son manchones anchos, no un empedrado de piedritas.
const CELDAS: i32 = 3;

/// Segunda capa de grietas, mas finas y mas juntas, encima de la primera.
/// Es lo que evita que las manchas grandes se vean vacias por dentro.
const CELDAS_FINAS: i32 = 9;

/// Periodo del ruido de manchas. Es lo que ensucia la piedra por dentro.
const PERIODO_MANCHAS: i32 = 6;

/// Cuanto se deforma la reja de celdas antes de buscar los bordes. Con 0
/// las grietas serian poligonos rectos de Voronoi; esto las vuelve
/// sinuosas y organicas, que es lo que hace que parezca roca partida.
const DEFORMACION: f32 = 1.25;

/// Ancho de la grieta, medido en distancia entre la celda mas cercana y la
/// segunda. Mas alto, grietas mas gordas.
const ANCHO_GRIETA: f32 = 0.018;
const ANCHO_GRIETA_FINA: f32 = 0.012;

// Paleta: piedra parda y grisacea, con la grieta casi negra.
const COLOR_PIEDRA: Vec3 = Vec3::new(0.62, 0.58, 0.50);
const COLOR_GRIETA: Vec3 = Vec3::new(0.11, 0.095, 0.075);

/// Mosaico ya cocinado: color y altura de cada texel. La altura es la que
/// usa el relieve para que las grietas se hundan de verdad ante la luz.
struct Mosaico {
    color: Vec<Vec3>,
    altura: Vec<f32>,
}

static MOSAICO: OnceLock<Mosaico> = OnceLock::new();

// ---------- Ruido ----------

fn revolver(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

fn entero_al_azar(x: i32, y: i32, semilla: u32) -> u32 {
    revolver(
        (x as u32)
            .wrapping_mul(374_761_393)
            .wrapping_add((y as u32).wrapping_mul(668_265_263))
            .wrapping_add(semilla),
    )
}

fn al_azar(x: i32, y: i32, semilla: u32) -> f32 {
    entero_al_azar(x, y, semilla) as f32 / u32::MAX as f32
}

fn suavizar(borde_a: f32, borde_b: f32, valor: f32) -> f32 {
    let t = ((valor - borde_a) / (borde_b - borde_a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mezclar(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Ruido de valor con periodo entero. El periodo es lo que hace que el
/// mosaico calce consigo mismo: los indices de la reja se envuelven, asi
/// que el borde derecho continua en el izquierdo y no se ve la costura.
fn ruido(x: f32, y: f32, periodo: i32, semilla: u32) -> f32 {
    let x0 = x.floor();
    let y0 = y.floor();
    let fx = x - x0;
    let fy = y - y0;

    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sy = fy * fy * (3.0 - 2.0 * fy);

    let ix = x0 as i32;
    let iy = y0 as i32;
    let envolver = |i: i32| i.rem_euclid(periodo);

    let a = al_azar(envolver(ix), envolver(iy), semilla);
    let b = al_azar(envolver(ix + 1), envolver(iy), semilla);
    let c = al_azar(envolver(ix), envolver(iy + 1), semilla);
    let d = al_azar(envolver(ix + 1), envolver(iy + 1), semilla);

    mezclar(mezclar(a, b, sx), mezclar(c, d, sx), sy)
}

/// Suma de octavas: cada una con el doble de frecuencia y la mitad de peso.
/// El periodo tambien se duplica para que todas sigan siendo periodicas.
fn ruido_fractal(x: f32, y: f32, periodo: i32, octavas: u32, semilla: u32) -> f32 {
    let mut suma = 0.0;
    let mut amplitud = 0.5;
    let mut frecuencia = 1.0;
    let mut total = 0.0;

    for o in 0..octavas {
        suma += amplitud
            * ruido(
                x * frecuencia,
                y * frecuencia,
                periodo * (1 << o),
                semilla.wrapping_add(o * 101),
            );
        total += amplitud;
        amplitud *= 0.5;
        frecuencia *= 2.0;
    }

    suma / total
}

/// Ruido celular (Worley) periodico. Devuelve la distancia a la celda mas
/// cercana, a la segunda, y el identificador de la primera.
///
/// La resta entre las dos distancias es lo que dibuja la grieta: vale cero
/// justo donde dos celdas empatan, o sea en el borde entre lajas.
fn celular(x: f32, y: f32, periodo: i32, semilla: u32) -> (f32, f32, u32) {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;

    let mut mas_cerca = f32::MAX;
    let mut segunda = f32::MAX;
    let mut identificador = 0u32;

    for dy in -1..=1 {
        for dx in -1..=1 {
            let cx = ix + dx;
            let cy = iy + dy;
            let wx = cx.rem_euclid(periodo);
            let wy = cy.rem_euclid(periodo);

            let px = cx as f32 + al_azar(wx, wy, semilla);
            let py = cy as f32 + al_azar(wx, wy, semilla ^ 0x9e37_79b9);

            let distancia = ((px - x) * (px - x) + (py - y) * (py - y)).sqrt();

            if distancia < mas_cerca {
                segunda = mas_cerca;
                mas_cerca = distancia;
                identificador = entero_al_azar(wx, wy, semilla ^ 0x0517);
            } else if distancia < segunda {
                segunda = distancia;
            }
        }
    }

    (mas_cerca, segunda, identificador)
}

// ---------- Generacion del mosaico ----------

fn generar() -> Mosaico {
    let mut color = vec![Vec3::zeros(); LADO * LADO];
    let mut altura = vec![0.0f32; LADO * LADO];

    for j in 0..LADO {
        for i in 0..LADO {
            // Coordenadas en unidades de celda: el mosaico entero mide
            // CELDAS de lado, y por eso el ruido periodico calza.
            let x = i as f32 / LADO as f32 * CELDAS as f32;
            let y = j as f32 / LADO as f32 * CELDAS as f32;

            // Se deforma la posicion antes de buscar la celda. Sin esto
            // las grietas serian los bordes rectos del diagrama de Voronoi;
            // con la deformacion se vuelven serpenteantes y ramificadas,
            // que es lo que las hace leer como roca partida.
            let dx = ruido_fractal(x, y, CELDAS * 4, 5, 71) - 0.5;
            let dy = ruido_fractal(x, y, CELDAS * 4, 5, 913) - 0.5;
            let xd = x + dx * DEFORMACION;
            let yd = y + dy * DEFORMACION;

            let (cerca, segunda, identificador) = celular(xd, yd, CELDAS, 17);
            let borde = segunda - cerca;

            // Grieta principal: fuerte donde dos celdas empatan.
            let grieta_ancha = 1.0 - suavizar(0.0, ANCHO_GRIETA, borde);

            // Capa fina: la misma idea con la reja mas apretada, a media
            // fuerza, para que las manchas grandes tengan detalle adentro.
            let escala_fina = CELDAS_FINAS as f32 / CELDAS as f32;
            let (cerca_f, segunda_f, _) = celular(xd * escala_fina, yd * escala_fina, CELDAS_FINAS, 523);
            let borde_fino = segunda_f - cerca_f;
            let grieta_fina = (1.0 - suavizar(0.0, ANCHO_GRIETA_FINA, borde_fino)) * 0.45;

            let grieta = (grieta_ancha + grieta_fina * (1.0 - grieta_ancha)).clamp(0.0, 1.0);

            // Halo claro justo afuera de la grieta, como el canto gastado
            // de la piedra. Es lo que le da el relieve a la vista.
            let halo = suavizar(0.004, 0.03, borde) * (1.0 - suavizar(0.03, 0.10, borde));

            // Cada mancha tiene su tono, apenas distinto del vecino, y
            // encima el ruido fractal la ensucia por dentro con nubes.
            let tono = 0.90 + 0.20 * (identificador as f32 / u32::MAX as f32);
            let manchas = ruido_fractal(x, y, PERIODO_MANCHAS, 6, 4_211);
            let nubes = ruido_fractal(x * 0.5, y * 0.5, PERIODO_MANCHAS, 4, 77);
            let grano = ruido(x * 40.0, y * 40.0, PERIODO_MANCHAS * 40, 8_821);

            let claridad =
                tono * (0.52 + 0.72 * manchas) * (0.70 + 0.60 * nubes) * (0.95 + 0.10 * grano);

            let piedra = COLOR_PIEDRA * claridad + Vec3::new(0.10, 0.10, 0.09) * halo;
            let final_ = piedra * (1.0 - grieta) + COLOR_GRIETA * grieta;

            let indice = j * LADO + i;
            color[indice] = final_;
            // La grieta se hunde; el resto ondula apenas con las manchas.
            altura[indice] = (1.0 - grieta) * (0.85 + 0.15 * manchas);
        }
    }

    Mosaico { color, altura }
}

fn mosaico() -> &'static Mosaico {
    MOSAICO.get_or_init(generar)
}

/// Muestreo bilineal con envoltura. Los indices dan la vuelta, asi que la
/// cara se ve continua aunque la uv toque justo el borde del mosaico.
fn muestrear<T, F>(u: f32, v: f32, leer: F) -> T
where
    T: std::ops::Add<Output = T> + std::ops::Mul<f32, Output = T>,
    F: Fn(usize, usize) -> T,
{
    let x = u * LADO as f32 - 0.5;
    let y = v * LADO as f32 - 0.5;

    let x0 = x.floor();
    let y0 = y.floor();
    let fx = x - x0;
    let fy = y - y0;

    let envolver = |i: f32| (i as i32).rem_euclid(LADO as i32) as usize;
    let i0 = envolver(x0);
    let j0 = envolver(y0);
    let i1 = envolver(x0 + 1.0);
    let j1 = envolver(y0 + 1.0);

    let arriba = leer(i0, j0) * (1.0 - fx) + leer(i1, j0) * fx;
    let abajo = leer(i0, j1) * (1.0 - fx) + leer(i1, j1) * fx;

    arriba * (1.0 - fy) + abajo * fy
}

/// Color difuso de la piedra en (u, v). Todo salio de ruido, no hay ningun
/// archivo de imagen detras.
pub fn piedra(u: f32, v: f32) -> Vec3 {
    let m = mosaico();
    muestrear(u, v, |i, j| m.color[j * LADO + i])
}

/// Altura de la superficie en (u, v). La usa el relieve para inclinar la
/// normal y que las grietas se vean hundidas en vez de pintadas.
pub fn altura_piedra(u: f32, v: f32) -> f32 {
    let m = mosaico();
    muestrear(u, v, |i, j| m.altura[j * LADO + i])
}
