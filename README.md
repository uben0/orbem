# A Minecraft clone written with Bevy

## TODO

- [x] meshing by iterating on values
- [x] capture mouse

## Ideas

- Use a K-d tree to store blocks (maybe even mesh)

- Add event on `loader` changing current chunk. All chunk update would only run on relevant chunks.

- Chunk mesh should be anisotropic, and would only require 3 neighbours instead of 6

- What if visual chunks and terrain chunk were dissociated and overlaping at half the chunk size? All sampling would involve 8 chunks.
