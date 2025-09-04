#!/bin/bash
set -e

echo "Registering git hooks..."

# Check if we're in a git repository
if [ ! -d ".git" ]; then
    echo "Error: Not in a git repository"
    exit 1
fi

# Check if git-hooks directory exists
if [ ! -d "git-hooks" ]; then
    echo "Error: git-hooks directory not found"
    exit 1
fi

# Check if pre-commit.sh exists
if [ ! -f "git-hooks/pre-commit.sh" ]; then
    echo "Error: git-hooks/pre-commit.sh not found"
    exit 1
fi

# Create .git/hooks directory if it doesn't exist
mkdir -p .git/hooks

# Copy the pre-commit hook
cp git-hooks/pre-commit.sh .git/hooks/pre-commit

# Make sure it's executable
chmod +x .git/hooks/pre-commit

echo "✅ Pre-commit hook registered successfully!"
echo "The hook will now run automatically before each commit."