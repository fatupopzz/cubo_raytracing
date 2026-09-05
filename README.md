# cubo_raytracing

Raytracer mínimo en Rust que renderiza un cubo texturizado en tiempo real, con
cámara en órbita. Proyecto del curso de Gráficas por Computadora (CC2018),
Universidad del Valle de Guatemala.

La geometría se resuelve enteramente con intersección rayo–primitiva: no hay
mallas, ni z-buffer, ni pipeline de rasterización.

## Cómo correrlo

```bash
cargo run --release
```

El flag `--release` no es opcional. En debug el trazado de rayos es lo bastante
lento como para que la ventana sea inusable.

### Controles

| Tecla | Acción |
|---|---|
| Flechas | Orbitar la cámara (yaw y pitch) |
| W / S | Acercar y alejar |

El cubo está alineado a los ejes y no rota; la que se mueve es la cámara. Vista
de frente a una cara, un cubo se ve como un cuadrado: eso es correcto, no un
error de render. Girá la cámara para ver las tres caras y cómo cada una recibe
luz distinta.

## Cómo funciona

### Intersección por el método de slabs

Un cubo alineado a los ejes (AABB) es la intersección de tres franjas infinitas,
una por eje. Cada franja aporta un intervalo de `t` durante el cual el rayo está
dentro de ella:

```
t_entrada = max(tx0, ty0, tz0)
t_salida  = min(tx1, ty1, tz1)
si t_salida < t_entrada  ->  el rayo pasa de largo
```

No hay raíz cuadrada ni ecuación cuadrática: solo divisiones y comparaciones. Y
el eje que impuso el `max` identifica la cara golpeada, así que la normal sale
gratis como vector base con el signo opuesto al de la dirección del rayo.

Tres detalles que hacen la diferencia entre que funcione y que casi funcione:

- **Swap de `t0` y `t1`.** `t0 = (min - o)/d` solo es el plano cercano si
  `d > 0`. Con dirección negativa el orden se invierte. Sin el swap, la caja
  desaparece únicamente desde ciertos ángulos, lo cual es más difícil de
  detectar que si desapareciera siempre.
- **Caso paralelo tratado aparte.** Cuando la dirección es casi cero en un eje,
  `0.0/0.0` produce `NaN`. Toda comparación con `NaN` es falsa, así que el
  algoritmo aceptaría cajas que el rayo nunca tocó.
- **Epsilon de `1e-4`.** Sin él, un rayo que sale de una cara vuelve a
  intersectarla en `t ≈ 1e-7` por error de punto flotante. Es el mismo acné
  que aparece en los rayos de sombra, aquí en versión caja.

### Textura

Damero procedural de 8 casillas por cara, generado por código: no se carga
ningún archivo de imagen. Las coordenadas UV las resuelve la propia primitiva
—elige el eje dominante de la normal y normaliza las otras dos coordenadas
contra el tamaño de la caja— y viajan dentro del `Intersect`, así que el
sombreado nunca necesita saber qué tipo de objeto golpeó.

### Iluminación

Una luz puntual, con difuso Lambert y especular Phong. El fondo es un gradiente
vertical para que la silueta del cubo se lea contra él.

## Estructura

```
src/
├── main.rs           ventana, cámara en órbita, rayos primarios, cast_ray, render loop
├── framebuffer.rs    buffer de Color
├── ray_intersect.rs  Ray, Intersect y el trait RayIntersect
├── material.rs       especular, brillo, albedo y puntero a la función de textura
├── texture.rs        damero procedural (u,v) -> Vec3
└── cube.rs           primitiva cubo (AABB) por slabs
```

Cada primitiva vive en su propio archivo e implementa `RayIntersect`. Agregar
otra no obliga a tocar `cast_ray` ni `render`: el sombreado solo habla con
`dyn RayIntersect`.

`framebuffer.point(x, y)` no recibe color. Pinta con el color actual, fijado
antes por `set_current_color`, y es el único punto de escritura del buffer.

`Ray::new` normaliza la dirección al construirse, para que no pueda existir un
rayo mal formado y `t` siga siendo una distancia real.

## Ramas

| Rama | Contenido |
|---|---|
| `master`, `textura` | Entrega completa: damero + Lambert + especular Phong |
| `luz-difusa` | El mismo cubo solo con Lambert y ambiente, color plano |

## Dependencias

- `raylib` 5.5 — ventana y presentación del framebuffer
- `nalgebra-glm` 0.21 — vectores

La textura es procedural, así que el proyecto no tiene assets externos.
