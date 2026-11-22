#!/bin/sh
# NaranjaHub — instalador portable autoextraible (.run)
# Copyright (C) 2026 Leyter Vázquez. V — Licencia GNU GPL v3
#
# Uso:
#   ./NaranjaHub-<versión>-x86_64.run              Instala en ~/.local/opt/NaranjaHub (sin root)
#   ./NaranjaHub-<versión>-x86_64.run --dir RUTA   Instala en la carpeta que elijas
#   ./NaranjaHub-<versión>-x86_64.run --portable   Extrae junto al archivo (modo memoria USB)
#   ./NaranjaHub-<versión>-x86_64.run --no-menu    No crea el acceso directo en el menú
#   ./NaranjaHub-<versión>-x86_64.run --uninstall  Desinstala (borra la carpeta y el acceso)
#   ./NaranjaHub-<versión>-x86_64.run --help       Muestra esta ayuda
#
# El instalador es 100% portable: solo necesita sh, tar y base64 (incluidos
# en cualquier Linux). No requiere root, FUSE ni aplicaciones externas.

set -eu

# Versión solo informativa; la carpeta extraída se detecta sola, por lo
# que el instalador sigue funcionando aunque cambie el número de versión.
VERSION="@VERSION@"
DIR=""
PORTABLE=0
MENU=1
UNINSTALL=0

print_help() {
    # Imprime el bloque de comentarios inicial (uso) sin líneas de código.
    awk 'NR > 1 && /^#/ { sub(/^# ?/, ""); print }
         NR > 1 && !/^#/ && !/^$/ { exit }' "$0"
}

while [ $# -gt 0 ]; do
    case "$1" in
        --dir)
            DIR="${2:-}"
            if [ -z "$DIR" ]; then
                echo "Error: --dir necesita una ruta." >&2
                exit 2
            fi
            shift 2
            ;;
        --portable) PORTABLE=1; shift ;;
        --no-menu) MENU=0; shift ;;
        --uninstall) UNINSTALL=1; shift ;;
        --help|-h) print_help; exit 0 ;;
        *)
            echo "Opción desconocida: $1 (usa --help)" >&2
            exit 2
            ;;
    esac
done

HOME_DIR="${HOME:-/tmp}"

if [ "$UNINSTALL" = "1" ]; then
    if [ "$PORTABLE" = "1" ]; then
        DEST="${DIR:-$(pwd)/NaranjaHub}"
    else
        DEST="${DIR:-$HOME_DIR/.local/opt/NaranjaHub}"
    fi
    rm -rf "$DEST"
    rm -f "$HOME_DIR/.local/share/applications/naranjahub.desktop"
    echo "NaranjaHub desinstalado (se eliminó $DEST y su acceso directo)."
    exit 0
fi

if [ "$PORTABLE" = "1" ]; then
    DEST="${DIR:-$(pwd)/NaranjaHub}"
else
    DEST="${DIR:-$HOME_DIR/.local/opt/NaranjaHub}"
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

ARCHIVE_LINE="$(awk '/^__NARANJAHUB_PAYLOAD__$/{print NR + 1; exit}' "$0")"
if [ -z "$ARCHIVE_LINE" ]; then
    echo "Error: instalador corrupto (falta la carga)." >&2
    exit 1
fi
tail -n +"$ARCHIVE_LINE" "$0" | base64 -d | tar -xzf - -C "$WORK"

mkdir -p "$DEST"
SRC_DIR="$(find "$WORK" -maxdepth 1 -type d -name 'NaranjaHub-*' | head -n 1)"
if [ -z "$SRC_DIR" ] || [ ! -f "$SRC_DIR/naranjahub" ]; then
    echo "Error: instalador corrupto (no se encontró la aplicación)." >&2
    exit 1
fi
cp -r "$SRC_DIR"/. "$DEST"/
chmod +x "$DEST/naranjahub"

if [ "$MENU" = "1" ]; then
    MENU_DIR="$HOME_DIR/.local/share/applications"
    ICON_DIR="$HOME_DIR/.local/share/icons/hicolor/256x256/apps"
    mkdir -p "$MENU_DIR" "$ICON_DIR"
    cp "$DEST/icons/naranjahub-256.png" "$ICON_DIR/naranjahub.png" 2>/dev/null || true
    sed -e "s|@EXEC@|$DEST/naranjahub|g" \
        -e "s|@ICON@|naranjahub|g" \
        "$DEST/naranjahub.desktop.template" > "$MENU_DIR/naranjahub.desktop"
    update-desktop-database "$MENU_DIR" 2>/dev/null || true
fi

echo ""
echo "NaranjaHub v$VERSION instalado correctamente en: $DEST"
if [ "$MENU" = "1" ]; then
    echo "Acceso directo creado en el menú de aplicaciones."
fi
echo "Ejecuta: \"$DEST/naranjahub\""
exit 0

__NARANJAHUB_PAYLOAD__
