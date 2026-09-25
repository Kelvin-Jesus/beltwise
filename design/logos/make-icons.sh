#!/usr/bin/env bash
# Builds every brand asset from the logo sheet (design/logos/src/beltwise-logo-sheet.webp):
# PWA and iOS icons, the favicon set, the title-screen mark and wordmark, and the README
# banner. Needs ImageMagick 7. Run from anywhere: ./design/logos/make-icons.sh
set -euo pipefail
cd "$(dirname "$0")/src"
PUB=../../../web/public
TILE='#090f16'  # background of the app icon in the sheet
PAGE='#0e141b'  # background of the sheet around the wordmark
OUT=(-depth 8 -strip -define png:compression-level=9)

magick beltwise-logo-sheet.webp -depth 8 sheet.png

# The symbol, cut from the large app icon with a 10 px margin of tile colour; its edges are
# feathered and near-background noise flattened, so it sits seamlessly on a solid tile.
magick sheet.png -crop 346x283+73+203 +repage -fuzz 4% -fill "$TILE" -opaque "$TILE" mark.png
magick -size 346x283 xc:black -fill white -draw "rectangle 6,6 339,276" -blur 0x4 mask.png
magick mark.png mask.png -alpha off -compose CopyOpacity -composite mark-soft.png

# Tile with the symbol at `width` px, centred: master for square icons.
tile() { # size width out
  magick -size "$1x$1" xc:"$TILE" \( mark-soft.png -filter Lanczos -resize "$2x" -unsharp 0x0.8+0.5+0 \) \
    -gravity center -composite -fuzz 3% -fill "$TILE" -opaque "$TILE" "${OUT[@]}" "$3"
}
# Rounded-corner copy (transparent corners), for browsers that show icons as-is.
round() { # in size radius out
  magick "$1" -filter Lanczos -resize "$2x$2" \
    \( -size "$2x$2" xc:black -fill white -draw "roundrectangle 0,0 $(($2 - 1)),$(($2 - 1)) $3,$3" \) \
    -alpha off -compose CopyOpacity -composite "${OUT[@]}" "$4"
}

tile 1024 840 master.png     # symbol ~82% wide
tile 1024 680 maskable.png   # inside the 80% safe zone of maskable icons
tile 256 244 favicon.png     # fills small sizes

round master.png 512 112 "$PUB/icon-512.png"
round master.png 192 42 "$PUB/icon-192.png"
magick maskable.png -filter Lanczos -resize 512x512 "${OUT[@]}" "$PUB/icon-maskable-512.png"
magick master.png -filter Lanczos -resize 180x180 "${OUT[@]}" "$PUB/icon-180.png"
round master.png 384 84 mark-384.png
magick mark-384.png -quality 90 -define webp:alpha-quality=100 "$PUB/logo-mark.webp"
round favicon.png 32 7 "$PUB/favicon-32.png"
magick favicon.png \( -clone 0 -resize 16x16 \) \( -clone 0 -resize 32x32 \) \( -clone 0 -resize 48x48 \) \
  -delete 0 "$PUB/favicon.ico"

# Wordmark: colour-to-alpha against the page background, so white letters and the amber
# cube keep their anti-aliasing on any dark background.
magick sheet.png -crop 681x156+445+658 +repage word-raw.png
FXA='kr=14/255;kg=20/255;kb=27/255; dr=u.r>kr?(u.r-kr)/(1-kr):(kr-u.r)/kr; dg=u.g>kg?(u.g-kg)/(1-kg):(kg-u.g)/kg; db=u.b>kb?(u.b-kb)/(1-kb):(kb-u.b)/kb; al=max(dr,max(dg,db)); al<0.12?0:min(1,(al-0.12)/0.8)'
FXC='al=max(v.r,0.004); kk=channel(14/255,20/255,27/255,0,0); min(1,max(0,(u-kk)/al+kk))'
magick word-raw.png -fx "$FXA" -colorspace gray word-alpha.png
magick word-raw.png word-alpha.png -fx "$FXC" word-rgb.png
magick word-rgb.png word-alpha.png -alpha off -compose CopyOpacity -composite -trim +repage \
  -quality 92 -define webp:alpha-quality=100 "$PUB/logo-wordmark.webp"

# README banner: the horizontal lockup on its own background, with breathing room.
magick sheet.png -crop 975x211+143+645 +repage -bordercolor "$PAGE" -border 60x44 \
  -fuzz 3% -fill "$PAGE" -opaque "$PAGE" "${OUT[@]}" ../beltwise-banner.png

rm -f sheet.png mark.png mask.png mark-soft.png master.png maskable.png favicon.png mark-384.png \
  word-raw.png word-alpha.png word-rgb.png
ls -la "$PUB" ../beltwise-banner.png
