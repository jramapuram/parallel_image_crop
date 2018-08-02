import os
import time
import argparse
import multiprocessing
import numpy as np
from cffi import FFI
from multiprocessing import Pool
from PIL import Image

parser = argparse.ArgumentParser(description='Rust vs. Python Image Cropping Bench')
parser.add_argument('--batch-size', type=int, default=10,
                    help="batch-size (default: 10)")
parser.add_argument('--num-trials', type=int, default=10,
                    help="number of trials to average over (default: 10)")
args = parser.parse_args()


def find(name, path):
    for root, dirs, files in os.walk(path):
        if name in files:
            return os.path.join(root, name)


def generate_scale_x_y(batch_size):
    scale = np.random.rand(batch_size).astype(np.float32)
    x = np.random.rand(batch_size).astype(np.float32)
    y = np.random.rand(batch_size).astype(np.float32)
    return scale, x, y

def rust_crop_bench(ffi, lib, path_list, scale, x, y, window_size, max_img_percentage):
    path_keepalive = [ffi.new("char[]", p) for p in path_list]
    batch_size = len(path_list)
    crops = np.zeros(window_size*batch_size)
    crops = lib.parallel_crop_and_resize(ffi.new("char* []", path_keepalive),
                                         ffi.cast("float*", ffi.cast("float*", np.ascontiguousarray(scale).ctypes.data)), # scale
                                         ffi.cast("float*", ffi.cast("float*", np.ascontiguousarray(x).ctypes.data)),     # x
                                         ffi.cast("float*", ffi.cast("float*", np.ascontiguousarray(y).ctypes.data)),     # y
                                         # ffi.cast("float*", ffi.cast("float*", np.ascontiguousarray(crops).ctypes.data)), # resultant crops
                                         window_size,
                                         max_img_percentage,
                                         batch_size)
    # print(crops)
    return crops


class CropLambda(object):
    """Returns a lambda that crops to a region.

    Args:
        window_size: the resized return image [not related to img_percentage].
        max_img_percentage: the maximum percentage of the image to use for the crop.
    """

    def __init__(self, path, window_size, max_img_percentage=0.15):
        self.path = path
        self.window_size = window_size
        self.max_img_percent = max_img_percentage

    def scale(self, val, newmin, newmax):
        return (((val) * (newmax - newmin)) / (1.0)) + newmin

    def __call__(self, crop):
        ''' converts [crop_center, x, y] to a 4-tuple
            defining the left, upper, right, and lower
            pixel coordinate and return a lambda '''
        with open(self.path, 'rb') as f:
            with Image.open(f) as img:
                img_size = np.array(img.size) # numpy-ize the img size (tuple)

                # scale the (x, y) co-ordinates to the size of the image
                assert crop[1] >= 0 and crop[1] <= 1, "x needs to be \in [0, 1]"
                assert crop[2] >= 0 and crop[2] <= 1, "y needs to be \in [0, 1]"
                x, y = [int(self.scale(crop[1], 0, img_size[0])),
                        int(self.scale(crop[2], 0, img_size[1]))]

                # calculate the scale of the true crop using the provided scale
                # Note: this is different from the return size, i.e. window_size
                crop_scale = min(crop[0], self.max_img_percent)
                crop_size = np.floor(img_size * crop_scale).astype(int) - 1

                # bound the (x, t) co-ordinates to be plausible
                # i.e < img_size - crop_size
                max_coords = img_size - crop_size
                x, y = min(x, max_coords[0]), min(y, max_coords[1])

                # crop the actual image and then upsample it to window_size
                # resample = 2 is a BILINEAR transform, avoid importing PIL for enum
                # TODO: maybe also try 1 = ANTIALIAS = LANCZOS
                crop_img = img.crop((x, y, x + crop_size[0], y + crop_size[1]))
                return crop_img.resize((self.window_size, self.window_size), resample=2)

class CropLambdaPool(object):
    def __init__(self, num_workers=8):
        self.num_workers = num_workers

    def _apply(self, lbda, z_i):
        return lbda(z_i)

    def __call__(self, list_of_lambdas, z_vec):
        with Pool(self.num_workers) as pool:
            return pool.starmap(self._apply, zip(list_of_lambdas, z_vec))

def python_crop_bench(paths, scale, x, y, window_size, max_img_percentage):
    crop_lbdas = [CropLambda(p, window_size, max_img_percentage) for p in paths]
    z = np.hstack([np.expand_dims(scale, 1), np.expand_dims(x, 1), np.expand_dims(y, 1)])
    return CropLambdaPool(num_workers=multiprocessing.cpu_count())(crop_lbdas, z)

def create_and_set_ffi():
    ffi = FFI()
    ffi.cdef("""
    typedef struct  {
        void* data;
        size_t len;
    } array_t;

    array_t parallel_crop_and_resize(char**, float*, float*, float*, uint32_t, float, size_t);

    """);

    lib = ffi.dlopen(find("libparallel_image_crop.so", ".."))
    return lib, ffi


if __name__ == "__main__":
    lena_gray = find("lena_gray.png", "..")
    lena_color = find("lena.png", "..")
    path_list = [lena_gray for _ in range(args.batch_size // 2)]+ \
         [lena_color for _ in range(args.batch_size // 2)]
    for i in range(len(path_list)):  # convert to ascii for ffi
        path_list[i] = path_list[i].encode('ascii')

    scale, x, y = generate_scale_x_y(len(path_list))

    # bench python
    python_time = []
    for i in range(args.num_trials):
        start_time = time.time()
        python_crop_bench(path_list, scale, x, y, 32, 0.25)
        python_time.append(time.time() - start_time)

    print("python crop average over {} trials : {} sec".format(args.num_trials, np.mean(python_time)))

    # bench rust lib
    rust_time = []
    lib, ffi = create_and_set_ffi()
    for i in range(args.num_trials):
        start_time = time.time()
        rust_crop_bench(ffi, lib, path_list, scale, x, y, 32, 0.25)
        rust_time.append(time.time() - start_time)

    print("rust crop average over {} trials : {} sec".format(args.num_trials, np.mean(rust_time)))
