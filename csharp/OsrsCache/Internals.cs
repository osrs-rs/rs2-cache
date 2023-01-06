using System;
using System.Runtime.InteropServices;

namespace OsrsCache
{
    public static class Internals
    {
        [DllImport("osrscache.dll")]
        public static extern IntPtr cache_open(string lpText);

        [DllImport("osrscache.dll")]
        public static extern IntPtr cache_read(IntPtr cache, ushort archive, ushort group, ushort file, UIntPtr xtea_keys, ref int out_len);
    }
}
