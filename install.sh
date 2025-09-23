#!/bin/bash

# Snyfter3 Installation Script
# This script installs Snyfter3 for easy system-wide access

set -e

echo "🚀 Installing Snyfter3..."

# Get the directory of this script
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Build release version
echo "📦 Building Snyfter3..."
cd "$SCRIPT_DIR"
cargo build --release

# Create local bin directory if it doesn't exist
mkdir -p "$HOME/.local/bin"

# Copy the launcher script
echo "📝 Installing launcher..."
cat > "$HOME/.local/bin/snyfter3" << 'EOF'
#!/bin/bash
exec /Users/jack/snyfter3/target/release/snyfter3 "$@"
EOF

chmod +x "$HOME/.local/bin/snyfter3"

# Check if ~/.local/bin is in PATH
if ! echo "$PATH" | grep -q "$HOME/.local/bin"; then
    echo ""
    echo "⚠️  Adding ~/.local/bin to PATH..."

    # Detect shell and add to appropriate config file
    if [[ "$SHELL" == *"zsh"* ]]; then
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
        echo "✅ Added to ~/.zshrc - Please run: source ~/.zshrc"
    elif [[ "$SHELL" == *"bash"* ]]; then
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
        echo "✅ Added to ~/.bashrc - Please run: source ~/.bashrc"
    else
        echo "Please add the following to your shell configuration:"
        echo 'export PATH="$HOME/.local/bin:$PATH"'
    fi
fi

echo ""
echo "✨ Snyfter3 has been installed successfully!"
echo ""
echo "📚 Quick Start Guide:"
echo "  - Run 'snyfter3' to start the application"
echo "  - Press Ctrl+N to create a new note"
echo "  - Press / to search"
echo "  - Press Enter to edit a note"
echo "  - Press Ctrl+Q to quit"
echo ""
echo "Happy note-taking! 📝"