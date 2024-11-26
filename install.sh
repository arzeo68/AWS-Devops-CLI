#!/bin/bash

# Script to build and install the Rust program "devops-cli" globally.

set -e

# Function to display an error message and exit.
function error_exit {
    echo "Error: $1"
    exit 1
}

# Check if Cargo is installed.
if ! command -v cargo &> /dev/null; then
    error_exit "Cargo is not installed. Please install Rust (https://www.rust-lang.org/tools/install)."
fi

echo "Cargo is installed."

# Ensure Cargo.toml exists in the current directory.
if [ ! -f "./Cargo.toml" ]; then
    error_exit "No Cargo.toml found in the current directory ($(pwd))."
fi

echo "Building the Rust project 'devops-cli' in the current directory ($(pwd))..."

# Build the project.
cargo build --release

echo "Build completed successfully."

# Define the expected path to the built binary.
BINARY_PATH="./target/release/devops-cli"

# Verify the built binary exists.
if [ ! -f "$BINARY_PATH" ]; then
    error_exit "Built binary not found at $BINARY_PATH."
fi

echo "Built binary found at $BINARY_PATH."

# Install the binary globally to /usr/local/bin.
echo "Installing the Rust binary 'devops-cli' globally to /usr/local/bin (requires sudo)..."
sudo cp "$BINARY_PATH" /usr/local/bin/

# Ensure the binary is executable.
sudo chmod +x /usr/local/bin/devops-cli

echo "Installation completed successfully."
echo "The program is now available globally as 'devops-cli'."