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

const Opcode = enum(u8) {
    continuation = 0x0,
    text = 0x1,
    binary = 0x2,
    close = 0x8,
    ping = 0x9,
    pong = 0xA,
};

fn maskKey() [4]u8 {
    var m: [4]u8 = undefined;
    std.crypto.random.bytes(&m);
    return m;
}

fn websocket(allocator: std.mem.Allocator, handler: fn ([]const u8) void, path: []const u8) !void {
    const host = "127.0.0.1";
    const port = 9001;

    // CONNECT SOCKET

    const address = try std.net.Address.parseIp(host, port);
    std.debug.print("Connecting WebSocket client\n", .{});
    const stream = try std.net.tcpConnectToAddress(address);
    defer stream.close();

    const reader = stream.reader();
    const writer = stream.writer();

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
    std.debug.print("Sending upgrade request\n", .{});
    try writer.writeAll(upgrade_request);

    // VERIFY WE GOT A SUCCESSFUL UPGRADE RESPONSE

    var response_buffer = std.ArrayList(u8).init(allocator);
    defer response_buffer.deinit();

    const delimiter = "\r\n\r\n"; // (end of HTTP headers)

    while (true) { // read until end of HTTP headers
        const byte = try reader.readByte();
        try response_buffer.append(byte);

        // check last four bytes for delimiter
        if (response_buffer.items.len >= delimiter.len) {
            const tail = response_buffer.items[(response_buffer.items.len - delimiter.len)..];
            if (std.mem.eql(u8, tail, delimiter)) break;
        }
    }

    try std.testing.expect(std.mem.startsWith(u8, response_buffer.items, "HTTP/1.1 101"));

    // START PROCESSING FRAMES

    outer: while (true) { // one frame at a time, refer to frame structure at the top of this file
        std.debug.print("Reading frame\n", .{});

        const frame_header = try reader.readBytesNoEof(2);

        // first byte: check fin and opcode(ignoring rsvx)
        const fin = frame_header[0] & 0b10000000 != 0;
        const rsv = frame_header[0] & 0b01110000;
        const opcode: Opcode = switch (frame_header[0] & 0b00001111) { // FIXME: nicer way to handle this?
            0x0, 0x1, 0x2, 0x8, 0x9, 0xA => |o| @enumFromInt(o), // otherwise @enumFromInt for invalid opcode is UB
            else => |o| {
                std.debug.print("Unknown opcode: {x}, closing\n", .{o});
                break :outer;
            },
        };

        if (rsv != 0) {
            std.debug.print("Reserved bits usage not supported, closing\n", .{});
            break :outer;
        }

        // second byte: mask bit and payload size
        const is_masked = (frame_header[1] & 0b10000000) != 0;
        try std.testing.expect(!is_masked); // server messages not masked
        const len_byte: u8 = frame_header[1] & 0b01111111;

        switch (opcode) {
            .close, .ping, .pong => { // control frame checks
                if (!fin) {
                    std.debug.print("Control frames cannot be fragmented, closing\n", .{});
                    break :outer;
                }
                if (len_byte > 125) {
                    std.debug.print("Control frame payloads cannot exceed 125 bytes, closing\n", .{});
                    break :outer;
                }
            },
            else => {},
        }

        const payload_len = switch (len_byte) { // keep in mind network byte ordering (big endian)
            0...125 => len_byte,
            126 => blk: {
                const len_bytes = try reader.readBytesNoEof(2);
                break :blk std.mem.bigToNative(
                    u16,
                    std.mem.bytesToValue(u16, &len_bytes),
                );
            },
            127 => blk: {
                const len_bytes = try reader.readBytesNoEof(8);
                break :blk std.mem.bigToNative(
                    u64,
                    std.mem.bytesToValue(u64, &len_bytes),
                );
            },
            else => unreachable,
        };

        // remaining bytes are the payload (messages from server are unmasked, so the key is omitted)
        const payload = try allocator.alloc(u8, payload_len);
        defer allocator.free(payload);

        try reader.readNoEof(payload);

        switch (opcode) {
            .text, .binary, .ping => |op| {
                std.debug.print(
                    "Received {s} frame with a {d}-byte payload\n",
                    .{
                        switch (op) {
                            .text => "text",
                            .binary => "binary",
                            .ping => "ping",
                            else => unreachable,
                        },
                        payload.len,
                    },
                );

                if (op != Opcode.ping) {
                    handler(payload); // user handler for text and binary payloads
                }

                const mask_key = maskKey();
                var header: [14]u8 = undefined; // max header size: 2 (min) + 8 (extended payload len) + 4 (mask key)
                var header_len: usize = 2;

                const response_opcode: Opcode = switch (op) {
                    .text => .text, // echo text
                    .binary => .binary, // echo binary
                    .ping => .pong, // respond with pong, echo payload
                    else => unreachable,
                };

                header[0] = 0x80 | @intFromEnum(response_opcode);

                switch (payload.len) {
                    0...125 => { // fits in 7-bit length
                        header[1] = 0x80 | @as(u8, @intCast(payload.len));
                    },
                    126...std.math.maxInt(u16) => { // use 16-bit extended length
                        header[1] = 0x80 | 126;
                        std.mem.writeInt(
                            u16,
                            header[2..4],
                            @as(u16, @intCast(payload.len)),
                            .big,
                        );
                        header_len += 2;
                    },
                    (std.math.maxInt(u16) + 1)...std.math.maxInt(u64) => { // use 64-bit extended length
                        header[1] = 0x80 | 127;
                        std.mem.writeInt(
                            u64,
                            header[2..10],
                            @as(u64, payload.len),
                            .big,
                        );
                        header_len += 8;
                    },
                }

                std.mem.copyForwards(u8, header[header_len..][0..4], &mask_key);
                header_len += 4;

                // send header
                try writer.writeAll(header[0..header_len]);

                var pld = try allocator.alloc(u8, payload_len);
                defer allocator.free(pld);

                for (payload, 0..) |byte, i| {
                    pld[i] = byte ^ mask_key[i % 4];
                }

                // wstest expects echo
                std.debug.print("Responding with {s}, echoing payload\n", .{
                    switch (op) {
                        .text => "text",
                        .binary => "binary",
                        .ping => "pong",
                        else => unreachable,
                    },
                });
                try writer.writeAll(pld);
            },
            .close => {
                std.debug.print("Received close frame\n", .{});
                const close_frame = [_]u8{
                    // byte 0: fin bit == true (one frame) | (rsv not used) | opcode == 8 (close)
                    0b10000000 | @intFromEnum(Opcode.close),
                    // byte 1: mask bit == true (payload is masked) | message length
                    0b10000000 | @as(u8, 0),
                } ++ maskKey(); // bytes 2-5: mask key

                // send frame
                std.debug.print("Responding with close frame\n", .{});
                try writer.writeAll(&close_frame);

                break :outer; // disconnect
            },
            .pong => std.debug.print("Received pong\n", .{}),
            else => unreachable, // TODO: remove me
        }
    }
    std.debug.print("Closing connection\n", .{});
}

var case_count: usize = undefined;

fn setCaseCount(payload: []const u8) void {
    std.debug.print("Case count: {s}\n", .{payload});
    case_count = std.fmt.parseInt(
        u16,
        payload,
        10,
    ) catch @panic("Failed to parse case count");
}

fn nop(_: []const u8) void {}

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

    std.time.sleep(2 * 1_000_000_000); // wait for the server to start

    // CHECK TEST CASE COUNT, RUN ALL TESTS, AND GENERATE REPORT

    std.debug.print("\nGETTING CASE COUNT\n\n", .{});
    try websocket(allocator, setCaseCount, "/getCaseCount");

    defer {
        std.debug.print("\nGENERATING RESULTS REPORT\n\n", .{});
        websocket(
            allocator,
            nop,
            "/updateReports?agent=Adventus",
        ) catch @panic("Failed to generate wstest report\n");
    }

    for (1..case_count + 1) |case| {
        std.debug.print("\nCASE {d}\n\n", .{case});

        const path: []const u8 = try std.fmt.allocPrint(
            allocator,
            "/runCase?case={d}&agent=Adventus",
            .{case},
        );
        defer allocator.free(path);

        try websocket(allocator, nop, path);
    }

    // TODO: CHECK RESULTS BY COMPARING TO EXPECTED INDEX.JSON
}
