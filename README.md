# jindex

Enumerate the paths through a JSON document,
with an output that is API-compatible with [gron](https://github.com/tomnomnom/gron)

[![Rust](https://github.com/ckampfe/jindex/actions/workflows/rust.yml/badge.svg)](https://github.com/ckampfe/jindex/actions/workflows/rust.yml)

## Installation

Latest stable release from crates.io:

```
$ cargo install jindex
```

Latest unstable (HEAD) release from source:

```
$ cargo install --git https://github.com/ckampfe/jindex
```

## Examples

You can pass JSON through stdin:

```
$ echo '{
  "a": 1,
  "b": 2,
  "c": ["x", "y", "z"],
  "d": {"e": {"f": [{}, 9, "g"]}}
}' | jindex

json.d.e.f[2] = "g";
json.d.e.f[1] = 9;
json.d.e.f[0] = {};
json.c[2] = "z";
json.c[1] = "y";
json.c[0] = "x";
json.b = 2;
json.a = 1;
```

or from a file:

```
$ jindex myfile.json

json.d.e.f[2] = "g";
json.d.e.f[1] = 9;
json.d.e.f[0] = {};
json.c[2] = "z";
json.c[1] = "y";
json.c[0] = "x";
json.b = 2;
json.a = 1;
```

With the [json_pointer](https://datatracker.ietf.org/doc/html/rfc6901) format option:

```
$ jindex -fjson_pointer myfile.json
/d/e/f/2        "g"
/d/e/f/1        9
/d/e/f/0        {}
/c/2    "z"
/c/1    "y"
/c/0    "x"
/b      2
/a      1
```

With the `json` format option:

```
jindex -fjson myfile.json
{"path_components":["d","e","f",2],"value":"g"}
{"path_components":["d","e","f",1],"value":9}
{"path_components":["d","e","f",0],"value":{}}
{"path_components":["c",2],"value":"z"}
{"path_components":["c",1],"value":"y"}
{"path_components":["c",0],"value":"x"}
{"path_components":["b"],"value":2}
{"path_components":["a"],"value":1}
```

## Command-line interface

```
$ jindex -h
jindex 0.8.2
Enumerate the paths through a JSON document

USAGE:
    jindex [OPTIONS] [json-location]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -f, --format <format>    gron, json_pointer, json [default: gron]

ARGS:
    <json-location>    A JSON file path

```

## Path output order

`jindex` makes *no guarantees at all* about the order in which paths are output.
Paths may appear depth-first, breadth-first, or any other order at all relative to their position in the input JSON document.
Further, *any ordering is not guaranteed to be stable from one version to the next*,
as it may change to aid the implementation of new optimizations.
If a stable order is important, I recommend using `sort` or some other after-the-fact
mechanism, as the set of paths output from a given input document are guaranteed
to be stable over time.

## Performance

To run the benchmarks:

```
# install the benchmark runner
$ cargo install cargo-criterion
```

```
# clone the project
$ git clone https://github.com/ckampfe/jindex
```

```
# run the benchmarks
$ cd jindex
$ cargo criterion
```

## Features

`jindex` uses [jemalloc](http://jemalloc.net/) by default for a substantial increase in throughput.
If you do not wish to use jemalloc, you can build without it by passing the `--no-default-features` flag to Cargo.

## Version policy

`jindex` remains pre-1.0 and as such does not guarantee API compatibility from one version to the next. That said, `jindex` has a very small API, and is not likely to change markedly in the future. Reaching a 1.0 version is a project goal but not one I consider more important than others. If this is a problem or if you have questions please open an issue.
