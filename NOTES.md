# Notes

## Optimisations to try

- store payload on the stack and if too big require handler to work in chunks
- disable all frame checks and measure difference

### Write to socket stream in chunks to use less memory?

```zig
// Write masked payload in chunks
var chunk: [4096]u8 = undefined;
var offset: usize = 0;
while (offset < payload.len) {
    const chunk_size = @min(chunk.len, payload.len - offset);
    for (0..chunk_size) |i| {
        chunk[i] = payload[offset + i] ^ mask_key[(offset + i) % 4];
    }
    try writer.writeAll(chunk[0..chunk_size]);
    offset += chunk_size;
}
```
