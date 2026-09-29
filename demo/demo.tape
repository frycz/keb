# Records demo/keb.gif — the README hero: list, rename, list of WhatsApp exports.
#
#   cargo build --release && vhs demo/demo.tape
#
# Run from the repo root. keb only ever runs inside playground/demo, which
# setup.sh recreates from scratch on every recording.

Output demo/keb.gif

Set Shell "bash"
Set FontSize 20
Set Width 1360
Set Height 520
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
Sleep 1.5s
Type "# spaces. parentheses. timestamps. why. 😤" Sleep 400ms Enter
Sleep 1.5s

Type "keb *" Sleep 400ms Enter
Sleep 2s
Type "ls -1" Sleep 400ms Enter
Sleep 1.5s
Type "# ahh. much better. 😌" Sleep 400ms Enter
Sleep 3s
