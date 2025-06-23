# A Minecraft clone written with Bevy

## TODO

- [x] understand trigger
- [ ] inspect ui
- [ ] textures
  - [x] draw quad
    - [x] custom frag shader
    - [x] nearest interpolation
    - [x] images in array
    - [x] solve texture loading
    - [x] custom mesh with texture index
    - [ ] pbr shader
      - [ ] issue: colors are washed
      - [ ] compare with default
- [ ] different blocks
- [ ] split in different crate for compile time
- [x] physics
  - [x] gizmos
    - [x] second camera
- [x] fix delay
- [x] meshing by iterating on values
- [x] capture mouse
- [x] place block

## Links

Perf profiling: https://www.youtube.com/watch?v=q3VOsGzkM-M

Webgpu doc: https://webgpufundamentals.org/

Bevy doc: https://taintedcoders.com/

Pixel art: https://orama-interactive.itch.io/pixelorama

## Notes

Use `BEVY_ASSET_ROOT` env var to set assets directory.

## Ideas

- Use a K-d tree to store blocks (maybe even mesh)

- Add event on `loader` changing current chunk. All chunk update would only run on relevant chunks.

- Chunk mesh should be anisotropic, and would only require 3 neighbours instead of 6

- What if visual chunks and terrain chunk were dissociated and overlaping at half the chunk size? All sampling would involve 8 chunks.
