#!/usr/bin/env bash
# Comprobaciones de calidad de DONE.md. Salida corta: PASS / FAIL / SKIP por punto.
# Uso: scripts/check-quality.sh   (desde cualquier directorio)
set -u
cd "$(dirname "$0")/.."

fails=0

run() { # run <nombre> <comando...>
  local name=$1; shift
  if out=$("$@" 2>&1); then
    printf 'PASS  %s\n' "$name"
  else
    printf 'FAIL  %s\n' "$name"
    printf '%s\n' "$out" | tail -n 15 | sed 's/^/      /'
    fails=$((fails + 1))
  fi
}

skip() { printf 'SKIP  %s (%s)\n' "$1" "$2"; }

# El crate de escritorio incrusta ui/dist: sin build de la UI no se puede compilar.
cargo_scope=(--workspace)
if [ ! -d ui/dist ]; then
  cargo_scope=(--workspace --exclude goymusic-desktop)
  skip "goymusic-desktop" "falta ui/dist; ejecuta: pnpm --dir ui build"
fi

run "cargo fmt" cargo fmt --all -- --check
run "clippy -D warnings" cargo clippy "${cargo_scope[@]}" --all-targets -- -D warnings
run "complejidad cognitiva" cargo clippy "${cargo_scope[@]}" -- -D clippy::cognitive_complexity

if command -v gitleaks >/dev/null; then
  run "secretos (gitleaks)" gitleaks detect --no-banner --redact
else
  skip "secretos" "gitleaks no instalado"
fi

if cargo audit --version >/dev/null 2>&1; then
  run "vulnerabilidades (cargo audit)" cargo audit
else
  skip "vulnerabilidades" "instala: cargo install cargo-audit --locked"
fi

if cargo machete --version >/dev/null 2>&1; then
  run "dependencias sin usar (machete)" cargo machete
else
  skip "dependencias sin usar" "instala: cargo install cargo-machete --locked"
fi

if [ -d ui/node_modules ]; then
  run "tipos UI (tsc)" pnpm --dir ui exec tsc --noEmit
else
  skip "tipos UI" "ejecuta: pnpm install"
fi

echo
if [ "$fails" -eq 0 ]; then echo "OK: sin fallos"; else echo "$fails comprobaciones fallan"; fi
exit $((fails > 0))
