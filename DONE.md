# Definición de «terminado»

Un cambio no está terminado porque los tests pasen. Está terminado cuando cumple todo esto.

## 1. Comprobaciones automáticas

`scripts/check-quality.sh` debe salir con código 0. Cubre:

- `cargo fmt` y `cargo clippy -D warnings` (lo mismo que exige CI).
- Complejidad cognitiva de funciones Rust (umbral en `clippy.toml`).
- Secretos en el repo (`gitleaks`).
- Dependencias con vulnerabilidades conocidas (`cargo audit`) y dependencias sin usar (`cargo machete`), si están instaladas.
- Tipos de la UI (`tsc --noEmit`).

Los tests (`cargo test --workspace`, `pnpm --dir ui test`) los ejecuta CI; hay que correrlos antes de integrar.

## 2. Revisión humana (5 minutos, no se delega)

- [ ] ¿Hace lo que se pidió y solo eso? Nada de funciones, pantallas o abstracciones «por si acaso».
- [ ] ¿Se puede borrar algo que quedó de un experimento o de una versión anterior?
- [ ] ¿Los tests comprueban el comportamiento o solo repiten la implementación? Probar un caso límite a mano.
- [ ] Si toca la UI: abrirla y usarla, no solo compilarla. ¿Tiene el gusto de GoyMusic o parece una plantilla?
- [ ] ¿Una decisión que la IA tomó por defecto (patrón, librería, estructura) es la que yo habría elegido?

## 3. Estabilidad

`scripts/stability.sh` da la tasa de commits de arreglo y el retrabajo reciente. Se mira cada semana; si suben, se frena el código nuevo.
