use nalgebra_glm::Vec3;

/// Color plano del cubo en esta rama. No hay damero: la idea es ver el
/// sombreado difuso solo, sin que el patron distraiga de como cae la luz
/// en cada cara.
const COLOR_BASE: Vec3 = Vec3::new(0.85, 0.62, 0.38);

/// Misma firma que cualquier textura procedural, pero ignora (u, v) y
/// devuelve siempre el mismo color.
pub fn plano(_u: f32, _v: f32) -> Vec3 {
    COLOR_BASE
}
