#!/bin/sh
set -e

# Detect OS
OS_TYPE=$(uname -s)

echo "Building Corallium..."
cargo build --release --quiet

# Find the binary (it might have different names on different platforms)
if [ -f "./target/release/Corallium" ]; then
    BINARY="./target/release/Corallium"
elif [ -f "./target/release/Corallium.exe" ]; then
    BINARY="./target/release/Corallium.exe"
else
    echo "ERROR: Could not find compiled binary in ./target/release/"
    exit 1
fi

case "$OS_TYPE" in
    Linux)
        echo "Installing for Linux..."
        sudo mkdir -p /opt/Corallium
        sudo cp "$BINARY" /bin/corallium
        sudo mkdir -p /opt/Corallium/src
        sudo cp -r ./src/std /opt/Corallium/src
        echo "Installed to /bin/corallium and /opt/Corallium/src/std"
        ;;
    Darwin)
        echo "Installing for macOS..."
        mkdir -p /usr/local/opt/Corallium/src
        mkdir -p /usr/local/bin
        cp "$BINARY" /usr/local/bin/corallium
        chmod +x /usr/local/bin/corallium
        cp -r ./src/std /usr/local/opt/Corallium/src
        echo "Installed to /usr/local/bin/corallium and /usr/local/opt/Corallium/src/std"
        echo ""
        echo "If /usr/local/bin is not in your PATH, add this to your shell profile:"
        echo "  export PATH=\"/usr/local/bin:\$PATH\""
        ;;
    *)
        echo "Unsupported OS: $OS_TYPE"
        echo "Please install manually or use install.bat on Windows"
        exit 1
        ;;
esac
