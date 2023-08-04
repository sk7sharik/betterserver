#!/bin/sh

TARGETDIR="disaster-server"
DEPS=( "git" "cargo" "rustc" "nano")

set -e
echo ":: Checking for dependencies..."

for cmd in "${DEPS[@]}" 
do
    echo "  * Checking for $cmd..."
    RESULT="$(command -v "$cmd")"

    if test -z "$RESULT"
    then
        echo "$cmd doesn't seem to be installed or is not in PATH."
        exit
    fi
done

if ! test -d "$TARGETDIR"
then
    echo ":: Cloning repository..."
    git clone https://github.com/teamexeempire/betterserver.git "$TARGETDIR"
    cd "$TARGETDIR"

    printf ":: Would you like to do additional configuration? (y/n): "
    read CHOICE

    if test "$CHOICE" -eq "y"
    then
        nano "config.toml"
    else
        echo ":: No additional changes were made!"
    fi 
else
    cd "$TARGETDIR"
    echo ":: Updating repository..."
    git pull
fi

echo ":: Building & starting the server"
cargo run --release