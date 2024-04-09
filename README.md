# Mergneh
(pronounced Mer-gneh), which comes from two Proto-Indo-European words: `*merǵ-` (meaning border) and `*ǵneh₃-` (meaning knowing).

A really simple program which creates running text in the terminal.

## Usage:
Mergneh reads text from various sources and then outputs it like a running banner.

Let's start with simple examples. Here's a static running text with 300ms delay:
![text](https://github.com/Iamnotagenius/mergneh/assets/58214104/76451c39-0391-40e4-9c4b-543f383f735f)
> [!NOTE]
> Go [here](https://docs.rs/humantime/latest/humantime/fn.parse_duration.html) to see what time suffixes are supported

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
> [!NOTE]
> Format placeholders are fully compatible with [Waybar's ones](https://github.com/Alexays/Waybar/wiki/Module:-MPD#format-replacements).

Fortunately, the program also supports streaming running text continuously in waybar's custom module (also under a feature flag).
Suppose we defined a module `custom/mpd`, now we only need to configure the module like this:
```json
"custom/mpd": {
    "on-click": "mpc toggle > /dev/null",
    "on-scroll-up": "mpc volume +5",
    "on-scroll-down": "mpc volume -5",
    "exec": "mg --mpd -l \" \" -R \" [{elapsedTime}/{totalTime}] {stateIcon}\" -w 30 --separator \"  \" --dont-repeat waybar -d 100ms -t",
    "return-type": "json"
}
```
And that's everything you need, really. Here's a demo:
![waybar](https://github.com/Iamnotagenius/mergneh/assets/58214104/c579972d-20a6-427b-9201-ffee547ec421)

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
          Repeat icons to use [default: 凌稜]
      --consume-icons <ICONS>
          Consume icons to use [default: ]
      --random-icons <ICONS>
          Random icons to use [default: ]
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

Usage: mg <SOURCE|--file <FILE>|--string <STRING>|--stdin|--cmd <ARGS>...|--mpd [<SERVER_ADDR>]> run [OPTIONS]

Options:
  -d, --duration <DURATION>  Tick duration [default: 1s]
  -h, --help                 Print help
```
Options for a `iter` subcommand:
```
Print just one iteration

Usage: mg <SOURCE|--file <FILE>|--string <STRING>|--stdin|--cmd <ARGS>...|--mpd [<SERVER_ADDR>]> iter <ITER_FILE>

Arguments:
  <ITER_FILE>  File containing data for next iteration

Options:
  -h, --help  Print help
```
Options for a `waybar` subcommand:
```
Run text with custom module in waybar (JSON output)

Usage: mg <SOURCE|--file <FILE>|--string <STRING>|--stdin|--cmd <ARGS>...|--mpd [<SERVER_ADDR>]> waybar [OPTIONS] [TOOLTIP]

Arguments:
  [TOOLTIP]  Tooltip to show on hover

Options:
  -d, --duration <DURATION>        Tick duration [default: 1s]
      --tooltip-cmd <ARGS>...      Use output of a command for tooltip
  -t, --tooltip-format [<FORMAT>]  Tooltip format with MPD placeholder support [default: {artist} - {title}]
  -h, --help                       Print help
```
