using FluentAssertions;
using Newtonsoft.Json.Linq;
using NSubstitute;
using NSubstitute.ReceivedExtensions;
using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading.Tasks;
using Xunit;

namespace OsrsCache.Test
{
    public class OsrsCacheServiceTests : IDisposable
    {
        private static readonly byte[] _fakeManagedData = new byte[] { 1, 2, 3, 4, 5 };
        private IInternals _fakeInternals;
        private OsrsCacheService _service;
        private IntPtr _fakeUnmangedData;

        public OsrsCacheServiceTests()
        {
            // Initialize unmanaged memory to hold the array.
            int size = Marshal.SizeOf(_fakeManagedData[0]) * _fakeManagedData.Length;
            _fakeUnmangedData = Marshal.AllocHGlobal(size);
            Marshal.Copy(_fakeManagedData, 0, _fakeUnmangedData, _fakeManagedData.Length);


            _fakeInternals = Substitute.For<IInternals>();
            _fakeInternals.CacheOpen(Arg.Any<string>()).Returns(IntPtr.Zero);
            _fakeInternals.CacheRead(IntPtr.Zero, Arg.Any<ushort>(), Arg.Any<ushort>(), Arg.Any<ushort>(), Arg.Any<UIntPtr>(), ref Arg.Any<int>())
                .Returns(x =>
                {
                    x[5] = _fakeManagedData.Length;
                    return _fakeUnmangedData;
                });
            _service = new OsrsCacheService(_fakeInternals, "test");
        }

        [Fact]
        public void Can_read_cache()
        {
            var array = _service.Read(1, 1, 1);
            var outRef = 0;
            _fakeInternals.Received(1).CacheRead(IntPtr.Zero, 1, 1, 1, UIntPtr.Zero, ref outRef);
            array.SequenceEqual(_fakeManagedData).Should().BeTrue();
        }

        [Fact]
        public async Task Can_create_unmanaged_stream()
        {
            var stream = _service.ReadStream(1, 1, 1);
            var outRef = 0;
            _fakeInternals.Received(1).CacheRead(IntPtr.Zero, 1, 1, 1, UIntPtr.Zero, ref outRef);
            var bArray = new byte[_fakeManagedData.Length];
            var count = await stream.ReadAsync(bArray, 0, _fakeManagedData.Length);
            count.Should().Be(_fakeManagedData.Length);
            bArray.SequenceEqual(_fakeManagedData).Should().BeTrue();
        }

        public void Dispose()
        {
            Marshal.FreeHGlobal(_fakeUnmangedData);
        }
    }
}
