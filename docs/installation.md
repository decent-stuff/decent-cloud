# Installation Guide

This guide provides detailed instructions for installing Decent Cloud on different platforms.

## System Requirements

- **Linux**: Ubuntu 24.04 or newer
- **MacOS**: Both Intel and Apple Silicon (M1/M2/M3) supported
- **Windows**: 64-bit version supported

## Platform-Specific Instructions

### Linux (Ubuntu 24.04+)

1. Create a bin directory in your home folder:

```bash
mkdir $HOME/bin
```

2. Download and install the Decent Cloud binary:

```bash
curl -L https://github.com/decent-stuff/decent-cloud/releases/latest/download/decent-cloud-linux-amd64 -o $HOME/bin/dc
chmod +x $HOME/bin/dc
```

3. Add to PATH by adding these lines to your `~/.bashrc`:

```bash
if [ -d "$HOME/bin" ] ; then
   export PATH="$HOME/bin:$PATH"
fi
```

4. Apply the changes:

```bash
source ~/.bashrc
```

### MacOS ARM64 (M1, M2, M3)

1. Install the Decent Cloud binary:

```bash
curl -L https://github.com/decent-stuff/decent-cloud/releases/latest/download/decent-cloud-darwin-arm64 -o /usr/local/bin/dc
chmod +x /usr/local/bin/dc
```

### Windows

1. Open PowerShell as Administrator and run:

```powershell
$download_url = "https://github.com/decent-stuff/decent-cloud/releases/latest/download/decent-cloud-windows-amd64.exe"
Invoke-WebRequest "$download_url" -OutFile "dc.exe"
```

2. Add the directory containing dc.exe to your PATH environment variable.

## Verifying Installation

To verify that Decent Cloud is installed correctly, open a new terminal and run:

```bash
dc --version
```

You should see the version number of your installed Decent Cloud client.

## Troubleshooting

For common installation issues and solutions, see the [Development Guide troubleshooting section](development.md#troubleshooting).

### Getting Help

If you encounter installation-specific issues:

- 📝 [Open an Issue](https://github.com/decent-stuff/decent-cloud/issues)
- 💬 [Join Discussions](https://github.com/orgs/decent-stuff/discussions)
