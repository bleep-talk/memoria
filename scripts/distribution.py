"""Build native release archives and a Homebrew formula. Never publish them."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "modules/batch-recovery-boundary/cli/Cargo.toml"
FEATURES = "git2/vendored-libgit2,git2/vendored-openssl"
TARGETS = {
    "aarch64-apple-darwin": ("on_macos", "on_arm"),
    "x86_64-apple-darwin": ("on_macos", "on_intel"),
    "aarch64-unknown-linux-gnu": ("on_linux", "on_arm"),
    "x86_64-unknown-linux-gnu": ("on_linux", "on_intel"),
}


def run(*args, **kwargs):
    return subprocess.run(args, cwd=ROOT, check=True, text=True, **kwargs)


def version():
    return tomllib.loads(MANIFEST.read_text())["package"]["version"]


def archive_name(target):
    return f"memoria-{version()}-{target}.tar.gz"


def check_linkage(binary, target):
    if target.endswith("apple-darwin"):
        output = run("otool", "-L", str(binary), capture_output=True).stdout
        libraries = [line.strip().split(" ")[0] for line in output.splitlines()[1:]]
        unsafe = [lib for lib in libraries if not lib.startswith(("/usr/lib/", "/System/Library/"))]
    else:
        output = run("ldd", str(binary), capture_output=True).stdout
        allowed = re.compile(r"^(linux-vdso|ld-linux|lib(c|m|gcc_s|pthread|dl|rt|util)\.)")
        unsafe = [line.strip() for line in output.splitlines()
                  if line.strip() and ("not found" in line or not allowed.match(Path(line.split()[0]).name))]
    if unsafe:
        raise ValueError(f"Release links to non-system libraries: {unsafe}")


def smoke(binary):
    with tempfile.TemporaryDirectory(prefix="memoria-smoke-") as temp:
        repo = str(Path(temp) / "memory")
        run(str(binary), "init", repo, stdout=subprocess.DEVNULL)
        args = (str(binary), "--repo", repo, "--json")
        run(*args, "write", "system/user.md", "--expect", "absent",
            input="The user prefers short answers.\n", stdout=subprocess.DEVNULL)
        context = json.loads(run(*args, "context", capture_output=True).stdout)
        assert "short answers" in context["data"]
        run(*args, "commit", "Remember preference", stdout=subprocess.DEVNULL)


def collect_licenses(destination, env, target):
    metadata = json.loads(run("cargo", "metadata", "--manifest-path", str(MANIFEST),
        "--format-version", "1", "--locked", "--features", FEATURES,
        "--filter-platform", target, env=env, capture_output=True).stdout)
    notices = destination / "THIRD_PARTY_LICENSES"
    notices.mkdir()
    index = []
    for package in metadata["packages"]:
        if not package["source"]:
            continue
        index.append(f'{package["name"]} {package["version"]}: {package["license"] or "see license files"}')
        source = Path(package["manifest_path"]).parent
        # Include nested notices for native code vendored by the Rust crates.
        candidates = [p for p in source.rglob("*") if p.is_file() and
                      p.name.upper().split(".")[0].startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE"))]
        if package["license_file"]:
            candidates.append(source / package["license_file"])
        for path in set(candidates):
            relative = path.relative_to(source)
            output = notices / f'{package["name"]}-{package["version"]}' / relative
            output.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, output)
    (notices / "INDEX.txt").write_text("\n".join(sorted(index)) + "\n")


def build(output):
    env = os.environ.copy()
    for key in ("OPENSSL_DIR", "OPENSSL_LIB_DIR", "OPENSSL_INCLUDE_DIR", "OPENSSL_NO_VENDOR",
                "LIBSSH2_SYS_USE_PKG_CONFIG", "LIBGIT2_SYS_USE_PKG_CONFIG"):
        env.pop(key, None)
    env["LIBZ_SYS_STATIC"] = "1"
    host = run("rustc", "-vV", env=env, capture_output=True).stdout
    target = next(line.removeprefix("host: ") for line in host.splitlines() if line.startswith("host: "))
    if target not in TARGETS:
        raise ValueError(f"Unsupported release target: {target}")
    run("cargo", "build", "--manifest-path", str(MANIFEST), "--locked", "--release",
        "--target", target, "--features", FEATURES, env=env)
    metadata = json.loads(run("cargo", "metadata", "--format-version", "1", "--no-deps",
                             env=env, capture_output=True).stdout)
    binary = Path(metadata["target_directory"]) / target / "release/memoria"
    check_linkage(binary, target)
    smoke(binary)
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="memoria-package-") as temp:
        package = Path(temp) / archive_name(target).removesuffix(".tar.gz")
        (package / "bin").mkdir(parents=True)
        shutil.copy2(binary, package / "bin/memoria")
        shutil.copytree(ROOT / "skills/memoria", package / "share/memoria/skill")
        (package / "docs").mkdir()
        readme = (ROOT / "README.md").read_text()
        readme = readme.replace("(skills/memoria/SKILL.md)", "(share/memoria/skill/SKILL.md)")
        readme = readme.replace("(modules/batch-recovery-boundary/USAGE.md)", "(docs/CLI.md)")
        (package / "README.md").write_text(readme)
        shutil.copy2(ROOT / "LICENSE", package / "LICENSE")
        shutil.copy2(ROOT / "docs/STATUS.md", package / "docs/STATUS.md")
        shutil.copy2(ROOT / "modules/batch-recovery-boundary/USAGE.md", package / "docs/CLI.md")
        collect_licenses(package, env, target)
        archive = output / archive_name(target)
        with tarfile.open(archive, "w:gz") as tar:
            tar.add(package, arcname=package.name)
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    archive.with_name(archive.name + ".sha256").write_text(f"{digest}  {archive.name}\n")
    print(archive)


def formula(artifacts, output):
    checksums = {}
    for target in TARGETS:
        archive = artifacts / archive_name(target)
        # A formula is emitted only when every platform archive exists.
        checksums[target] = hashlib.sha256(archive.read_bytes()).hexdigest()
    lines = ["class Memoria < Formula", '  desc "Simple memory for AI agents"',
             '  homepage "https://github.com/bleep-talk/memoria"',
             f'  version "{version()}"', '  license "MIT"', ""]
    for os_block in ("on_macos", "on_linux"):
        lines.append(f"  {os_block} do")
        if os_block == "on_macos":
            lines.append("    depends_on macos: :ventura")
        for target, (operating_system, architecture) in TARGETS.items():
            if operating_system != os_block:
                continue
            lines.extend([f"    {architecture} do",
                f'      url "https://github.com/bleep-talk/memoria/releases/download/v{version()}/{archive_name(target)}"',
                f'      sha256 "{checksums[target]}"', "    end"])
        lines.extend(["  end", ""])
    lines.extend([
        "  def install", '    bin.install "bin/memoria"',
        '    (share/"memoria").install "share/memoria/skill"',
        '    doc.install "README.md", "LICENSE", "docs", "THIRD_PARTY_LICENSES"',
        '    inreplace doc/"README.md", "(share/memoria/skill/SKILL.md)", "(../../memoria/skill/SKILL.md)"',
        "  end", "",
        "  test do", '    system bin/"memoria", "init", testpath/"memory"',
        '    output = shell_output("#{bin}/memoria --repo #{testpath}/memory --json context")',
        '    assert_equal 1, JSON.parse(output).fetch("schema_version")',
        '    assert_path_exists share/"memoria/skill/SKILL.md"', "  end", "end", ""])
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text("\n".join(lines))
    print(output)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    package = commands.add_parser("build")
    package.add_argument("--output", type=Path, default=ROOT / "dist/releases")
    tap = commands.add_parser("formula")
    tap.add_argument("--artifacts", type=Path, default=ROOT / "dist/releases")
    tap.add_argument("--output", type=Path, default=ROOT / "dist/homebrew-tap/Formula/memoria.rb")
    args = parser.parse_args()
    if args.command == "build":
        build(args.output.resolve())
    else:
        formula(args.artifacts.resolve(), args.output.resolve())
