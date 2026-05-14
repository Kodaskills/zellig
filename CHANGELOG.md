# Changelog

## 0.1.0 (2026-05-14)


 ### Features

  * **local:** bypass CUDA at runtime when no devices found
  ([f66be76](https://github.com/Kodaskills/zellig/commit/f66be7697dc7d39b2f9a51b2302a89c669026496))
  * one binary — ct2rs + CUDA dynamic loading
  ([9c5b974](https://github.com/Kodaskills/zellig/commit/9c5b97451306b12e0aaf8558923d62fcddf6076e))
  * zellig v0.1.0 — multi-backend CLI translation tool
  ([46c4054](https://github.com/Kodaskills/zellig/commit/46c4054e3686d4df177ea1c7461937eae6f2eaa4))

  ### Bug Fixes

  * correct nvidia/cuda image tags to 12.6.3
  ([38846a8](https://github.com/Kodaskills/zellig/commit/38846a84c0b144b6a8165b7993d13ead19ac193e))
  * resolve clippy lint errors (sort_by_key, collapsible_match)
  ([382ee20](https://github.com/Kodaskills/zellig/commit/382ee20d2bff24fa1d2394aa4deec57bfcd82495))
  * use $HOME/.cargo/bin in container steps (GHA sets HOME=/github/home)
  ([594c6ce](https://github.com/Kodaskills/zellig/commit/594c6cefe7337b26ce6f3d480095c2a355b80383))

  ### Continuous Integration

  * Bump docker/build-push-action from 6 to 7
  ([093cb08](https://github.com/Kodaskills/zellig/commit/093cb08b46c08af8b715c691a7ba88df4f28a7f2))
  * Bump docker/login-action from 3 to 4
  ([e52a1f3](https://github.com/Kodaskills/zellig/commit/e52a1f3733107459efca3572f419fa3f365ef16d))
  * Bump docker/metadata-action from 5 to 6
  ([6c5b6fe](https://github.com/Kodaskills/zellig/commit/6c5b6fee3ee9569556270381f9f363e4dfe6d65d))
  * curl not found on Linux build ([82df4ac](https://github.com/Kodaskills/zellig/commit/82df4ac52d0ac811e1ddbb7ac1f7403d4ae4f7b6))
  * drop --all-features from clippy/test/docs
  ([10d68cc](https://github.com/Kodaskills/zellig/commit/10d68cc8a30891e379b478a3f1355743e5b9a939))
  * drops zig wrapper for linux — fixes OpenMP build failure
  ([4fc56fc](https://github.com/Kodaskills/zellig/commit/4fc56fc301679db07e2ccd304e41edd39439c571))
  * local features ([af5064c](https://github.com/Kodaskills/zellig/commit/af5064c528efb4a60b58384f130360d40a2777a0))

  ### Documentation

  * add Docker installation and use examples
  ([490785a](https://github.com/Kodaskills/zellig/commit/490785acd84d25280c309d0baf2c069330ca97df))
  * change index.html footer ([366fa4e](https://github.com/Kodaskills/zellig/commit/366fa4e8d9e853b8b243f3a8f4221e1a6ea27c94))
  * change README logo src ([969713a](https://github.com/Kodaskills/zellig/commit/969713a8610bc5a0e728f9cc1c7e60ea0e78936b))
  * move web directory to docs for github pages
  ([ee7c8a4](https://github.com/Kodaskills/zellig/commit/ee7c8a44d9dd11cdee85d98d03596b569925749a))

  ### Miscellaneous Chores

  * add license and repository to Cargo.toml
  ([f1a2ed2](https://github.com/Kodaskills/zellig/commit/f1a2ed2369e9d7a798de6bdbef829ba36295fe11))
  * release 0.1.0 ([85b64f7](https://github.com/Kodaskills/zellig/commit/85b64f74a97d2d74efc151a269a43c881a6b35ff))
