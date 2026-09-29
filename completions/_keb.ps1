
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'keb' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'keb'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'keb' {
            [CompletionResult]::new('--separator', '--separator', [CompletionResultType]::ParameterName, 'Emit this character between words instead of `-`')
            [CompletionResult]::new('--max-length', '--max-length', [CompletionResultType]::ParameterName, 'Override the 255 byte / UTF-16 unit name limit')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'How to print each rename; `json` and `null` are the parse-safe ones')
            [CompletionResult]::new('-n', '-n', [CompletionResultType]::ParameterName, 'Print the plan, change nothing')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Print the plan, change nothing')
            [CompletionResult]::new('-r', '-r', [CompletionResultType]::ParameterName, 'Recurse into directories')
            [CompletionResult]::new('--recursive', '--recursive', [CompletionResultType]::ParameterName, 'Recurse into directories')
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'Overwrite on collision, and rename protected names under -r')
            [CompletionResult]::new('--force', '--force', [CompletionResultType]::ParameterName, 'Overwrite on collision, and rename protected names under -r')
            [CompletionResult]::new('-d', '-d', [CompletionResultType]::ParameterName, 'Permit renaming directories. Without it, only files are renamed')
            [CompletionResult]::new('--allow-dirs', '--allow-dirs', [CompletionResultType]::ParameterName, 'Permit renaming directories. Without it, only files are renamed')
            [CompletionResult]::new('-0', '-0', [CompletionResultType]::ParameterName, 'Paths on stdin are null-separated, for `find -print0`')
            [CompletionResult]::new('--null', '--null', [CompletionResultType]::ParameterName, 'Paths on stdin are null-separated, for `find -print0`')
            [CompletionResult]::new('--ascii', '--ascii', [CompletionResultType]::ParameterName, 'Narrow the output to ASCII, dropping scripts that are otherwise kept')
            [CompletionResult]::new('--absolute', '--absolute', [CompletionResultType]::ParameterName, 'Print absolute paths. Lexical only — symlinks are not resolved')
            [CompletionResult]::new('--list-protected', '--list-protected', [CompletionResultType]::ParameterName, 'List the names keb never renames, and why. Renames nothing')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
