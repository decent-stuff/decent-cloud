#!/usr/bin/env python3

import argparse
import os
import shutil
import subprocess
import pathlib
import sys
from glob import glob

def check_dependencies():
    dependencies = ['docker']
    missing_deps = [dep for dep in dependencies if shutil.which(dep) is None]

    if missing_deps:
        print(f"Error: Missing dependencies: {', '.join(missing_deps)}")
        if 'docker' in missing_deps:
            print("  sudo apt-get install docker.io")
        sys.exit(1)

def should_rebuild(src_files, target_file):
    # Check if the target needs to be rebuilt based on the modification time
    if not os.path.exists(target_file):
        return True
    target_mtime = os.path.getmtime(target_file)
    return any(os.path.getmtime(src) > target_mtime for src in src_files)

def run_mmdc(repo_root: pathlib.Path, input_path: pathlib.Path, output_path: pathlib.Path):
    """Build the figure using mermaid-cli docker container."""
    # docker run --rm -u `id -u`:`id -g` -v /input_dir:/data minlag/mermaid-cli -i input_file
    print(f"Building figure: {input_path.name} --> {output_path.name}")
    subprocess.check_call([
        'docker', 'run', '--rm',
        '-u', f'{os.getuid()}:{os.getgid()}',
        '-v', f'{repo_root}:/data',
        'minlag/mermaid-cli',
        '-i', str(input_path.relative_to(repo_root)),
        '-o', str(output_path.relative_to(repo_root))
    ])
    if output_path.exists():
        output_path.touch()
    else:
        print("Error: Mermaid conversion failed.")
        sys.exit(1)

def run_latexmk(repo_root: pathlib.Path, input_path: pathlib.Path, cache_dir: pathlib.Path):
    """Build the pdf using latexmk docker container."""
    print(f"Building PDF: {input_path.name}")
    cache_subdir = input_path.parent.relative_to(repo_root)
    args = [
        'docker', 'run', '--rm',
        '-u', f'{os.getuid()}:{os.getgid()}',
        '-v', f'{repo_root}:/data',
        '-v', f'{cache_dir}:/cache',
        '--workdir', f'/data/{cache_subdir}',
        'thubo/latexmk', '-pdf',
        '-view=none', str(input_path.name),
        f'-output-directory=/cache/{cache_subdir}'
    ]
    print('Running:', ' '.join(args))
    subprocess.check_call(args)
    output_pdf = cache_dir / cache_subdir / "whitepaper.pdf"
    if not output_pdf.exists():
        print("Error: Latexmk conversion failed.")
        sys.exit(1)

def build_latex(repo_root: pathlib.Path, latex_dir: pathlib.Path, cache_dir: pathlib.Path):
    """Build the pdf using latexmk docker container."""
    latex_files = list(latex_dir.glob('*.tex')) + list(latex_dir.glob('*.cls')) + list(latex_dir.glob('*.bib')) + list(latex_dir.glob('*.bbl')) + list(latex_dir.glob('*.600pk'))
    pdf_file = cache_dir / latex_dir.relative_to(repo_root) / "whitepaper.pdf"

    if should_rebuild(latex_files, pdf_file):
        run_latexmk(repo_root=repo_root, input_path=latex_dir / 'whitepaper.tex', cache_dir=cache_dir)

def build_figures(repo_root: pathlib.Path, figures_dir: pathlib.Path):
    for mmd_file in figures_dir.glob('*.mmd'):
        output_path = mmd_file.with_suffix('.png')
        if should_rebuild([mmd_file], output_path):
            run_mmdc(repo_root=repo_root, input_path=mmd_file, output_path=output_path)


def build_latex(repo_root: pathlib.Path, latex_dir: pathlib.Path, cache_dir: pathlib.Path):
    latex_files = list(latex_dir.glob('*.tex')) + list(latex_dir.glob('*.cls')) + list(latex_dir.glob('*.bib')) + list(latex_dir.glob('*.bbl')) + list(latex_dir.glob('*.600pk'))
    pdf_file = cache_dir / latex_dir.relative_to(repo_root) / "whitepaper.pdf"

    if should_rebuild(latex_files, pdf_file):
        run_latexmk(repo_root=repo_root, input_path=latex_dir / 'whitepaper.tex', cache_dir=cache_dir)

def get_repo_root():
    try:
        repo_root = subprocess.check_output(['git', 'rev-parse', '--show-toplevel']).strip().decode('utf-8')
        return pathlib.Path(repo_root)
    except subprocess.CalledProcessError:
        print("Error: Not a git repository.")
        sys.exit(1)

def clean(cache_dir: pathlib.Path):
    if cache_dir.exists():
        shutil.rmtree(cache_dir)

def parse_args():
    parser = argparse.ArgumentParser()
    parser.add_argument('--clean', action='store_true')
    return parser.parse_args()

def main():
    check_dependencies()

    repo_root = get_repo_root()
    cache_dir = repo_root / "build"

    args = parse_args()
    if args.clean:
        clean(cache_dir=cache_dir)

    cache_dir.mkdir(parents=True, exist_ok=True)

    build_figures(repo_root, repo_root / "docs" / "whitepaper" / "figures")
    build_latex(repo_root, repo_root / "docs" / "whitepaper", cache_dir)

    output_pdf = cache_dir / "docs" / "whitepaper" / "whitepaper.pdf"
    final_output_path = repo_root / "docs" / "whitepaper" / "whitepaper.pdf"
    if output_pdf.exists():
        shutil.move(str(output_pdf), str(final_output_path))
        print(f"PDF written to {final_output_path}")
    else:
        print("Error: PDF was not generated.")

if __name__ == '__main__':
    main()
