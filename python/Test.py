import ctypes


class Cache:
    # Initialize the Cache object with the loaded library and the cache
    def __init__(self, lib, cache_c):
        self.lib = lib
        self.cache_c = cache_c

    # Open a cache at a given path
    @staticmethod
    def open(path):
        # Load the osrscache library into C types
        lib = ctypes.cdll.LoadLibrary('../rust/target/debug/osrscache.dll')

        # Open the Cache using C types
        cache_c = lib.cache_open(path.encode('utf-8'))

        # Return a Cache object, containing the loaded library and the cache
        return Cache(lib, cache_c)

    # Read a file from the cache
    def read(self, archive, group, file):
        len = 0
        self.lib.cache_read(self.cache_c, archive, group,
                            file, 0, len)
        print(len)


cache = Cache.open("./cache")
# Read a blue partyhat from the cache
cache.read(2, 10, 1042)
