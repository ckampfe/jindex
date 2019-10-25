jindex
===

## Installation

```
$ git clone git@github.com:ckampfe/jindex.git
$ cd jindex
$ cargo install --path .
```

## Use

You can pass JSON through stdin:

```
$ echo '{
  "a": 1,
  "b": 2,
  "c": ["x", "y", "z"],
  "d": {"e": {"f": [{}, 9, "g"]}}
}' | jindex
["a"] => 1
["b"] => 2
["c"] => ["x","y","z"]
["d"] => {"e":{"f":[{},9,"g"]}}
["c", 0] => "x"
["c", 1] => "y"
["c", 2] => "z"
["d", "e"] => {"f":[{},9,"g"]}
["d", "e", "f"] => [{},9,"g"]
["d", "e", "f", 0] => {}
["d", "e", "f", 1] => 9
["d", "e", "f", 2] => "g"
```

Or from a file:

```
$ echo '{
  "a": 1,
  "b": 2,
  "c": ["x", "y", "z"],
  "d": {"e": {"f": [{}, 9, "g"]}}
}' > simple.json

$ jindex simple.json
["a"] => 1
["b"] => 2
["c"] => ["x","y","z"]
["d"] => {"e":{"f":[{},9,"g"]}}
["c", 0] => "x"
["c", 1] => "y"
["c", 2] => "z"
["d", "e"] => {"f":[{},9,"g"]}
["d", "e", "f"] => [{},9,"g"]
["d", "e", "f", 0] => {}
["d", "e", "f", 1] => 9
["d", "e", "f", 2] => "g"
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
