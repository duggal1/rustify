
    module.exports = {
      reactStrictMode: true,
      webpack: (config) => {
        config.optimization = {
          minimize: true,
          splitChunks: {
            chunks: 'all',
          },
        };
        return config;
      },
    };
    