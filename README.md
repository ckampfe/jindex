jindex
===

## Installation

```
$ git clone git@github.com:ckampfe/jindex.git
$ cd jindex
$ cargo install --path .
```

## Use

```
$ cat foo.json
{
  "a": 1,
  "b": 2,
  "c": "bar"
}
$ cat foo.json | jindex
["a"] => 1
["b"] => 2
["c"] => "bar"
$ jindex foo.json
["a"] => 1
["b"] => 2
["c"] => "bar"
```

```
$ jindex -h
jindex 0.1.0

USAGE:
    jindex [json-location]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

ARGS:
    <json-location>
```
