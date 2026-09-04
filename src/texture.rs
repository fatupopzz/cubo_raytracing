use nalgebra_glm::Vec3;

/// Casillas por cara, en cada eje del mapeo uv. Con 8 el damero se ve
/// parejo de cerca y la perspectiva se nota cuando la cara se inclina.
const CASILLAS: f32 = 8.0;

/// Los dos colores del damero. Contrastan fuerte a proposito: es lo que
/// deja ver como se deforman las caras al girar la camara.
const COLOR_A: Vec3 = Vec3::new(0.92, 0.86, 0.72);
const COLOR_B: Vec3 = Vec3::new(0.18, 0.32, 0.55);

/// Damero procedural: no se carga nada de disco, el color sale de la
/// paridad de la casilla en la que cae (u, v).
pub fn damero(u: f32, v: f32) -> Vec3 {
    let cu = (u.clamp(0.0, 1.0) * CASILLAS).floor() as i32;
    let cv = (v.clamp(0.0, 1.0) * CASILLAS).floor() as i32;

    if (cu + cv) % 2 == 0 {
        COLOR_A
    } else {
        COLOR_B
    }
}
