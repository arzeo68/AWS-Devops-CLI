#!/usr/bin/env bash

PROGRAM_NAME="devops-cli"

cargo build --release
sudo cp target/release/$PROGRAM_NAME /usr/local/bin/$PROGRAM_NAME
