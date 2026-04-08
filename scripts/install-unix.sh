#!/usr/bin/env bash
set -euo pipefail

APP_NAME="hematite"
APP_VERSION="__HEMATITE_VERSION__"
INSTALL_ROOT="${HEMATITE_INSTALL_ROOT:-$HOME/.local/opt}"
ACTIVE_DIR="${INSTALL_ROOT}/${APP_NAME}"
VERSION_DIR="${INSTALL_ROOT}/${APP_NAME}-${APP_VERSION}"
BIN_DIR="${HEMATITE_BIN_DIR:-$HOME/.local/bin}"
SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

mkdir -p "$INSTALL_ROOT" "$BIN_DIR"
rm -rf "$VERSION_DIR"
rm -rf "$ACTIVE_DIR"
mkdir -p "$VERSION_DIR"

cp -a "$SRC_DIR"/. "$VERSION_DIR"/
rm -f "$VERSION_DIR/install.sh"
chmod +x "$VERSION_DIR/$APP_NAME"

ln -sfn "$VERSION_DIR" "$ACTIVE_DIR"
ln -sfn "$ACTIVE_DIR/$APP_NAME" "$BIN_DIR/$APP_NAME"

echo "Installed $APP_NAME $APP_VERSION to $ACTIVE_DIR"
echo "Linked $BIN_DIR/$APP_NAME -> $ACTIVE_DIR/$APP_NAME"

case ":${PATH}:" in
  *":$BIN_DIR:"*) ;;
  *)
    echo
    echo "$BIN_DIR is not on your PATH."
    echo "Add this to your shell profile, then open a new terminal:"
    echo "  export PATH=\"$BIN_DIR:\$PATH\""
    ;;
esac

if [[ "$(uname -s)" == "Linux" ]] && command -v ldd >/dev/null 2>&1; then
  if ldd "$ACTIVE_DIR/$APP_NAME" 2>/dev/null | grep -Eq 'lib(sonic|pcaudio)\.so[^[:space:]]* => not found'; then
    echo
    echo "Linux note: Hematite's voice stack needs libsonic and libpcaudio from your distro."
    echo "Install those libraries, then launch Hematite again."
  fi
fi

echo
echo "Open a new terminal in your project folder and run:"
echo "  $APP_NAME"
