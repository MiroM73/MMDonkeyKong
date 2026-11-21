# Languages

Built-in languages and how to add or fix a translation without recompiling.

[← Back to README](../README.md)

Built in: **English, Slovenčina, Čeština, Deutsch**. Choose under View → Language. "Automatic" follows
the operating system language (falls back to English).

## Adding or fixing a translation – no recompiling

Language files are plain UTF-8 text files named `<code>.lang` (`fr.lang`, `pl.lang`, `pt-BR.lang` …):

```
# comment
language.name = Français
menu.file = Fichier
error.save_settings = Impossible d'enregistrer les paramètres : {error}
```

1. Copy `lang/en.lang` to `<code>.lang` (e.g. `fr.lang`) into one of the folders below.
2. Translate the text after `=`. Keep the keys and placeholders like `{error}` unchanged.
3. View → Language → **Reload language files** and pick the new language.

Files are loaded from:
- `lang\` next to `mmdonkeykong.exe` (good for distributing with the program),
- `%APPDATA%\MMDonkeyKong\lang\` (per user; Linux `~/.config/MMDonkeyKong/lang/`).

Rules:
- missing keys fall back to English, so a partial translation works too,
- a file named like a built-in language (`sk.lang`, `de.lang` …) overrides only the keys it contains –
  handy for fixing a single text,
- `\n` in a value is a line break.

The built-in languages live in `lang/*.lang` in the source tree and are embedded at compile time;
`cargo test` checks that every built-in language has all English keys with the same placeholders.
Every new `t!("key")` in the code needs the key in **all four** files.
