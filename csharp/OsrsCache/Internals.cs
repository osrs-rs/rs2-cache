using System;
using System.Runtime.InteropServices;

namespace OsrsCache
{
    public class Internals : IInternals
    {
        [DllImport("osrscache.dll")]
        private static extern IntPtr cache_open(string lpText);

        [DllImport("osrscache.dll")]
        private static extern IntPtr cache_read(IntPtr cache, ushort archive, ushort group, ushort file, UIntPtr xtea_keys, ref int out_len);

        public IntPtr CacheOpen(string lpText)
        {
            return cache_open(lpText);
        }

        public IntPtr CacheRead(IntPtr cache, ushort archive, ushort group, ushort file, UIntPtr xtea_keys, ref int out_len)
        {
            return cache_read(cache, archive, group, file, xtea_keys, ref out_len);
        }
    }
}
