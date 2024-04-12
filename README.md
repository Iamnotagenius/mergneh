# Mergneh
(pronounced Mer-gneh), which comes from two Proto-Indo-European words: `*merǵ-` (meaning border) and `*ǵneh₃-` (meaning knowing).

A really simple program which creates running text in the terminal.

## Usage:
Mergneh reads text from various sources and then outputs it like a running banner.

Let's start with simple examples. Here's a static running text with 300ms delay:
![text](https://github.com/Iamnotagenius/mergneh/assets/58214104/76451c39-0391-40e4-9c4b-543f383f735f)
> [!NOTE]
> Go [here](https://docs.rs/humantime/latest/humantime/fn.parse_duration.html) to see what time suffixes are supported.

Neat, aint it? Mergneh can also save state between runs in a file:
```ansi
❯ mg "I am a running text" -s " | " iter iter_file.txt
I am a running text | I am a run
❯ mg "I am a running text" -s " | " iter iter_file.txt
 am a running text | I am a runn
❯ mg "I am a running text" -s " | " iter iter_file.txt
am a running text | I am a runni
❯ mg "I am a running text" -s " | " iter iter_file.txt
m a running text | I am a runnin
❯ mg "I am a running text" -s " | " iter iter_file.txt
 a running text | I am a running
❯ mg "I am a running text" -s " | " iter iter_file.txt
a running text | I am a running
```

If you want to, you can use a command to make running text dynamic:
![cmd](https://github.com/Iamnotagenius/mergneh/assets/58214104/38defa19-3532-4ea3-8e81-49bcb35b91d6)

You can compile it with mpd support, then it would be able to connect to mpd daemon and read its status:
![mpd](https://github.com/Iamnotagenius/mergneh/assets/58214104/05cc8e92-8fdb-43da-85c2-5a356b50f11b)

Here's an example with waybar.
Suppose we defined a module `custom/mpd`, now we only need to configure the module like this:
```json
"custom/mpd": {
    "on-click": "mpc toggle > /dev/null",
    "on-scroll-up": "mpc volume +5",
    "on-scroll-down": "mpc volume -5",
    "exec": "mg --mpd -l \"{\\\"text\\\":\\\" \" -R \" [{elapsedTime}/{totalTime}] {stateIcon}\\\",\\\"tooltip\\\":\\\"{artist} - {title}: [{album} ({date})] ({randomIcon:1}{repeatIcon:1}{singleIcon:1}{consumeIcon:1}) {{{songPosition}/{queueLength}}}\\\"}}\" -w 30 -s \"  \" -e \"&=&amp;\" -1 run -d 100ms -n",
    "return-type": "json"
}
```
And that's everything you need, really. Here's a demo:
![waybar](https://github.com/Iamnotagenius/mergneh/assets/58214104/c579972d-20a6-427b-9201-ffee547ec421)

### MPD format specifiers
The `--format` and subsequenly `--prefix-format`, `--suffix-format` support following format designators:
- `{albumArtist}`
- `{album}`
- `{artist}`
- `{consumeIcon}`
- `{date}`
- `{elapsedTime}`
- `{filename}`
- `{queueLength}`
- `{randomIcon}`
- `{repeatIcon}`
- `{singleIcon}`
- `{songPosition}`
- `{stateIcon}`
- `{title}`
- `{totalTime}`
- `{volume}`

> [!IMPORTANT]
> The `{*Icon}` placeholders take icons from respective options.
> The `--status-icons` option must be a 3-character long string, icons are specified in this order: play, pause, stop.
> Other sets of icons are 1 or 2-characters long: for enabled state and other one is optional for disabled state.

> [!NOTE]
> Icon placeholders can have a padding (specified like this: `{stateIcon:1}`), this is useful when icon glyphs are too big for one character.

> [!NOTE]
> `{*Time}` placeholders can have additional formatting specified after the ':' like this: `{elapsedTime:%M min %S sec}`. (the default one is `%M:%S`)
> For a more detailed overview of supported time specifiers go [here](https://docs.rs/chrono/latest/chrono/format/strftime/index.html).
> Keep in mind that you can only use a limited subset of specifiers.

### A brief overview of available options
`mg -h` should give you enough information. Anyway, here's available options:
```
Commands:
  run     Run text in a terminal
  iter    Print just one iteration
  waybar  Run text with custom module in waybar (JSON output)
  help    Print this message or the help of the given subcommand(s)

Options:
  -w, --window <WINDOW>  Window size [default: 32]
  -s, --separator <SEP>  String to print between content [default: ]
  -n, --newline <NL>     String to replace newlines with [default: ]
  -l, --prefix <PREFIX>  String to print before running text [default: ]
  -r, --suffix <SUFFIX>  String to print after running text [default: ]
  -1, --dont-repeat      Do not repeat contents if it fits in the window size
      --reset-on-change  Reset text window on content change
  -h, --help             Print help
  -V, --version          Print version

Sources:
  -f, --file <FILE>          Pull contents from a file (BEWARE: it loads whole file into memory!)
  -S, --string <STRING>      Use a string as contents
      --stdin                Pull contents from stdin (BEWARE: it loads whole input into memory just like --file)
      --cmd <ARGS>...        Execute a command and use its output as contents (use a ';' as a terminator)
      --mpd [<SERVER_ADDR>]  Display MPD status as running text [default server address is 127.0.0.0:6600]
  <SOURCE>                   Same as --file, if file with this name does not exist or is a directory, it will behave as --string

MPD Options:
      --status-icons <ICONS>
          Status icons to use [default: ]
      --repeat-icons <ICONS>
          Repeat icons to use [default: ]
      --consume-icons <ICONS>
          Consume icons to use [default: ]
      --random-icons <ICONS>
          Random icons to use [default: ]
      --single-icons <ICONS>
          Single icons to use [default: ]
      --format <FORMAT>
          Format string to use in running text [default: "{artist} - {title}"]
  -L, --prefix-format <FORMAT>
          Format string to use in prefix
  -R, --suffix-format <FORMAT>
          Format string to use in suffix
  -D, --default-placeholder <PLACEHOLDER>
          Default placeholder for missing values [default: N/A]

```
Options for a `run` subcommand:
```
Run text in a terminal

Usage: mg <SOURCE|--file <FILE>|--string <STRING>|--stdin|--cmd <ARGS>...> run [OPTIONS]

Options:
  -d, --duration <DURATION>  Tick duration [default: 1s]
  -n, --newline              Print each iteration on next line
  -h, --help                 Print help
```
Options for an `iter` subcommand:
```
Print just one iteration

Usage: mg <SOURCE|--file <FILE>|--string <STRING>|--stdin|--cmd <ARGS>...|--mpd [<SERVER_ADDR>]> iter <ITER_FILE>

Arguments:
  <ITER_FILE>  File containing data for next iteration

Options:
  -h, --help  Print help
```
