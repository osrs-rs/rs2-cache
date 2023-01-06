using System;

namespace OsrsCache
{
    public interface IInternals
    {
        IntPtr CacheOpen(string lpText);
        IntPtr CacheRead(IntPtr cache, ushort archive, ushort group, ushort file, UIntPtr xtea_keys, ref int out_len);
    }
}