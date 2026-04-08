#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

version="$(sed -nE 's/^version\s*=\s*"([^"]+)".*/\1/p' Cargo.toml | head -n 1)"
if [[ -z "$version" ]]; then
  echo "Could not determine package version from Cargo.toml." >&2
  exit 1
fi

platform_raw="$(uname -s)"
arch="$(uname -m)"
case "$platform_raw" in
  Linux) platform="linux" ;;
  Darwin) platform="macos" ;;
  *)
    echo "Unsupported host platform: $platform_raw" >&2
    exit 1
    ;;
esac

dist_root="$repo_root/dist/$platform"
release_dir="$repo_root/target/release"
bundle_name="Hematite-$version-$platform-$arch-portable"
bundle_dir="$dist_root/$bundle_name"
archive_path="$dist_root/$bundle_name.tar.gz"
binary_path="$release_dir/hematite"
readme_out="$bundle_dir/README.txt"
install_template="$repo_root/scripts/install-unix.sh"
install_out="$bundle_dir/install.sh"

echo "Building release binary (v$version) for $platform-$arch..."
cargo build --release

if [[ ! -f "$binary_path" ]]; then
  echo "Required release artifact missing: $binary_path" >&2
  exit 1
fi

mkdir -p "$dist_root"
rm -rf "$bundle_dir"
mkdir -p "$bundle_dir"

cp -a "$binary_path" "$bundle_dir/"
chmod +x "$bundle_dir/hematite"

shopt -s nullglob
for pattern in "$release_dir"/*.so "$release_dir"/*.so.* "$release_dir"/*.dylib; do
  cp -a "$pattern" "$bundle_dir/"
done
for framework in "$release_dir"/*.framework; do
  cp -a "$framework" "$bundle_dir/"
done
shopt -u nullglob

sed "s/__HEMATITE_VERSION__/$version/g" "$install_template" > "$install_out"
chmod +x "$install_out"

cat > "$readme_out" <<EOF
Hematite $version
=================

What this is:
- Hematite is a local AI coding harness and terminal CLI for LM Studio.
- This archive contains the Unix release bundle for $platform-$arch.

Install:
1. Extract this archive.
2. Run: ./install.sh
3. If ~/.local/bin is not already on PATH, add:
   export PATH="\$HOME/.local/bin:\$PATH"
4. Open a new terminal, cd into your project, and run: hematite

Manual run without installing:
- Launch directly from this folder: ./hematite

Linux note:
- Hematite's voice stack links libsonic and libpcaudio from the host distro.
- If the binary fails to start on Linux, install those libraries and retry.

Before running:
1. Install LM Studio (https://lmstudio.ai).
2. Download and load a coding model. Recommended: Qwen/Qwen3.5-9B Q4_K_M.
3. Optionally load nomic-embed-text-v2 alongside it for semantic search.
4. Start LM Studio's local server on port 1234.

More info: https://github.com/undergroundrap/hematite-cli
EOF

rm -f "$archive_path"
tar -C "$dist_root" -czf "$archive_path" "$bundle_name"

archive_mb="$(du -m "$archive_path" | awk '{print $1}')"
binary_mb="$(du -m "$bundle_dir/hematite" | awk '{print $1}')"

echo
echo "Portable bundle ready: $bundle_dir"
echo "  hematite      - ${binary_mb}MB"
echo "Portable archive: $archive_path (${archive_mb}MB)"
