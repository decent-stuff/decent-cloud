#!/usr/bin/env python3
"""Install pocket-ic-server for testing."""

import os
import sys
import subprocess
import urllib.request
import json
import tempfile
import shutil
from pathlib import Path

def get_latest_release():
    """Get the latest release information from GitHub API."""
    url = "https://api.github.com/repos/dfinity/pocket-ic/releases/latest"
    try:
        with urllib.request.urlopen(url) as response:
            data = json.loads(response.read().decode())
            return data
    except Exception as e:
        print(f"Error fetching release info: {e}")
        return None

def download_pocket_ic():
    """Download pocket-ic binary for the current platform."""
    release_info = get_latest_release()
    if not release_info:
        print("Failed to fetch release information")
        return False

    # Determine the correct asset for current platform
    platform = sys.platform
    machine = os.uname().machine.lower()

    asset_name = None
    if platform == "linux":
        if machine in ["x86_64", "amd64"]:
            asset_name = "pocket-ic-x86_64-linux"
        elif machine in ["aarch64", "arm64"]:
            asset_name = "pocket-ic-aarch64-linux"
    elif platform == "darwin":  # macOS
        if machine in ["x86_64", "amd64"]:
            asset_name = "pocket-ic-x86_64-darwin"
        elif machine in ["aarch64", "arm64"]:
            asset_name = "pocket-ic-aarch64-darwin"
    elif platform == "win32" or platform == "cygwin":
        if machine in ["x86_64", "amd64"]:
            asset_name = "pocket-ic-x86_64-windows.exe"

    if not asset_name:
        print(f"Unsupported platform: {platform}-{machine}")
        return False

    # Find the asset in release data
    asset_url = None
    for asset in release_info["assets"]:
        if asset["name"] == asset_name:
            asset_url = asset["browser_download_url"]
            break

    if not asset_url:
        print(f"Could not find asset: {asset_name}")
        return False

    # Download the binary
    print(f"Downloading {asset_name}...")
    try:
        with tempfile.NamedTemporaryFile(delete=False) as tmp_file:
            with urllib.request.urlopen(asset_url) as response:
                shutil.copyfileobj(response, tmp_file)

            # Make executable and move to final location
            bin_dir = Path.home() / ".local" / "bin"
            bin_dir.mkdir(parents=True, exist_ok=True)

            pocket_ic_path = bin_dir / "pocket-ic"
            shutil.move(tmp_file.name, pocket_ic_path)
            pocket_ic_path.chmod(0o755)

            print(f"Installed pocket-ic to {pocket_ic_path}")
            print("\nAdd ~/.local/bin to your PATH if you haven't already:")
            print("export PATH=\"$HOME/.local/bin:$PATH\"")

            return True
    except Exception as e:
        print(f"Error downloading pocket-ic: {e}")
        return False

def main():
    """Main installation function."""
    print("Installing pocket-ic server...")

    # Check if pocket-ic is already available
    if shutil.which("pocket-ic"):
        print("pocket-ic is already installed")
        return 0

    # Download and install
    if download_pocket_ic():
        print("Installation completed successfully!")
        return 0
    else:
        print("Installation failed!")
        return 1

if __name__ == "__main__":
    sys.exit(main())