using System;
using System.Runtime.InteropServices;

namespace OsrsCache
{
    public class OsrsCacheService
    {
        [DllImport("osrscache.dll")]
        private static extern IntPtr cache_open(string lpText);

        [DllImport("osrscache.dll")]
        private static extern IntPtr cache_read(IntPtr cache, ushort archive, ushort group, ushort file, UIntPtr xtea_keys, ref int out_len);

        // The internal cache object
        public readonly IntPtr _cache;

        public OsrsCacheService(string path)
        {
            _cache = cache_open(path);
        }

        public byte[] Read(ushort archive, ushort group, ushort file, int[] xtea_keys_param = null)
        {
            // Start with xtea_keys default to a nullptr
            var xtea_keys = UIntPtr.Zero;
            if (xtea_keys_param != null)
            {
                // Create pointer to the data and pass it to cache_read
                // ...
            }

            // Call cache_read
            var out_len = 0;
            var buf = cache_read(_cache, archive, group, file, xtea_keys, ref out_len);

            // Copy the data from the IntPtr to the byte array
            byte[] managedArray = new byte[out_len];
            Marshal.Copy(buf, managedArray, 0, out_len);

            return managedArray;
        }
    }
}
