# A Minecraft clone written with Bevy

## TODO

- [ ] terrain generation
  - [x] don't generate all at once
  - [ ] stone and dirt below
  - [ ] ui to tweak generation live
  - [ ] or use json file with hot reload
  - [ ] water?
- [x] inspect ui
- [ ] textures
  - [ ] transparency
- [x] different blocks
- [ ] split in different crate for compile time
- [x] physics
    - [ ] in water
- [ ] de-capture mouse on escape
- [x] place block
  - [ ] prevent placing block if collides

## Links

Perf profiling: https://www.youtube.com/watch?v=q3VOsGzkM-M

Webgpu doc: https://webgpufundamentals.org/

Bevy doc: https://taintedcoders.com/

Pixel art: https://www.oramainteractive.com/Pixelorama/

## Notes

Use `BEVY_ASSET_ROOT` env var to set assets directory.

## Ideas

- Use a K-d tree to store blocks (maybe even mesh)

- Add event on `loader` changing current chunk. All chunk update would only run on relevant chunks.

- Chunk mesh should be anisotropic, and would only require 3 neighbours instead of 6

- What if visual chunks and terrain chunk were dissociated and overlaping at half the chunk size? All sampling would involve 8 chunks.
