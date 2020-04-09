jindex
===

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

["a"]   1
["b"]   2
["c", 0]        "x"
["c", 1]        "y"
["c", 2]        "z"
["d", "e", "f", 0]      {}
["d", "e", "f", 1]      9
["d", "e", "f", 2]      "g"
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
["a"]   1
["b"]   2
["c", 0]        "x"
["c", 1]        "y"
["c", 2]        "z"
["d", "e", "f", 0]      {}
["d", "e", "f", 1]      9
["d", "e", "f", 2]      "g"
```

With a custom separator between the path and the value:

```
$ jindex -s@@@ simple.json
["a"]@@@1
["b"]@@@2
["c", 0]@@@"x"
["c", 1]@@@"y"
["c", 2]@@@"z"
["d", "e", "f", 0]@@@{}
["d", "e", "f", 1]@@@9
["d", "e", "f", 2]@@@"g"
```

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
