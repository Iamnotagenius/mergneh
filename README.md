# Mergneh
(pronounced Mer-gneh), which comes from two Proto-Indo-European words: `*merǵ-` (meaning border) and `*ǵneh₃-` (meaning knowing).
A really simple program which creates running text in the terminal.

Usage:
```
Usage: mg [OPTIONS] <SOURCE|--file <FILE>|--string <STRING>|--stdin> <COMMAND>

Commands:
  run   Run text in a terminal
  iter  Print just one iteration
  help  Print this message or the help of the given subcommand(s)

Arguments:
  <SOURCE>  same as --file, if file with this name does not exist or is a directory, it will behave as --string

Options:
  -f, --file <FILE>      Pull contents from a file (BEWARE: it loads whole file into memory!)
  -S, --string <STRING>  Use a string as contents
      --stdin            Pull contents from stdin (BEWARE: it loads whole input into memory just like --file)
  -w, --window <WINDOW>  Window size [default: 6]
  -s, --separator <SEP>  String to print between content [default: ]
  -n, --newline <NL>     String to replace newlines with [default: ]
  -l, --prefix <PREFIX>  String to print before running text [default: ]
  -r, --suffix <SUFFIX>  String to print after running text [default: ]
  -1, --dont-repeat      Do not repeat contents if it fits in the window size
  -h, --help             Print help
  -V, --version          Print version
```
