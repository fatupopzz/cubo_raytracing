use crate::material::Material;
use crate::ray_intersect::{Intersect, Ray, RayIntersect};
use nalgebra_glm::Vec3;

/// Tolerancia para aceptar un impacto. Sin esto un rayo que sale de la
/// superficie se choca contra su propia cara en t = 0.
const EPSILON: f32 = 1e-4;

/// Debajo de esto se considera que el rayo es paralelo a la franja del eje
/// y la division no sirve.
const CASI_CERO: f32 = 1e-8;

/// Cubo alineado a los ejes (AABB), definido por sus dos esquinas.
pub struct Cube {
    pub min: Vec3,
    pub max: Vec3,
    pub material: Material,
}

impl Cube {
    /// Cubo centrado en `centro` con lado `arista`.
    pub fn new(centro: Vec3, arista: f32, material: Material) -> Self {
        let mitad = arista * 0.5;
        let desplazamiento = Vec3::new(mitad, mitad, mitad);

        Cube::from_bounds(centro - desplazamiento, centro + desplazamiento, material)
    }

    /// Caja a partir de sus esquinas, por si se quiere una no cubica.
    pub fn from_bounds(min: Vec3, max: Vec3, material: Material) -> Self {
        Cube { min, max, material }
    }

    /// Mapeo uv por cara: se elige el eje dominante de la normal (o sea,
    /// cual de las seis caras es) y las otras dos coordenadas se normalizan
    /// contra el tamano de la caja. Asi cada cara recibe el rango 0..1
    /// completo y el damero se ve entero en cada una.
    pub fn uv(&self, punto: Vec3, normal: Vec3) -> (f32, f32) {
        let (eje_u, signo_u, eje_v, signo_v) = Cube::mapeo_de_cara(normal);

        let coordenada = |eje: usize, signo: f32| {
            let t = (punto[eje] - self.min[eje]) / (self.max[eje] - self.min[eje]);
            let t = if signo > 0.0 { t } else { 1.0 - t };
            t.clamp(0.0, 1.0)
        };

        (coordenada(eje_u, signo_u), coordenada(eje_v, signo_v))
    }

    /// Cual de las seis caras es, o sea sobre que eje apunta la normal.
    fn eje_dominante(normal: Vec3) -> usize {
        let (nx, ny, nz) = (normal.x.abs(), normal.y.abs(), normal.z.abs());

        if nx >= ny && nx >= nz {
            0
        } else if ny >= nz {
            1
        } else {
            2
        }
    }

    /// Que eje recorre u en esta cara y en que sentido, e igual para v.
    ///
    /// Los sentidos no son arbitrarios: van dando la vuelta al cubo siempre
    /// para el mismo lado. Si todas las caras midieran desde su esquina
    /// minima, las caras vecinas quedarian espejadas y en cada arista se
    /// veria una mariposa, el patron reflejandose contra si mismo. Asi, y
    /// como el mosaico es continuo consigo mismo, el dibujo cruza la arista
    /// sin cortarse.
    fn mapeo_de_cara(normal: Vec3) -> (usize, f32, usize, f32) {
        match Cube::eje_dominante(normal) {
            // Caras de X: u recorre Z, v recorre Y.
            0 => {
                if normal.x > 0.0 {
                    (2, -1.0, 1, 1.0)
                } else {
                    (2, 1.0, 1, 1.0)
                }
            }
            // Caras de Y: u recorre X, v recorre Z.
            1 => {
                if normal.y > 0.0 {
                    (0, 1.0, 2, 1.0)
                } else {
                    (0, 1.0, 2, -1.0)
                }
            }
            // Caras de Z: u recorre X, v recorre Y.
            _ => {
                if normal.z > 0.0 {
                    (0, 1.0, 1, 1.0)
                } else {
                    (0, -1.0, 1, 1.0)
                }
            }
        }
    }

    /// Direcciones en las que crecen u y v sobre la cara. Tienen que
    /// coincidir con el mapeo de `uv`, porque el relieve inclina la normal
    /// justo en esos dos sentidos.
    fn base_de_cara(normal: Vec3) -> (Vec3, Vec3) {
        let (eje_u, signo_u, eje_v, signo_v) = Cube::mapeo_de_cara(normal);

        let mut tangente = Vec3::zeros();
        let mut bitangente = Vec3::zeros();
        tangente[eje_u] = signo_u;
        bitangente[eje_v] = signo_v;

        (tangente, bitangente)
    }
}

impl RayIntersect for Cube {
    /// Metodo de slabs: la caja es la interseccion de tres franjas, una por
    /// eje. Se recorta el intervalo [t_entrada, t_salida] del rayo contra
    /// cada franja; si al final el intervalo sigue vivo, hubo impacto.
    fn ray_intersect(&self, ray: &Ray) -> Intersect {
        let mut t_entrada = f32::NEG_INFINITY;
        let mut t_salida = f32::INFINITY;
        let mut eje_entrada = 0usize;
        let mut eje_salida = 0usize;

        for i in 0..3 {
            let origen = ray.origin[i];
            let direccion = ray.direction[i];

            if direccion.abs() < CASI_CERO {
                // Rayo paralelo a esta franja: la division daria 0.0/0.0 y
                // el NaN resultante hace falsa toda comparacion, con lo que
                // el algoritmo aceptaria cajas que el rayo nunca toco.
                // Se resuelve aparte: si el origen esta fuera de la franja
                // no hay impacto, y si esta dentro este eje no restringe.
                if origen < self.min[i] || origen > self.max[i] {
                    return Intersect::empty();
                }
                continue;
            }

            let mut t0 = (self.min[i] - origen) / direccion;
            let mut t1 = (self.max[i] - origen) / direccion;

            // Con direccion negativa la division invierte el orden: sin
            // este swap la caja desaparece desde ciertos angulos.
            if t0 > t1 {
                std::mem::swap(&mut t0, &mut t1);
            }

            if t0 > t_entrada {
                t_entrada = t0;
                eje_entrada = i;
            }

            if t1 < t_salida {
                t_salida = t1;
                eje_salida = i;
            }

            if t_salida < t_entrada {
                return Intersect::empty();
            }
        }

        // t_entrada es la cara por la que entra el rayo. Si quedo detras
        // del origen pero t_salida no, la camara esta adentro del cubo y el
        // impacto valido es la cara de salida.
        let (distancia, eje) = if t_entrada > EPSILON {
            (t_entrada, eje_entrada)
        } else if t_salida > EPSILON {
            (t_salida, eje_salida)
        } else {
            return Intersect::empty();
        };

        let punto = ray.origin + ray.direction * distancia;

        // La normal es el vector base del eje que gano la comparacion, con
        // el signo opuesto al de la direccion del rayo en ese eje: asi
        // siempre mira hacia el rayo, incluso desde adentro del cubo.
        let mut normal = Vec3::zeros();
        normal[eje] = if ray.direction[eje] > 0.0 { -1.0 } else { 1.0 };

        let (u, v) = self.uv(punto, normal);
        let (tangente, bitangente) = Cube::base_de_cara(normal);

        Intersect::new(
            distancia,
            punto,
            normal,
            u,
            v,
            tangente,
            bitangente,
            self.material,
        )
    }
}
