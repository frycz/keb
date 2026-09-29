complete -c keb -l separator -d 'Emit this character between words instead of `-`' -r
complete -c keb -l max-length -d 'Override the 255 byte / UTF-16 unit name limit' -r
complete -c keb -l format -d 'How to print each rename; `json` and `null` are the parse-safe ones' -r -f -a "arrow\t'`old -> new`'
old\t'The name before the rename'
new\t'The name after the rename'
json\t'One JSON object per line: `{"from":"…","to":"…"}`'
null\t'`old\\0new\\0`, for `xargs -0`'"
complete -c keb -s n -l dry-run -d 'Print the plan, change nothing'
complete -c keb -s r -l recursive -d 'Recurse into directories'
complete -c keb -s f -l force -d 'Overwrite on collision, and rename protected names under -r'
complete -c keb -s d -l allow-dirs -d 'Permit renaming directories. Without it, only files are renamed'
complete -c keb -s 0 -l null -d 'Paths on stdin are null-separated, for `find -print0`'
complete -c keb -l ascii -d 'Narrow the output to ASCII, dropping scripts that are otherwise kept'
complete -c keb -l absolute -d 'Print absolute paths. Lexical only — symlinks are not resolved'
complete -c keb -l list-protected -d 'List the names keb never renames, and why. Renames nothing'
complete -c keb -s h -l help -d 'Print help (see more with \'--help\')'
complete -c keb -s V -l version -d 'Print version'
