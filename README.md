# logstat

A small command-line tool that reads an nginx access log in the `combined`
format and prints a short summary:

- total number of requests and number of skipped (malformed) lines
- unique client IPs and total bytes sent
- top N requested paths
- requests by HTTP method
- responses by status class (2xx / 3xx / 4xx / 5xx)
- number of 5xx responses per hour

Malformed lines are counted and skipped, so one broken line does not stop
the whole report.

## Usage

Requires a Rust toolchain (https://rustup.rs).

```sh
cargo run -- examples/sample.log
cargo run -- examples/sample.log --top 3
cargo run -- --help
```

Or build a release binary and run it directly:

```sh
cargo build --release
./target/release/logstat /var/log/nginx/access.log --top 10
```

Options:

| Option      | Description                              | Default |
|-------------|------------------------------------------|---------|
| `<PATH>`    | Path to the access log                   |         |
| `--top <N>` | How many paths to show in the top list   | 5       |
| `-h, --help` | Print help                            |         |

## Example output

```
$ cargo run -- examples/sample.log
Total requests: 17
Skipped lines:  3
Unique IPs:     6
Total bytes:    12559

Top 5 paths:
       4  /api/orders
       4  /api/users
       3  /
       2  /index.html
       1  /admin

Requests by method:
      13  GET
       3  POST
       1  DELETE

Status classes:
  2xx       9
  3xx       2
  4xx       2
  5xx       4

5xx by hour:
  13:00       1
  14:00       2
  15:00       1
```

## Log format

nginx `combined` format:

```
$remote_addr - $remote_user [$time_local] "$request" $status $body_bytes_sent "$http_referer" "$http_user_agent"
```

Example line:

```
192.168.1.10 - - [10/Oct/2023:13:55:36 +0000] "GET /index.html HTTP/1.1" 200 2326 "-" "curl/8.4.0"
```

## Notes

- "5xx by hour" groups by hour of day (00–23). For a log that spans several
  days, the same hour from different days is added together.
- Paths with equal counts are sorted alphabetically, so the output is the
  same on every run.
- The file must be valid UTF-8; otherwise the tool stops with a read error.

## Project layout

```
src/main.rs        CLI arguments, file reading, report output
src/parser.rs      parsing one log line into a LogEntry
src/stats.rs       counting statistics over parsed entries
examples/sample.log  small log for trying the tool and for tests
```

## Tests

```sh
cargo test
```
