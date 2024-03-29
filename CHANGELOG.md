# Changelog

## Unreleased

## 0.10.0 - 2023-03-26

- Set default pathvalue components capacity to be the machine wordsize (8 elements on 64-bit, 4 elements on 32-bit)
- Remove `structopt` and use `clap`'s derive functionality instead
- Bump `nix` to version `0.26`
- `nix` dependency: use only the `signal` feature
- Use `unicode-ident`, remove `unicode-xid`
- Use `tikv-jemallocator`, remove `jemallocator`
- Bump many transitive deps
- Use more descriptive lifetimes in `lib`, `main`, and `path_value_sink`

## 0.9.0 - 2022-01-10

- Add `format` CLI option. Currently implementations for `gron`, `json_pointer`, and `json` outputs.
- Breaking change: all formatters now only print scalars by default. Scalars are defined as: JSON strings, numbers, booleans, nulls, as well as empty objects and empty arrays. I will likely add back the ability to print all values (scalars and aggregates) at some point soon. This breaks API compatibility with the `gron` project, but I think this is the correct choice: outputing `{}` or `[]` for an object or array that is not empty is misleading.
- Fix clippys on tests
- Bump itoa to version 1
- Bump bumpalo, crossbeam-channel, crossbeam-epoch, crossbeam-utils, syn
- Readme updates

## 0.8.2 - 2021-11-10

- Use `unicode-xid` to determine whether a string is a valid Javascript/JSON identifier or not.

## 0.8.1 - 2021-11-08

- Use `license` instead of `license-file` in `Cargo.toml`

## 0.8.0 - 2021-11-08

- Note that in accordance with the project's <1.0 version, this is a breaking change.
- The output of the `jindex` CLI now matches [gron](https://github.com/tomnomnom/gron), and passes many of `gron`'s tests. In this configuration it is significantly faster than `gron` itself as proven by benchmarks.
- If you prefer the existing [JSON Pointer](https://datatracker.ietf.org/doc/html/rfc6901) output format in the CLI, it should be a one or two line change to `main.rs` at most. At some point in the future I may make separate Cargo examples for both `gron` and `json_pointer` output formats that would allow for the building of separate binaries.
- The functionality of `jindex` is now available as a library that you can use in your own code and extend through a trait. This is what is allowing the `gron` and `JSON Pointer` output formats to exist side-by-side.
- Adds a benchmark suite that measures both the new `gron`-style output as well as the previous `JSON Pointer` style.
- `jindex` is reliable in day-to-day use and decently well tests but I still consider it pre-1.0 as I am not 100% sure that the API design will not change slightly in the future.

## 0.7.0 - 2021-03-28

- Speedup by using `ManuallyDrop` to have the OS clean up rather than running destructors
- Bump deps

## 0.6.0 - 2021-03-14

- Fix performance regression introduced in 0.5.0.
- Bump deps

## 0.5.0 - 2021-02-19

- Big internal refactor/rebuild. No user/API changes other than an additional error message for when a non-array/object JSON value is passed.
- There is a slight performance regression that should be unnoticeable on all but very large inputs (hundreds of megabytes).

## 0.4.0 - 2020-12-03

- Use a string pool to reuse path strings for a nice speedup: [071f20d](https://github.com/ckampfe/jindex/commit/071f20d)
- Remove unneeded value_buf, write to io_buf directly: [469d282](https://github.com/ckampfe/jindex/commit/469d282)
- Bump cc and serde_json: [99ea54d](https://github.com/ckampfe/jindex/commit/99ea54d)

## 0.3.0 - 2020-11-22

- Clippy: allow_too_many arguments on `build_and_write_path`: [f13ad65](https://github.com/ckampfe/jindex/commit/f13ad65f0ae348d3eaa5f5be612980584bc32207)
- Make sure to flush stdout `BufWriter` before it is dropped: [e00005a](https://github.com/ckampfe/jindex/commit/e00005a00e2626246b6c026f42a0a36c1229b2c1)
- Bump cc and syn: [1a295c4](https://github.com/ckampfe/jindex/commit/1a295c4941e55c17f220c0d82f65a19dbc6b3e1d)

## 0.2.0 - 2020-11-18

- do not create a new vec for every value: [ff17bed](https://github.com/ckampfe/jindex/commit/ff17bedf9dd11245af25d88f1b576fabc31b1112)
- add some documentation for various buffers: [c7395d2](https://github.com/ckampfe/jindex/commit/c7395d20d7ad376b4db42f87a5bce0f2ffffcd0f)
- only derive StructOpt on Options: [834d28d](https://github.com/ckampfe/jindex/commit/834d28ddc1cc3d6e2344e4f54bad067cccb37b77)
- bump unicode-segmentation: [8751c23](https://github.com/ckampfe/jindex/commit/8751c23cc9fedc03ac3105c5e3e80bd4823e6183)

## 0.1.0 - 2020-11-07

### Notable changes

Open sourced!
