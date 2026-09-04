use nalgebra_glm::Vec3;

/// Propiedades de superficie. En esta rama el sombreado es solo difuso,
/// asi que el material no guarda especular ni brillo: alcanza con el peso
/// del termino difuso y la funcion que da el color.
#[derive(Debug, Clone, Copy)]
pub struct Material {
    /// Peso del termino difuso.
    pub albedo: f32,
    /// Funcion que define el color de la superficie en (u, v).
    pub textura: fn(f32, f32) -> Vec3,
}

impl Material {
    pub fn new(albedo: f32, textura: fn(f32, f32) -> Vec3) -> Self {
        Material { albedo, textura }
    }

    /// Material de relleno para el Intersect vacio. Nunca se sombrea, pero
    /// evita tener que envolver el material en un Option.
    pub fn vacio() -> Self {
        Material {
            albedo: 0.0,
            textura: |_u, _v| Vec3::zeros(),
        }
    }
}
