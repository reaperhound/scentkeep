#!/usr/bin/env bash
set -euo pipefail

bundle_dir=src-tauri/target/release/bundle
deb_files=("$bundle_dir"/deb/*.deb)
if (( ${#deb_files[@]} != 1 )) || [[ ! -f ${deb_files[0]} ]]; then
  echo "Expected exactly one Debian bundle in $bundle_dir/deb" >&2
  exit 1
fi

version=$(node -p 'require("./package.json").version')
arch=$(dpkg-deb -f "${deb_files[0]}" Architecture)
case "$arch" in
  amd64) arch=x86_64 ;;
  arm64) arch=aarch64 ;;
  *) echo "Unsupported Debian architecture: $arch" >&2; exit 1 ;;
esac

work_dir=$(mktemp -d)
trap 'rm -rf "$work_dir"' EXIT
dpkg-deb -x "${deb_files[0]}" "$work_dir"

size=$(du -sb "$work_dir" | cut -f1)
cat > "$work_dir/.PKGINFO" <<EOF
pkgname = scentkeep
pkgbase = scentkeep
pkgver = $version-1
pkgdesc = Scentkeep desktop app
builddate = $(date +%s)
packager = Scentkeep release workflow
size = $size
arch = $arch
depend = gtk3
depend = webkit2gtk-4.1
depend = libayatana-appindicator
EOF

mkdir -p "$bundle_dir/arch"
tar --zstd -cf "$bundle_dir/arch/scentkeep-$version-1-$arch.pkg.tar.zst" -C "$work_dir" .
