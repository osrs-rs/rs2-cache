using System;
using System.IO;
using System.Runtime.InteropServices;

namespace OsrsCache
{
    public class OsrsCacheService : IDisposable
    {
        // The internal cache object
        private IntPtr _cache;

        public OsrsCacheService(string path)
        {
            _cache = Internals.cache_open(path);
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
            var buf = Internals.cache_read(_cache, archive, group, file, xtea_keys, ref out_len);

            // Copy the data from the IntPtr to the byte array
            byte[] managedArray = new byte[out_len];
            Marshal.Copy(buf, managedArray, 0, out_len);

            return managedArray;
        }

        public unsafe Stream ReadStream(ushort archive, ushort group, ushort file, int[] xtea_keys_param = null)
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
            var buf = Internals.cache_read(_cache, archive, group, file, xtea_keys, ref out_len);
            return new UnmanagedMemoryStream((byte*)buf.ToPointer(), out_len);
        }

        public void Dispose()
        {
            _cache = IntPtr.Zero;
        }
    }
}
