use nalgebra_glm::Vec3;

/// Propiedades de superficie que NO dependen del punto exacto de la cara.
///
/// El color difuso no vive aca: lo pone la textura, evaluada en (u, v) al
/// momento de sombrear. El material aporta el especular, el brillo, los
/// pesos del albedo y cuanto relieve tiene la superficie.
#[derive(Debug, Clone, Copy)]
pub struct Material {
    /// Color del reflejo especular.
    pub especular: Vec3,
    /// Exponente Phong: mas alto, brillo mas chico y concentrado.
    pub brillo: f32,
    /// Pesos [difuso, especular] con los que se mezclan los dos terminos.
    pub albedo: [f32; 2],
    /// Textura procedural que define el color difuso en (u, v).
    pub textura: fn(f32, f32) -> Vec3,
    /// Altura de la superficie en (u, v). Su pendiente inclina la normal.
    pub altura: fn(f32, f32) -> f32,
    /// Fuerza del relieve. En 0 la superficie queda lisa y la textura se
    /// ve pintada; subiendolo, los surcos agarran la luz.
    pub relieve: f32,
}

impl Material {
    pub fn new(
        especular: Vec3,
        brillo: f32,
        albedo: [f32; 2],
        textura: fn(f32, f32) -> Vec3,
        altura: fn(f32, f32) -> f32,
        relieve: f32,
    ) -> Self {
        Material {
            especular,
            brillo,
            albedo,
            textura,
            altura,
            relieve,
        }
    }

    /// Material de relleno para el Intersect vacio. Nunca se sombrea, pero
    /// evita tener que envolver el material en un Option.
    pub fn vacio() -> Self {
        Material {
            especular: Vec3::zeros(),
            brillo: 1.0,
            albedo: [0.0, 0.0],
            textura: |_u, _v| Vec3::zeros(),
            altura: |_u, _v| 0.0,
            relieve: 0.0,
        }
    }
}
