# GoyMusic

Reproductor de música open source para música propia (Rust + Tauri, UI en TypeScript/Vite). Workspace: `crates/{core,library,audio,services}` y `apps/desktop`.

## Cuándo está terminado

Ver `DONE.md`. Antes de dar algo por hecho: `scripts/check-quality.sh` en verde, tests de Rust y de la UI en verde, y la revisión humana de la checklist.

## Reglas

- Haz solo lo que se pide. No añadas funciones, pantallas ni abstracciones que nadie ha pedido.
- Borra el código que un cambio deja sin uso.
- Los tests comprueban comportamiento, no repiten la implementación.
- La UI se prueba abriéndola, no solo compilándola.
