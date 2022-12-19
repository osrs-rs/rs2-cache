from ctypes import *


class Cache:
    # Initialize the Cache object with the loaded library and the cache
    def __init__(self, lib, cache_c):
        # TODO: Init all the stuff here with loading library and setting relevant types and args instead of in "open"
        self.lib = lib
        self.cache_c = cache_c

    # Open a cache at a given path
    @staticmethod
    def open(path):
        # Load the osrscache library into C types
        lib = cdll.LoadLibrary('../rust/target/debug/osrscache.dll')

        # Open the Cache using C types
        lib.cache_open.restype = c_void_p
        cache_c = lib.cache_open(path.encode('utf-8'))

        # Return a Cache object, containing the loaded library and the cache
        return Cache(lib, cache_c)

    # Read a file from the cache
    def read(self, archive, group, file):
        output_len = c_uint32(0)
        self.lib.cache_read.argtypes = [
            c_void_p, c_int, c_int, c_int, c_int, POINTER(c_uint32)]
        self.lib.cache_read(self.cache_c, archive, group,
                            file, 0, byref(output_len))
        print(output_len)


cache = Cache.open("./cache")
# Read a blue partyhat from the cache
cache.read(2, 10, 1042)


# Example of python ctypes here:
# https://stackoverflow.com/questions/26363641/passing-a-pointer-value-to-a-c-function-from-python
