# Parallel Image Crop
A simple rust library that crops a set of images in parallel and that can be called from python.

## Description
The library requires a mini-batch of `paths`, `x-coordinates`, `y-coordinates`, `scales` and a `window-size`.
Using this it returns a crop per image. Parallelism takes place automatically using `rayon`

## Performance Statistics
These are using FFI for the rust library.
There are probably better python implementations, but this implementation is almost a 1:1 between Python & Rust.
The following tests compare the Rust implementation against `PIL-SIMD`, `pyvips` using `threading` and `multiprocessing` in Python.


Gray Scale 4000 x 4000 JPEG Image:

```bash
(base) ➜  parallel_image_crop git:(master) ✗ python benchmarks/test.py --batch-size=32 --num-trials=100 --use-vips=0 --use-threading=0
python crop average over 100 trials : 0.48177589893341066 +/- 0.06522766741342648 sec
rust crop average over 100 trials : 0.5541290354728698 +/- 0.01701503452857914 sec
(base) ➜  parallel_image_crop git:(master) ✗ python benchmarks/test.py --batch-size=32 --num-trials=100 --use-vips=0 --use-threading=1
python crop average over 100 trials : 0.46666148900985716 +/- 0.04960818629061339 sec
rust crop average over 100 trials : 0.5545140480995179 +/- 0.018513324366932055 sec
(base) ➜  parallel_image_crop git:(master) ✗ python benchmarks/test.py --batch-size=32 --num-trials=100 --use-vips=1 --use-threading=0
python crop average over 100 trials : 0.4660675573348999 +/- 0.057874179663163314 sec
rust crop average over 100 trials : 0.5588112926483154 +/- 0.02093116503601568 sec
(base) ➜  parallel_image_crop git:(master) ✗ python benchmarks/test.py --batch-size=32 --num-trials=100 --use-vips=1 --use-threading=1
python crop average over 100 trials : 0.4722147536277771 +/- 0.0485672093840053 sec
rust crop average over 100 trials : 0.5598368859291076 +/- 0.023788753163516176 sec
```

Gray Scale 4000 x 4000 PNG Image:

```bash
(base) ➜  parallel_image_crop git:(master) ✗ python benchmarks/test.py --batch-size=32 --num-trials=100 --use-vips=0 --use-threading=0
python crop average over 100 trials : 0.48749495029449463 +/- 0.08113528509347652 sec
rust crop average over 100 trials : 0.5558233976364135 +/- 0.016653053525873318 sec
(base) ➜  parallel_image_crop git:(master) ✗ python benchmarks/test.py --batch-size=32 --num-trials=100 --use-vips=0 --use-threading=1
python crop average over 100 trials : 0.47109495639801025 +/- 0.04871502077173173 sec
rust crop average over 100 trials : 0.5580063557624817 +/- 0.03254635395283061 sec
(base) ➜  parallel_image_crop git:(master) ✗ python benchmarks/test.py --batch-size=32 --num-trials=100 --use-vips=1 --use-threading=0
python crop average over 100 trials : 0.45097975969314574 +/- 0.05091062232673361 sec
rust crop average over 100 trials : 0.5570903038978576 +/- 0.02757379330167939 sec
(base) ➜  parallel_image_crop git:(master) ✗ python benchmarks/test.py --batch-size=32 --num-trials=100 --use-vips=1 --use-threading=1
python crop average over 100 trials : 0.46305535078048704 +/- 0.05080726901263223 sec
rust crop average over 100 trials : 0.5532968330383301 +/- 0.024264257976699295 sec
```

Color 512 x 512 PNG Image:
```bash
(base) ➜  parallel_image_crop git:(master) ✗ python benchmarks/test.py --batch-size=32 --num-trials=100 --use-vips=0 --use-threading=0
python crop average over 100 trials : 0.1581390118598938 +/- 0.07718986836579676 sec
rust crop average over 100 trials : 0.049066624641418456 +/- 0.0063537388027109275 sec
(base) ➜  parallel_image_crop git:(master) ✗ python benchmarks/test.py --batch-size=32 --num-trials=100 --use-vips=0 --use-threading=1
python crop average over 100 trials : 0.16133457899093628 +/- 0.08104974713972524 sec
rust crop average over 100 trials : 0.04776606798171997 +/- 0.002771486956986403 sec
(base) ➜  parallel_image_crop git:(master) ✗ python benchmarks/test.py --batch-size=32 --num-trials=100 --use-vips=1 --use-threading=0
python crop average over 100 trials : 0.1862180781364441 +/- 0.08092567829068359 sec
rust crop average over 100 trials : 0.048760356903076174 +/- 0.004486362361121834 sec
(base) ➜  parallel_image_crop git:(master) ✗ python benchmarks/test.py --batch-size=32 --num-trials=100 --use-vips=1 --use-threading=1
python crop average over 100 trials : 0.15620680570602416 +/- 0.07046950472514979 sec
rust crop average over 100 trials : 0.0478854775428772 +/- 0.0029215781261751495 sec
```

Color 512 x 512 JPEG Image:
```bash
(base) ➜  parallel_image_crop git:(master) ✗ python benchmarks/test.py --batch-size=32 --num-trials=100 --use-vips=0 --use-threading=0
python crop average over 100 trials : 0.2412877368927002 +/- 0.08093420929641125 sec
rust crop average over 100 trials : 0.05020998954772949 +/- 0.0029470025228493205 sec
(base) ➜  parallel_image_crop git:(master) ✗ python benchmarks/test.py --batch-size=32 --num-trials=100 --use-vips=0 --use-threading=1
python crop average over 100 trials : 0.24630839586257935 +/- 0.08689070644503785 sec
rust crop average over 100 trials : 0.053718960285186766 +/- 0.005535220006405835 sec
(base) ➜  parallel_image_crop git:(master) ✗ python benchmarks/test.py --batch-size=32 --num-trials=100 --use-vips=1 --use-threading=0
python crop average over 100 trials : 0.25572638273239134 +/- 0.07592489302786763 sec
rust crop average over 100 trials : 0.056830999851226804 +/- 0.0025679050981689214 sec
(base) ➜  parallel_image_crop git:(master) ✗ python benchmarks/test.py --batch-size=32 --num-trials=100 --use-vips=1 --use-threading=1
python crop average over 100 trials : 0.275831139087677 +/- 0.06285512636507674 sec
rust crop average over 100 trials : 0.05264364004135132 +/- 0.004437379466866233 sec
```

## Takeaway

If you have small images the Rust library seems faster.
If you have large images go with `PIL-SIMD` or `vips`
