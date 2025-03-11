const std = @import("std");

// Refer to the spec at https://datatracker.ietf.org/doc/html/rfc6455
//
//                          Frame Structure
//                          ===============
//  0                   1                   2                   3
//  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
//  +-+-+-+-+-------+-+-------------+-------------------------------+
//  |F|R|R|R| opcode|M| Payload len |    Extended payload length    |
//  |I|S|S|S|  (4)  |A|     (7)     |             (16/64)           |
//  |N|V|V|V|       |S|             |   (if payload len==126/127)   |
//  | |1|2|3|       |K|             |                               |
//  +-+-+-+-+-------+-+-------------+ - - - - - - - - - - - - - - - +
//  |     Extended payload length continued, if payload len == 127  |
//  + - - - - - - - - - - - - - - - +-------------------------------+
//  |                               |Masking-key, if MASK set to 1  |
//  +-------------------------------+-------------------------------+
//  | Masking-key (continued)       |          Payload Data         |
//  +-------------------------------- - - - - - - - - - - - - - - - +
//  :                     Payload Data continued ...                :
//  + - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - +
//  |                     Payload Data continued ...                |
//  +---------------------------------------------------------------+

fn maskKey() [4]u8 {
    var m: [4]u8 = undefined;
    std.crypto.random.bytes(&m);
    return m;
}

fn websocket(allocator: std.mem.Allocator, buffer: []u8, path: []const u8) !?[]u8 {
    const host = "127.0.0.1";
    const port = 9001;

    // CONNECT SOCKET

    const address = try std.net.Address.parseIp(host, port);
    const socket = try std.posix.socket(address.any.family, std.posix.SOCK.STREAM, std.posix.IPPROTO.TCP);
    defer std.posix.close(socket);
    try std.posix.connect(socket, &address.any, address.getOsSockLen());

    // UPGRADE TO WEBSOCKET CONNECTION

    // generate random WebSocket key
    var ws_key: [24]u8 = undefined;
    var random_bytes: [16]u8 = undefined;
    std.crypto.random.bytes(&random_bytes);
    _ = std.base64.standard.Encoder.encode(&ws_key, &random_bytes);

    const upgrade_request = try std.fmt.allocPrint(allocator, "GET {s} HTTP/1.1\r\n" ++
        "Host: {s}:{d}\r\n" ++
        "Upgrade: websocket\r\n" ++
        "Connection: Upgrade\r\n" ++
        "Sec-WebSocket-Key: {s}\r\n" ++
        "Sec-WebSocket-Version: 13\r\n" ++
        "\r\n", .{ path, host, port, ws_key });
    defer allocator.free(upgrade_request);

    // send the upgrade request
    _ = try std.posix.write(socket, upgrade_request);

    // verify we got a successful upgrade response
    var bytes_read = try std.posix.read(socket, buffer);
    try std.testing.expect(std.mem.startsWith(u8, buffer, "HTTP/1.1 101"));

    // START PROCESSING FRAMES

    var write_buffer: [1024]u8 = undefined;

    // find end of HTTP response
    var pos = std.mem.indexOf(u8, buffer, "\r\n\r\n").? + 4;

    // FIXME: use some kind of callback handler instead?
    // need to be able to store messages when connecting to /getCaseCount
    var case_count: ?[]u8 = null;

    while (true) { // one frame at a time
        if (pos == bytes_read) { // read more if buffer is fully processed
            bytes_read = try std.posix.read(socket, buffer);
            if (bytes_read == 0) break;
            pos = 0;
        }

        if (pos + 2 > bytes_read) continue;

        // refer to frame structure at the top of this file
        const frame_header = buffer[pos .. pos + 2];
        pos += 2;

        // first byte: check fin and opcode(ignoring rsvx)
        const fin = frame_header[0] & 0b10000000 != 0;
        try std.testing.expect(fin); // haven't got fragmentation yet
        const opcode = frame_header[0] & 0b00001111;
        // second byte: mask bit and payload size
        try std.testing.expect(frame_header[1] & 0b10000000 == 0); // server messages not masked

        const short_payload_len: u8 = frame_header[1] & 0b01111111;
        // FIXME: handle not having enough bytes
        const payload_len = switch (short_payload_len) { // keep in mind network byte ordering (big endian)
            0...125 => short_payload_len,
            126 => blk: {
                const payload_len = std.mem.bigToNative(
                    u16,
                    std.mem.bytesToValue(u16, buffer[pos .. pos + 2]),
                );
                pos += 2;
                break :blk payload_len;
            },
            127 => blk: {
                const payload_len = std.mem.bigToNative(
                    u64,
                    std.mem.bytesToValue(u64, buffer[pos .. pos + 8]),
                );
                pos += 8;
                break :blk payload_len;
            },
            else => unreachable,
        };

        // remaining bytes are the payload (messages from server are unmasked, so the key is omitted)
        const payload = buffer[pos .. pos + payload_len];
        pos += payload_len;

        if (case_count == null) {
            case_count = payload;
        }

        switch (opcode) {
            0x1 => {
                // wstest expects echo
                std.debug.print("Received text: {s}\n", .{payload});

                const mask_key = maskKey();

                const short_len: u8 = switch (payload.len) {
                    0...125 => @intCast(payload.len), // direct payload length in 7 bits
                    126...std.math.maxInt(u16) => 126, // use 16-bit extended length
                    (std.math.maxInt(u16) + 1)...std.math.maxInt(u64) => 127, // use 64-bit extended length
                };

                write_buffer[0] = 0x81; // fin == 1, opcode == 1 (text)
                write_buffer[1] = 0x80 | @as(u8, short_len);

                var write_pos: usize = 2;

                // append extended payload length if necessary
                if (short_len == 126) {
                    std.mem.writeInt(
                        u16,
                        write_buffer[write_pos..][0..2],
                        @as(u16, @intCast(payload.len)),
                        .big,
                    );
                    write_pos += 2;
                } else if (short_len == 127) {
                    std.mem.writeInt(
                        u64,
                        write_buffer[write_pos..][0..8],
                        @as(u64, @intCast(payload.len)),
                        .big,
                    );
                    write_pos += 8;
                }

                std.mem.copyForwards(u8, write_buffer[write_pos .. write_pos + 4], &mask_key);
                write_pos += 4;

                for (payload, 0..) |byte, i| {
                    write_buffer[write_pos + i] = byte ^ mask_key[i % 4];
                }

                std.debug.print("Sending echo\n", .{});

                _ = try std.posix.write(socket, write_buffer[0 .. write_pos + payload.len]);
            },
            0x8 => {
                std.debug.print("Received close frame\n", .{});
                const close_frame = [_]u8{
                    // byte 0: fin bit == true (one frame) | (rsv not used) | opcode == 8 (close)
                    0b10000000 | @as(u8, 8),
                    // byte 1: mask bit == true (payload is masked) | message length
                    0b10000000 | @as(u8, 0),
                }
                // bytes 2-5: mask key
                ++ maskKey();

                // send frame
                _ = try std.posix.write(socket, &close_frame);

                break; // disconnect
            },
            0x9 => std.debug.print("Received ping\n", .{}),
            0xA => std.debug.print("Received pong\n", .{}),
            else => std.debug.print("Unknown opcode: {x}\n", .{opcode}),
        }
    }

    return case_count;
}

test "autobahn" {
    const allocator = std.testing.allocator;

    // START AUTOBAHN FUZZING SERVER

    std.debug.print("Starting Autobahn fuzzing server container\n", .{});

    {
        const argv = [_][]const u8{
            "docker",
            "run",
            "--detach",
            "--rm",
            "--volume=./autobahn-testsuite:/mount",
            "--publish=9001:9001",
            "--name=fuzzingserver",
            "crossbario/autobahn-testsuite",
            "wstest",
            "--mode=fuzzingserver",
            "--spec=/mount/fuzzingserver.json",
        };

        const result = try std.process.Child.run(.{ .allocator = allocator, .argv = &argv });
        defer allocator.free(result.stderr);
        defer allocator.free(result.stdout);

        try std.testing.expectEqual(std.process.Child.Term{ .Exited = 0 }, result.term);
    }

    defer {
        const argv = [_][]const u8{
            "docker",
            "stop",
            "fuzzingserver",
        };

        const result = std.process.Child.run(.{
            .allocator = allocator,
            .argv = &argv,
        }) catch @panic("Failed to execute docker stop");

        defer allocator.free(result.stderr);
        defer allocator.free(result.stdout);

        std.testing.expectEqual(
            std.process.Child.Term{ .Exited = 0 },
            result.term,
        ) catch @panic("Failed to stop docker container"); // try not allowed in defer block

        std.debug.print("Stopped Autobahn fuzzing server container\n", .{});
    }

    std.time.sleep(1_000_000_000); // wait for the server to start

    // CHECK TEST CASE COUNT, RUN ALL TESTS, AND GENERATE REPORT

    var buffer: [1024]u8 = undefined;

    const response = try websocket(allocator, &buffer, "/getCaseCount");

    if (response) |r| {
        std.debug.print("Case count: {s}\n", .{r});
        const case_count = try std.fmt.parseInt(u16, r, 10);

        defer _ = websocket(
            allocator,
            &buffer,
            "/updateReports?agent=Adventus",
        ) catch @panic("Failed to generate wstest report\n");

        for (1..case_count + 1) |case| {
            std.debug.print("\nCASE {d}\n\n", .{case});

            const path: []const u8 = try std.fmt.allocPrint(
                allocator,
                "/runCase?case={d}&agent=Adventus",
                .{case},
            );
            defer allocator.free(path);

            _ = try websocket(allocator, &buffer, path);
        }
    }

    // TODO: CHECK RESULTS BY COMPARING TO EXPECTED INDEX.JSON
}
