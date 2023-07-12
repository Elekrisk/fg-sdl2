#!/usr/bin/bash

mkdir temp
curl -L --output temp/sdl2_windows.zip https://github.com/libsdl-org/SDL/releases/download/release-2.28.1/SDL2-devel-2.28.1-VC.zip
cd temp
unzip sdl2_windows.zip
cp SDL2-2.28.1/lib/x64/SDL2.{dll,lib} ..
cd ..
curl -L --output temp/sdl2_ttf_windows.zip https://github.com/libsdl-org/SDL_ttf/releases/download/release-2.20.2/SDL2_ttf-devel-2.20.2-VC.zip
cd temp
unzip sdl2_ttf_windows.zip
cp SDL2_ttf-2.20.2/lib/x64/SDL2_ttf.{dll,lib} ..
cd ..
rm -r temp
