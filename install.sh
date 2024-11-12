#!/usr/bin/env bash

if [ "$EUID" -ne 0 ]
  then echo "Please run as root"
  exit
fi

PROGRAM_NAME="devops-cli"

cargo build --release
sudo cp target/release/$PROGRAM_NAME /usr/local/bin/$PROGRAM_NAME

Echo "Install successfully"