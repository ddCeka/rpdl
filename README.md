# Full Command-Line for `rpdl`

This file contains the manual for the `rpdl` command-line program.


## `rpdl`

**Usage:** `rpdl [OPTIONS] <COMMAND>`

###### **Subcommands:**

* `single` — Download a single file with multi-threaded chunks
* `multi` — Download multiple files concurrently
* `batch` — Download URLs from a file (one per line)
* `specialized` — Download from specialized sites using their APIs
* `info` — Show network metrics and performance info
* `resume` — Resume an interrupted download
* `completions` — Generate shell completions
* `torrent` — Download from torrent file or magnet link
* `torrent-info` — Get information about a torrent
* `schedule` — Schedule a download

###### **Options:**

* `-v`, `--verbose` — Enable verbose output
* `-q`, `--quiet` — Suppress all output except errors



## `rpdl single`

Download a single file with multi-threaded chunks

**Usage:** `rpdl single [OPTIONS] --output <OUTPUT> <URL>`

###### **Arguments:**

* `<URL>` — URL to download

###### **Options:**

* `-o`, `--output <OUTPUT>` — Output file path
* `-c`, `--max-chunks <MAX_CHUNKS>` — Maximum concurrent chunks

  Default value: `8`

* `-s`, `--chunk-size <CHUNK_SIZE>` — Chunk size in MB

  Default value: `5`

* `--strategy <STRATEGY>` — Download strategy (simple or smart)

  Default value: `smart`

  Possible values: `simple`, `smart`

* `--no-adaptive` — Disable adaptive chunk sizing
* `--no-progress` — Disable progress bars
* `-t`, `--timeout <TIMEOUT>` — Timeout in seconds

  Default value: `30`



## `rpdl multi`

Download multiple files concurrently

**Usage:** `rpdl multi [OPTIONS] [URLS]...`

###### **Arguments:**

* `<URLS>` — URLs to download (can be specified multiple times)

###### **Options:**

* `-o`, `--output-dir <OUTPUT_DIR>` — Output directory for downloaded files
* `-c`, `--max-concurrent <MAX_CONCURRENT>` — Maximum concurrent downloads

  Default value: `10`

* `--unordered` — Download files in unordered mode (faster)
* `--no-progress` — Disable progress bars
* `-t`, `--timeout <TIMEOUT>` — Timeout in seconds per file

  Default value: `30`

* `-u`, `--user-agent <USER_AGENT>` — Custom user agent string



## `rpdl batch`

Download URLs from a file (one per line)

**Usage:** `rpdl batch [OPTIONS] <FILE>`

###### **Arguments:**

* `<FILE>` — File containing URLs (one per line)

###### **Options:**

* `-o`, `--output-dir <OUTPUT_DIR>` — Output directory for downloaded files
* `-c`, `--max-concurrent <MAX_CONCURRENT>` — Maximum concurrent downloads

  Default value: `10`

* `--no-progress` — Disable progress bars
* `-t`, `--timeout <TIMEOUT>` — Timeout in seconds per file

  Default value: `30`



## `rpdl specialized`

Download from specialized sites using their APIs

**Usage:** `rpdl specialized <COMMAND>`

###### **Subcommands:**

* `github` — Download from GitHub release or source
* `list` — List all supported specialized sites



## `rpdl specialized github`

Download from GitHub release or source

**Usage:** `rpdl specialized github [OPTIONS] --output <OUTPUT> <REPO> [TAG] [ASSET]`

###### **Arguments:**

* `<REPO>` — owner/repo or owner/repo#commit
* `<TAG>` — Optional: tag name or 'latest'
* `<ASSET>` — Optional: specific asset name

###### **Options:**

* `-o`, `--output <OUTPUT>` — Output file or directory path
* `--token <TOKEN>` — GitHub personal access token (optional)
* `--show-info` — Show release/repository information



## `rpdl specialized list`

List all supported specialized sites

**Usage:** `rpdl specialized list`



## `rpdl info`

Show network metrics and performance info

**Usage:** `rpdl info [OPTIONS] <URL>`

###### **Arguments:**

* `<URL>` — URL to test

###### **Options:**

* `-s`, `--samples <SAMPLES>` — Number of test chunks to download

  Default value: `5`



## `rpdl resume`

Resume an interrupted download

**Usage:** `rpdl resume [OPTIONS] <FILE>`

###### **Arguments:**

* `<FILE>` — Path to the incomplete download file

###### **Options:**

* `-f`, `--force` — Force resume even if state is corrupted



## `rpdl completions`

Generate shell completions

**Usage:** `rpdl completions <SHELL>`

###### **Arguments:**

* `<SHELL>` — Shell to generate completions for

  Possible values: `bash`, `elvish`, `fish`, `zsh`




## `rpdl torrent`

Download from torrent file or magnet link

**Usage:** `rpdl torrent [OPTIONS] <SOURCE>`

###### **Arguments:**

* `<SOURCE>` — Torrent file path, URL, or magnet link

###### **Options:**

* `-o`, `--output-dir <OUTPUT_DIR>` — Output directory
* `--select <SELECT>` — Select specific files (regex pattern)
* `--list-only` — List files without downloading
* `--no-progress` — Disable progress display



## `rpdl torrent-info`

Get information about a torrent

**Usage:** `rpdl torrent-info <SOURCE>`

###### **Arguments:**

* `<SOURCE>` — Torrent file, URL, or magnet link



## `rpdl schedule`

Schedule a download

**Usage:** `rpdl schedule <COMMAND>`

###### **Subcommands:**

* `add` — Add a new scheduled download
* `list` — List all scheduled downloads
* `remove` — Remove a scheduled download
* `start` — Start the scheduler
* `toggle` — Enable/disable a scheduled download



## `rpdl schedule add`

Add a new scheduled download

**Usage:** `rpdl schedule add [OPTIONS] --output <OUTPUT> --at <AT> <ID> <URL>`

###### **Arguments:**

* `<ID>` — Unique ID for this scheduled download
* `<URL>` — URL to download

###### **Options:**

* `-o`, `--output <OUTPUT>` — Output path
* `--at <AT>` — Schedule time (RFC3339 format)
* `--repeat <REPEAT>` — Repeat pattern (daily, weekly, hourly)



## `rpdl schedule list`

List all scheduled downloads

**Usage:** `rpdl schedule list`



## `rpdl schedule remove`

Remove a scheduled download

**Usage:** `rpdl schedule remove <ID>`

###### **Arguments:**

* `<ID>` — ID of scheduled download to remove



## `rpdl schedule start`

Start the scheduler

**Usage:** `rpdl schedule start`



## `rpdl schedule toggle`

Enable/disable a scheduled download

**Usage:** `rpdl schedule toggle [OPTIONS] <ID>`

###### **Arguments:**

* `<ID>` — ID of scheduled download

###### **Options:**

* `--enable` — Enable or disable

