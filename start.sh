#!/bin/bash

command_exists () {
  command -v "$1" >/dev/null 2>&1
}

if command_exists cargo; then
    echo "'cargo' is installed"
    cargo --version
else
    echo "'cargo' is not installed"
    echo "Detailed installation instructions can be found at https://rust-lang.org/learn/get-started/"
    echo "Alternatively, if you trust this program, you can install it with 'curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh'"
    exit 1
fi

if [ -d "./release" ]; then 
  echo "Previous build found..."
  echo "Cleaning up..."

  rm -rf ./release
fi

echo "Starting compilation..."
cargo build --release
echo "Compilation complete..."

mkdir release
mkdir ./release/configs
mv target/release/scrollr_backend ./release
cp -r ./configs ./release

if [ -f .env ]; then
  cp .env ./release/.env
else
  echo "No .env found..."
  cp .env.example .env
  echo ".env generated from .env.example, please populate this with valid info before continuing"
  exit 2
fi

cd release 
echo "Starting application from ./release"
./scrollr_backend
cd ..