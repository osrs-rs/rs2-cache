using FluentAssertions;
using Microsoft.Extensions.DependencyInjection;
using Xunit;

namespace OsrsCache.Test
{
    public class ServiceCollectionExtensionsTests
    {
        [Fact]
        public void Can_add_services_to_provider()
        {
            var serviceCollection = new ServiceCollection();
            serviceCollection.AddOsrsCacheService("test");
            serviceCollection.Count.Should().Be(2);
            serviceCollection.Should().Contain(t => t.ServiceType == typeof(IInternals));
            serviceCollection.Should().Contain(t => t.ServiceType == typeof(OsrsCacheService));
        }
    }
}