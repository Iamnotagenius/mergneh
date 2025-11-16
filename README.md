# Mergneh
(pronounced Mer-gneh), which comes from two Proto-Indo-European words: `*merǵ-` (meaning border) and `*ǵneh₃-` (meaning knowing).

A really simple program which creates running text in the terminal.

## Usage:
Mergneh reads text from various sources and then outputs it like a running banner.

Let's start with simple examples. Here's a static running text with 300ms delay:
![example1](https://github.com/user-attachments/assets/5bb8918e-3883-465a-bac6-342508afdf7b)
> [!NOTE]
> Go [here](https://docs.rs/humantime/latest/humantime/fn.parse_duration.html) to see what time suffixes are supported.

If you want to, you can use a command to make running text dynamic:
![example2](https://github.com/user-attachments/assets/7e16ec9f-f09a-4178-9f50-031a0f87d64b)

It is possible to have multiple running fragments on one line:
![example3](https://github.com/user-attachments/assets/98dadfe1-27fe-479d-b327-7e22c06831f6)
As you can see, Fragment is defined by one of the sources
followed by a set of options which are applied to this specific fragment.

You can compile it with mpd support, then it would be able to connect to mpd daemon and read its status:
![example4](https://github.com/user-attachments/assets/ac85dbfd-8671-4365-9522-29603183ac10)

Here's an example with waybar.
Suppose we defined a module `custom/mpd`, now we only need to configure the module like this:
```json
"custom/mpd": {
    "on-click": "mpc toggle > /dev/null",
    "on-scroll-up": "mpc volume +5",
    "on-scroll-down": "mpc volume -5",
    "exec": "~/.config/waybar/mergneh_mpd.sh",
    "return-type": "json"
}
```
And in `mergneh_mpd.sh`:
```bash
#!/usr/bin/env bash

exec mg \
    -S '{"text":" ' \
    --mpd -w 30 -s ' > ' -e '&=&amp;,"=&quot;' -t \
    --mpd --format ' [{elapsedTime}/{totalTime}] {stateIcon}' \
    -S '","tooltip":"' \
    --mpd --format '{artist} - {title}: {album} ({date}) ({randomIcon:1}{repeatIcon:1}{singleIcon}{consumeIcon:1}) #{songPosition}/{queueLength}' \
        -e '&=&amp;,"=&quot;' -w 1000 -t \
    -S '"}' \
    run -n -d 100ms
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
