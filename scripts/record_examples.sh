#!/bin/bash
# Script to record TUI examples using VHS (https://github.com/charmbracelet/vhs)
# Install VHS: brew install vhs
# Usage: ./scripts/record_examples.sh [example_name]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
OUTPUT_DIR="$PROJECT_DIR/screenshots"
TAPES_DIR="$PROJECT_DIR/tapes"

# Create output directories
mkdir -p "$OUTPUT_DIR"
mkdir -p "$TAPES_DIR"

# Check if VHS is installed
if ! command -v vhs &> /dev/null; then
    echo "Error: VHS is not installed"
    echo "Install it with: brew install vhs"
    echo "Or visit: https://github.com/charmbracelet/vhs"
    exit 1
fi

# Function to generate VHS tape file
generate_tape() {
    local example_name=$1
    local duration=${2:-5}
    local tape_file="$TAPES_DIR/${example_name}.tape"

    cat > "$tape_file" <<EOF
Output "$OUTPUT_DIR/${example_name}.gif"
Set FontSize 14
Set Width 1200
Set Height 800
Set Theme "Catppuccin Mocha"

Type "cargo run --example ${example_name}"
Enter
Sleep ${duration}s
Ctrl+C
Sleep 1s
EOF

    echo "$tape_file"
}

# Function to record an example
record_example() {
    local example_name=$1
    local duration=${2:-5}

    echo "Recording example: $example_name"

    # Generate tape file
    tape_file=$(generate_tape "$example_name" "$duration")

    # Run VHS
    cd "$PROJECT_DIR"
    vhs "$tape_file"

    echo "✓ Recorded: $OUTPUT_DIR/${example_name}.gif"
}

# If specific example is provided, record only that one
if [ $# -gt 0 ]; then
    record_example "$1" "${2:-5}"
    exit 0
fi

# Otherwise, record all examples
echo "Recording all examples..."
echo

# Simple examples (5 seconds)
record_example "counter" 5
record_example "hello" 3
record_example "spinner" 5
record_example "progress" 5

# Interactive examples (8 seconds)
record_example "input" 8
record_example "list" 8
record_example "form" 10

# Complex examples (10 seconds)
record_example "dashboard" 10
record_example "todo" 10

echo
echo "All examples recorded successfully!"
echo "Screenshots saved to: $OUTPUT_DIR"
