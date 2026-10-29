#!/usr/bin/env bash
# Métricas de estabilidad desde el historial de git (equivalentes aproximados a DORA).
# Uso: scripts/stability.sh [dias=90]
set -u
cd "$(dirname "$0")/.."
days=${1:-90}
since="$days days ago"

total=$(git rev-list --count --since="$since" HEAD)
# Mensajes en español: «corrige», «arregla», «correcciones», «revierte», o prefijos fix/revert.
fixes=$(git log --since="$since" --format=%s | grep -ciE '^(fix|revert)|corrige|correcci|arregl|revierte' || true)

echo "Ventana: últimos $days días"
echo "Commits totales:          $total"
echo "Commits de arreglo:       $fixes"
if [ "$total" -gt 0 ]; then
  echo "Tasa de arreglos:         $((100 * fixes / total)) %   (más baja = mejor)"
fi

# Retrabajo: archivos que se vuelven a tocar a menos de 14 días de su cambio anterior.
rework=$(git log --since="$since" --reverse --name-only --format='@%ct' | awk '
  /^@/ { t = substr($0, 2); next }
  NF   { if ((f in last) && t - last[f] < 14*86400) n++; last[f] = t; f = "" }
  END  { print n + 0 }' f="")
echo "Retoques a <14 días:      $rework  (cambios de archivo que repiten otro reciente)"
