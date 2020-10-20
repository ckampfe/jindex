# jindex

Enumerate the paths through a JSON document.

[![CircleCI](https://circleci.com/gh/ckampfe/jindex.svg?style=svg)](https://circleci.com/gh/ckampfe/jindex)

## Installation

```
$ git clone git@github.com:ckampfe/jindex.git
$ cd jindex
$ cargo install --path .
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

/a      1
/b      2
/c/0    "x"
/c/1    "y"
/c/2    "z"
/d/e/f/0        {}
/d/e/f/1        9
/d/e/f/2        "g"
```

Or from a file:

```
$ cat simple.json
{
  "a": 1,
  "b": 2,
  "c": ["x", "y", "z"],
  "d": {"e": {"f": [{}, 9, "g"]}}
}

$ jindex simple.json
/a      1
/b      2
/c/0    "x"
/c/1    "y"
/c/2    "z"
/d/e/f/0        {}
/d/e/f/1        9
/d/e/f/2        "g"
```

With a custom separator between the path and the value:

```
$ jindex -s@@@ simple.json
/a@@@1
/b@@@2
/c/0@@@"x"
/c/1@@@"y"
/c/2@@@"z"
/d/e/f/0@@@{}
/d/e/f/1@@@9
/d/e/f/2@@@"g"
```

Paths are done in the style of [RFC6901](https://tools.ietf.org/html/rfc6901).

## Command-line interface

```
$ jindex -h
jindex 0.1.0

USAGE:
    jindex [FLAGS] [OPTIONS] [json-location]

FLAGS:
    -a, --all        Write all path values, including composite ones
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -s, --separator <separator>    Separator string, defaults to tab [default:  ]

ARGS:
    <json-location>    A JSON file path
```

## Path output order

The order in which paths are output is undefined.
Paths may appear depth-first, breadth-first, or any other order at all relative to their position in the input JSON document.
Further, any ordering is not guaranteed to be stable from one version to the next,
as it may change to aid the implementation of new optimizations.
If a stable order is important, I recommend using `sort` or some other after-the-fact
mechanism, as the set of paths output from a given input document are guaranteed
to be stable over time
(instability of the actual member paths of the set is considered a bug).

## Benchmark

With jemalloc (enabled by default):

```
$ ls -la ~/code/sf-city-lots-json/citylots.json
.rw-r--r-- 189M clark  9 Apr 15:52 /Users/clark/code/sf-city-lots-json/citylots.json

$ /usr/bin/time -l jindex ~/code/sf-city-lots-json/citylots.json > /dev/null
        2.69 real         2.31 user         0.37 sys
1151422464  maximum resident set size
         0  average shared memory size
         0  average unshared data size
         0  average unshared stack size
    281130  page reclaims
         0  page faults
         0  swaps
         0  block input operations
         0  block output operations
         0  messages sent
         0  messages received
         0  signals received
         0  voluntary context switches
        64  involuntary context switches
```

## Features

`jindex` uses [jemalloc](http://jemalloc.net/) by default for a substantial increase in throughput.
If you do not wish to use jemalloc, you can build without it by passing the `--no-default-features` flag to Cargo.
