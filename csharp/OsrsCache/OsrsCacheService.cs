using System;
using System.IO;
using System.Runtime.InteropServices;

namespace OsrsCache
{
    public class OsrsCacheService : IDisposable
    {
        // The internal cache object
        private IntPtr _cache;
        private readonly IInternals _internals;

        public OsrsCacheService(IInternals internals, string path)
        {
            _cache = internals.CacheOpen(path);
            _internals = internals;
        }

        public byte[] Read(ushort archive, ushort group, ushort file, int[] xtea_keys_param = null)
        {
            // Call cache_read
            var out_len = 0;
            var buf = _internals.CacheRead(_cache, archive, group, file, xtea_keys: UIntPtr.Zero, ref out_len);

            // Copy the data from the IntPtr to the byte array
            byte[] managedArray = new byte[out_len];
            Marshal.Copy(buf, managedArray, 0, out_len);
            return managedArray;
        }

        public unsafe Stream ReadStream(ushort archive, ushort group, ushort file, int[] xtea_keys_param = null)
        {
            // Call cache_read
            var out_len = 0;
            var buf = _internals.CacheRead(_cache, archive, group, file, xtea_keys: UIntPtr.Zero, ref out_len);
            return new UnmanagedMemoryStream((byte*)buf.ToPointer(), out_len);
        }

        public void Dispose()
        {
            _cache = IntPtr.Zero;
        }
    }
}
