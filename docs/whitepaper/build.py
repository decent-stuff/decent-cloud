#!/usr/bin/env python3

import os
import shutil
import subprocess
import pathlib

def generate_genrule(file_name):
    base_name = os.path.splitext(file_name)[0]
    return f"""
genrule(
    name = "{base_name}",
    srcs = ["{file_name}"],
    out = "{base_name}.png",
    cmd = 'mmdc -i "{file_name}" -o "$OUT"',
    visibility = ["PUBLIC"],
)
"""

os.chdir(os.path.dirname(__file__))
base_dir = pathlib.Path(subprocess.check_output("git rev-parse --show-toplevel".split()).decode("utf8").strip())

def main():
    mmd_files = [f for f in os.listdir('figures') if f.endswith('.mmd')]
    with open('figures/BUCK', 'w') as buck_file:
        for mmd_file in mmd_files:
            buck_file.write(generate_genrule(mmd_file))

    # Copy back the png figures
    subprocess.check_call(["buck2", "build", "docs/whitepaper/figures:"], cwd=base_dir)
    targets = subprocess.check_output(["buck2", "targets", "--show-output", "//docs/whitepaper/figures:"]).decode("utf8").splitlines()
    targets = [t for t in targets if t.endswith(".png")]
    if not targets:
        print("No .png targets found")
    else:
        for t in targets:
            rule, target = t.split()
            target = pathlib.Path(target)
            print("PNG written to %s" % (base_dir / "docs/whitepaper/figures/" / target.name))
            shutil.copy(base_dir / target, base_dir / "docs/whitepaper/figures/")
    subprocess.check_call(["buck2", "build", "//docs/whitepaper"], cwd=base_dir)
    # Copy back the final PDF
    targets = subprocess.check_output(["buck2", "targets", "--show-output", "//docs/whitepaper"]).decode("utf8").splitlines()
    targets = [t for t in targets if t.endswith(".pdf")]
    if not targets:
        print("No .pdf targets found")
    else:
        for t in targets:
            rule, target = t.split()
            target = pathlib.Path(target)
            print("PDF written to buck-out/%s" % (os.path.basename(target)))
            shutil.copy(base_dir / target, base_dir / "buck-out/")

if __name__ == '__main__':
    main()
