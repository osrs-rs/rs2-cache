using Microsoft.Extensions.DependencyInjection;
using System;
using System.Collections.Generic;
using System.Text;

namespace OsrsCache
{
    public static class ServiceCollectionExtensions
    {
        /// <summary>
        /// Adds Osrs cache service to DI
        /// </summary>
        /// <param name="serviceProvider">The service provider</param>
        /// <param name="cachePath">Path to the cache</param>
        /// <returns></returns>
        public static IServiceCollection AddOsrsCacheService(this IServiceCollection serviceProvider, string cachePath)
        {
            serviceProvider.AddScoped((provider) => new OsrsCacheService(cachePath));
            return serviceProvider;
        }

    }
}
