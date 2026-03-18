#!/bin/bash

echo "🔧 Installing pre-commit hooks for Confers..."

if ! command -v pre-commit &> /dev/null; then
    echo "📦 Installing pre-commit..."
    pip install pre-commit
fi

echo "📥 Installing git hooks..."
pre-commit install

echo "🔄 Updating hooks..."
pre-commit autoupdate

echo "✅ Pre-commit hooks installed successfully!"
echo ""
echo "Available hooks:"
pre-commit hooks
