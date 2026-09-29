# Records demo/preview.png — the GitHub social preview (1280×640), one frame showing a
# folder before and after.
#
#   cargo build --release && vhs demo/preview.tape
#
# Run from the repo root. keb only ever runs inside playground/demo, which setup.sh
# recreates from scratch. VHS insists on a video output, so the GIF goes to the
# gitignored playground and only the screenshot is kept.

Output playground/preview.gif

Set Shell "bash"
Set FontSize 19
Set Width 1280
Set Height 640
Set Padding 40
Set LineHeight 1.55
Set Theme "Catppuccin Mocha"
Set TypingSpeed 0

Hide
Type "export PS1='$ ' PATH=$PWD/target/release:$PATH" Enter
Type "sh demo/setup.sh && cd playground/demo && clear" Enter
Sleep 500ms
Type "# keb: rename files to kebab case, safely" Enter
Type "keb *" Enter
Sleep 2s
Type "ls -1" Enter
Sleep 1s
Show

Sleep 200ms
Screenshot demo/preview.png
