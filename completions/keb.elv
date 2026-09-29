
use builtin;
use str;

set edit:completion:arg-completer[keb] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'keb'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'keb'= {
            cand --separator 'Emit this character between words instead of `-`'
            cand --max-length 'Override the 255 byte / UTF-16 unit name limit'
            cand --format 'How to print each rename; `json` and `null` are the parse-safe ones'
            cand -n 'Print the plan, change nothing'
            cand --dry-run 'Print the plan, change nothing'
            cand -r 'Recurse into directories'
            cand --recursive 'Recurse into directories'
            cand -f 'Overwrite on collision, and rename protected names under -r'
            cand --force 'Overwrite on collision, and rename protected names under -r'
            cand -d 'Permit renaming directories. Without it, only files are renamed'
            cand --allow-dirs 'Permit renaming directories. Without it, only files are renamed'
            cand -0 'Paths on stdin are null-separated, for `find -print0`'
            cand --null 'Paths on stdin are null-separated, for `find -print0`'
            cand --ascii 'Narrow the output to ASCII, dropping scripts that are otherwise kept'
            cand --absolute 'Print absolute paths. Lexical only — symlinks are not resolved'
            cand --list-protected 'List the names keb never renames, and why. Renames nothing'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
