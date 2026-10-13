#!/usr/bin/env bash
set -euo pipefail

# Packages a built GPUI app binary into a preview archive plus a SHA-256 file.
#
# The GPUI front end ships straight out of cargo today: there is no installer
# or app bundle yet, so preview channels distribute the binary together with
# the side-by-side DLLs on Windows. This script only serves the GPUI preview
# release (release-gpui.yml).
#
# Usage:
#   package-gpui-release.sh \
#     --platform <macos-aarch64|macos-x86_64|windows-x64|linux-x64> \
#     --version <version> \
#     --binary-dir <directory-with-the-built-binary> \
#     [--binary-name <name>] \
#     --output <dist-directory>

platform=""
version=""
binary_dir=""
binary_name=""
output_dir=""

while (( $# > 0 )); do
    case "$1" in
        --platform)
            platform="$2"
            shift 2
            ;;
        --version)
            version="$2"
            shift 2
            ;;
        --binary-dir)
            binary_dir="$2"
            shift 2
            ;;
        --binary-name)
            binary_name="$2"
            shift 2
            ;;
        --output)
            output_dir="$2"
            shift 2
            ;;
        -h|--help)
            sed -n '2,20p' "$0"
            exit 0
            ;;
        *)
            printf 'Unknown argument: %s\n' "$1" >&2
            exit 2
            ;;
    esac
done

if [[ -z "$platform" || -z "$version" || -z "$binary_dir" || -z "$output_dir" ]]; then
    printf 'Usage: %s --platform <platform> --version <version> --binary-dir <dir> --output <dir>\n' "$0" >&2
    exit 2
fi

if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    printf 'Version must use the form MAJOR.MINOR.PATCH: %s\n' "$version" >&2
    exit 2
fi

case "$platform" in
    macos-aarch64|macos-x86_64|linux-x64)
        default_binary_name="Lithe"
        archive_kind="tar.gz"
        ;;
    windows-x64)
        default_binary_name="Lithe.exe"
        archive_kind="zip"
        ;;
    *)
        printf 'Unsupported platform: %s\n' "$platform" >&2
        exit 2
        ;;
esac

binary_name="${binary_name:-$default_binary_name}"

if [[ ! -d "$binary_dir" ]]; then
    printf 'Binary directory does not exist: %s\n' "$binary_dir" >&2
    exit 1
fi

binary_path="$binary_dir/$binary_name"
if [[ ! -f "$binary_path" ]]; then
    printf 'Expected binary not found: %s\n' "$binary_path" >&2
    exit 1
fi

archive_stem="Lithe-GPUI-${version}-${platform}"
stage_dir="$output_dir/stage/$archive_stem"
rm -rf "$stage_dir"
mkdir -p "$stage_dir"

cp "$binary_path" "$stage_dir/"
chmod +x "$stage_dir/$binary_name"

if [[ "$platform" == windows-* ]]; then
    # cargo places the side-by-side assemblies next to the executable; signing
    # and running both require them, so the archive keeps the flat layout.
    shopt -s nullglob
    for dll in "$binary_dir"/*.dll; do
        cp "$dll" "$stage_dir/"
    done
    shopt -u nullglob
fi

mkdir -p "$output_dir"
archive_path="$output_dir/$archive_stem.$archive_kind"
rm -f "$archive_path"

case "$archive_kind" in
    tar.gz)
        tar -czf "$archive_path" -C "$output_dir/stage" "$archive_stem"
        ;;
    zip)
        # Git Bash on Windows runners ships GNU tar, which cannot write zip
        # archives; Compress-Archive is always present on the runner image.
        powershell.exe -NoProfile -NonInteractive -Command \
            "Compress-Archive -Path '$stage_dir' -DestinationPath '$archive_path' -Force" \
            > /dev/null
        if [[ ! -f "$archive_path" ]]; then
            printf 'Compress-Archive did not produce %s\n' "$archive_path" >&2
            exit 1
        fi
        ;;
esac

(cd "$output_dir" && shasum -a 256 "$archive_stem.$archive_kind" > "$archive_stem.$archive_kind.sha256")
rm -rf "$output_dir/stage"

printf '%s\n' "$archive_path"
