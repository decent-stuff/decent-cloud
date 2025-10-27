#!/usr/bin/env python3
"""Set up Python environment for the project."""

import os
import sys
import subprocess
import venv
from pathlib import Path

def check_python_version():
    """Check if Python version meets requirements."""
    if sys.version_info < (3, 10):
        print("Error: Python 3.10 or higher is required")
        return False
    print(f"Python {sys.version.split()[0]} detected")
    return True

def create_venv():
    """Create virtual environment if it doesn't exist."""
    venv_path = Path(".venv")
    if venv_path.exists():
        print("Virtual environment already exists")
        return True

    print("Creating virtual environment...")
    try:
        venv.create(venv_path, with_pip=True)
        print("Virtual environment created successfully")
        return True
    except Exception as e:
        print(f"Error creating virtual environment: {e}")
        return False

def install_dependencies():
    """Install project dependencies."""
    if not Path("pyproject.toml").exists():
        print("No pyproject.toml found, skipping dependency installation")
        return True

    print("Installing dependencies...")

    # Determine pip command based on platform
    if sys.platform == "win32":
        pip_cmd = [".venv\\Scripts\\pip.exe"]
    else:
        pip_cmd = ["./.venv/bin/pip"]

    try:
        # Install in editable mode with dev dependencies
        subprocess.check_call(pip_cmd + ["install", "-e", ".[dev]"])
        print("Dependencies installed successfully")
        return True
    except subprocess.CalledProcessError as e:
        print(f"Error installing dependencies: {e}")
        return False

def print_activation_instructions():
    """Print instructions for activating the virtual environment."""
    print("\nTo activate the virtual environment:")
    if sys.platform == "win32":
        print("  .venv\\Scripts\\activate")
    else:
        print("  source .venv/bin/activate")
    print("\nOr use the Python directly:")
    if sys.platform == "win32":
        print("  .venv\\Scripts\\python.exe")
    else:
        print("  .venv/bin/python3")

def main():
    """Main setup function."""
    print("Setting up Python environment for decent-cloud...")

    # Change to project root if we're in scripts directory
    if Path.cwd().name == "scripts":
        os.chdir("..")

    if not check_python_version():
        return 1

    if not create_venv():
        return 1

    if not install_dependencies():
        return 1

    print_activation_instructions()
    print("\nSetup completed successfully!")
    return 0

if __name__ == "__main__":
    sys.exit(main())