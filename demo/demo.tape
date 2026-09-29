# Records demo/keb.gif — the README hero.
#
#   cargo build --release && vhs demo/demo.tape
#
# Run from the repo root. keb only ever runs inside playground/demo, which
# setup.sh recreates from scratch on every recording.

Output demo/keb.gif

Set Shell "bash"
Set FontSize 20
Set Width 1140
Set Height 580
Set Padding 28
Set Theme "Catppuccin Mocha"
Set TypingSpeed 55ms

Hide
Type "export PS1='$ ' PATH=$PWD/target/release:$PATH" Enter
Type "sh demo/setup.sh && cd playground/demo && clear" Enter
Sleep 500ms
Show

Sleep 800ms
Type "ls -1" Sleep 400ms Enter
Sleep 2.5s

Type "keb -n *" Sleep 600ms Enter
Sleep 4s

Ctrl+L
Type "keb *" Sleep 400ms Enter
Sleep 1.5s
Type "ls -1" Sleep 400ms Enter
Sleep 4s
