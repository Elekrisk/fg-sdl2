#/usr/bin/bash

PLATFORM=$1
TARGET=$2

if [ $PLATFORM = "windows" ]
then
    FILE="fg-sdl2.exe"
elif [ $PLATFORM = "linux" ]
then
    FILE="fg-sdl2"
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

ZIP_NAME="${PLATFORM}_${TARGET}_x86_64.zip"

mkdir temp
cp -r assets temp/
cp "$EXE" temp/
if [ $PLATFORM = "windows" ]
then
    cp "target/$TARGET/SDL2.dll" temp/
fi

cd temp
zip -r "$ZIP_NAME" *
cd ..
mv "temp/$ZIP_NAME" .
rm -r temp
