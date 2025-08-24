import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  images: {
    remotePatterns: [
      {
        protocol: 'https',
        hostname: 'backpack.exchange',
      },
    ],
  },
};

export default nextConfig;
