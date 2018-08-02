# Parallel Image Crop
A simple rust library that crops a set of images in parallel and that can be called from python.

## Description
The library requires a mini-batch of `paths`, `x-coordinates`, `y-coordinates`, `scales` and a `window-size`.
Using this it returns a crop per image. Parallelism takes place automatically using `rayon`

## Performance Statistics
These are using FFI for the rust library.
There are probably better python implementations, but this leverages `PIL-SIMD`:

```bash
> python benchmarks/test.py --batch-size=32 --num-trials=200
python crop average over 200 trials : 0.1789684545993805 sec
rust crop average over 200 trials : 0.06855524778366089 sec
```
