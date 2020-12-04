# Changelog

## Unreleased

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
