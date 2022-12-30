using System.Drawing;
using System.Runtime.InteropServices;

class Program
{
    private static void Main()
    {
        Console.WriteLine("C# example");

        var cache = Cache.Open("./cache");

        var data = cache.Read(2, 10, 1042);

        Console.WriteLine("Length: " + data.Length);
        Console.WriteLine("Bytes:");
        foreach (byte b in data)
        {
            Console.Write("0x" + b.ToString("X") + " ");
        }
    }
}

class Cache
{
    [DllImport("osrscache.dll")]
    private static extern IntPtr cache_open(string lpText);

    [DllImport("osrscache.dll")]
    private static extern IntPtr cache_read(IntPtr cache, ushort archive, ushort group, ushort file, UIntPtr xtea_keys, ref uint out_len);

    // The internal cache object
    IntPtr cache;

    public static Cache Open(string path)
    {
        // Create the cache object and return it
        var cache = new Cache
        {
            cache = cache_open(path)
        };

        return cache;
    }

    public byte[] Read(ushort archive, ushort group, ushort file, int[]? xtea_keys_param = null)
    {
        // Start with xtea_keys default to a nullptr
        var xtea_keys = UIntPtr.Zero;
        if (xtea_keys_param != null)
        {
            // Create pointer to the data and pass it to cache_read
            // ...
        }

        // Call cache_read
        uint out_len = 0;
        var buf = cache_read(cache, archive, group, file, xtea_keys, ref out_len);

        // Copy the data from the IntPtr to the byte array
        byte[] managedArray = new byte[out_len];
        Marshal.Copy(buf, managedArray, 0, (int)out_len);

        return managedArray;
    }
}