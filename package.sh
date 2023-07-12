#/usr/bin/bash

PLATFORM=$1
TARGET=$2

if [ $PLATFORM = "windows" ]
then
    FILE="fg-sdl2.exe"
    TARGET_PREFIX=""
elif [ $PLATFORM = "linux" ]
then
    FILE="fg-sdl2"
    TARGET_PREFIX="x86_64-pc-windows-gnu"
else
    echo "Invalid platform $PLATFORM"
    exit
fi

if [ $TARGET = "debug" ]
then
    EXE="target/debug/$FILE"
elif [ $TARGET = "release" ]
then
    EXE="target/release/$FILE"
else
    echo "Invalid target $TARGET"
    exit
fi

if [ $HASH = "" ]
then
    echo "Invalid hash"
    exit
fi

ZIP_NAME="${PLATFORM}_${TARGET}_x86_64.zip"

mkdir temp
cp -r assets temp/
cp "$EXE" temp/
if [ $PLATFORM = "windows" ]
then
    cp SDL2.dll SDL2_ttf.dll temp/
fi

mkdir temp/config
echo "{\"current_release\" = \"$HASH\", \"last_check\" = 0, \"filename\" = \"$ZIP_NAME\"}" > temp/config/autoupdate.json

cd temp
zip -r "$ZIP_NAME" *
cd ..
mv "temp/$ZIP_NAME" .
rm -r temp
