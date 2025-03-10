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

        // TODO: use a cool 'blk:' return here
        // TODO: try a cool range switch statement here
        const payload_len: u8 = frame_header[1] & 0b01111111; // response payload length same as payload sent
        try std.testing.expect(payload_len <= 125); // TODO: handle larger payload sizes

        // // check if we need to read further for payload length
        // if (payload_len == 126) {
        //     try std.testing.expect(pos + 2 > bytes_read); // TODO: handle not enough bytes
        //     // TODO: read payload size
        // } else if (payload_len == 127) {
        //     try std.testing.expect(pos + 8 > bytes_read); // TODO: handle not enough bytes
        //     // TODO: read payload size
        // }

        if (pos + payload_len > bytes_read) continue;

        // remaining bytes are the payload (messages from server are unmasked, so the key is omitted)
        const payload = buffer[pos .. pos + payload_len];
        pos += payload_len;

        if (case_count == null) {
            case_count = payload;
        }

        switch (opcode) {
            0x1 => {
                std.debug.print("Received text: {s}\n", .{payload});

                // wstest expects echo
                const mask_key = maskKey();

                write_buffer[0] = 0x81; // fin == 1, opcode == 1 (text)
                // FIXME: check send size
                write_buffer[1] = 0x80 | @as(u8, @truncate(payload.len));
                write_buffer[2..6].* = mask_key;

                var j: usize = 0;
                for (payload, 6..) |byte, i| {
                    write_buffer[i] = byte ^ mask_key[j % 4];
                    j += 1;
                }

                _ = try std.posix.write(socket, write_buffer[0 .. 6 + payload.len]);
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

        _ = try websocket(allocator, &buffer, "/updateReports?agent=Adventus");
    }

    // TODO: CHECK RESULTS BY COMPARING TO EXPECTED INDEX.JSON
}
