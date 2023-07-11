param (
    [string]$platform = 'windows',
    [string]$target = 'debug'
)

$hash = $env:GITHUB_SHA

if ( $platform -eq 'windows' )
{
    $file = "fg-sdl2.exe"
}
elseif ( $platform -eq 'linux' )
{
    $file = "fg-sdl2"
}
else
{
    echo "Invalid platform $platform"
    exit
}

if ( $target -eq 'debug' )
{
    
}
elseif ( $target -eq 'linux' )
{
    
}
else
{
    echo "Invalid target $target"
    exit
}

if ( $hash -eq "" )
{
    echo "Invalid hash"
    exit
}

$exe = "target/$target/$file"

$zip_name = "${platform}_${target}_x86_64.zip"

mkdir temp
cp -r assets temp/
cp "$exe" temp/
if ( $platform -eq 'windows' )
{
    cp "target/$target/SDL2.dll" temp/
}

$autoupdate_config = @{
    "current_release" = $hash
    "last_check" = 0
    "filename" = $zip_name
}

convertto-json $autoupdate_config > temp/config/autoupdate_config

$args = @{
    Path = "./temp/*"
    DestinationPath = "./$zip_name"
}
compress-archive @args
