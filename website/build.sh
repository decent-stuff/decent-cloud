#!/bin/bash

# Install dependencies
echo "Installing dependencies..."
npm install

# Build the wasm package
echo "Building @decent-stuff/dc-client package..."
cd ../wasm
npm install
npm run build

# Build the Next.js app
echo "Building Next.js app..."
cd ../website
npx next build