use crate::material::Material;
use nalgebra_glm::{normalize, Vec3};

/// Rayo con origen y direccion. La direccion SIEMPRE queda normalizada al
/// construirlo: asi el parametro t de las intersecciones es distancia real
/// y no hace falta volver a normalizar en cada primitiva.
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Ray {
            origin,
            direction: normalize(&direction),
        }
    }
}

/// Resultado de intentar chocar un rayo contra una primitiva.
///
/// Trae ya resueltas las coordenadas de textura (u, v): las calcula cada
/// primitiva porque solo ella sabe como se mapea su superficie. Gracias a
/// eso `cast_ray` no necesita saber contra que figura choco.
pub struct Intersect {
    pub distance: f32,
    pub point: Vec3,
    pub normal: Vec3,
    pub u: f32,
    pub v: f32,
    /// Direcciones en las que crecen u y v sobre la superficie. Las pone la
    /// primitiva porque solo ella sabe como mapeo su textura, y son las que
    /// dejan inclinar la normal con el relieve sin que el sombreado tenga
    /// que saber contra que figura choco.
    pub tangente: Vec3,
    pub bitangente: Vec3,
    pub material: Material,
    pub is_intersecting: bool,
}

impl Intersect {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        distance: f32,
        point: Vec3,
        normal: Vec3,
        u: f32,
        v: f32,
        tangente: Vec3,
        bitangente: Vec3,
        material: Material,
    ) -> Self {
        Intersect {
            distance,
            point,
            normal,
            u,
            v,
            tangente,
            bitangente,
            material,
            is_intersecting: true,
        }
    }

    /// El "no le pegue a nada". La distancia infinita hace que nunca gane
    /// la comparacion del objeto mas cercano.
    pub fn empty() -> Self {
        Intersect {
            distance: f32::INFINITY,
            point: Vec3::zeros(),
            normal: Vec3::zeros(),
            u: 0.0,
            v: 0.0,
            tangente: Vec3::zeros(),
            bitangente: Vec3::zeros(),
            material: Material::vacio(),
            is_intersecting: false,
        }
    }
}

/// Contrato que cumple toda primitiva. Mientras una figura nueva implemente
/// esto, entra a la escena sin tocar `cast_ray` ni `render`.
pub trait RayIntersect {
    fn ray_intersect(&self, ray: &Ray) -> Intersect;
}
