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

// bit masks for first two bytes in frame
const fin_mask = 0b10000000; // byte 1: FIN
const rsv_mask = 0b01110000; // byte 1: RSV
const opc_mask = 0b00001111; // byte 1: opcode
const msk_mask = 0b10000000; // byte 2: MASK
const len_mask = 0b01111111; // byte 2: Payload len

/// This type is used by the application to handle messages. A Message
/// encapsulates the data received in text and binary frames (or text/binary
/// re-assembled from fragmented frames).
const Message = struct {
    const Type = enum {
        text,
        binary,
    };

    type: Type,
    data: []u8,
};

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

const Client = struct {
    stream: std.net.Stream,

    /// Connect socket to server and send upgrade handshake.
    fn Connect(path: []const u8) !Client {
        const host = "127.0.0.1";
        const port = 9001;

        // CONNECT SOCKET

        const address = try std.net.Address.parseIp(host, port);
        std.debug.print("Connecting WebSocket client\n", .{});
        const stream = try std.net.tcpConnectToAddress(address);

        const reader = stream.reader();
        const writer = stream.writer();

        // UPGRADE TO WEBSOCKET CONNECTION

        var buffer: [512]u8 = undefined;

        // generate random WebSocket key
        var ws_key: [24]u8 = undefined;
        var random_bytes: [16]u8 = undefined;
        std.crypto.random.bytes(&random_bytes);
        _ = std.base64.standard.Encoder.encode(&ws_key, &random_bytes);

        const upgrade_request = try std.fmt.bufPrint(
            &buffer,
            "GET {s} HTTP/1.1\r\n" ++
                "Host: {s}:{d}\r\n" ++
                "Upgrade: websocket\r\n" ++
                "Connection: Upgrade\r\n" ++
                "Sec-WebSocket-Key: {s}\r\n" ++
                "Sec-WebSocket-Version: 13\r\n" ++
                "\r\n",
            .{ path, host, port, ws_key },
        );

        std.debug.print("Sending upgrade request\n", .{});
        try writer.writeAll(upgrade_request);

        // VERIFY WE GOT A SUCCESSFUL UPGRADE RESPONSE

        const expected_status = "HTTP/1.1 101 Switching Protocols\r\n";
        var status: [expected_status.len]u8 = undefined;
        try reader.readNoEof(&status);
        try std.testing.expect(std.mem.eql(u8, &status, expected_status));

        // read one line at a time until end of header: line with just "\r\n"
        while (try reader.readUntilDelimiterOrEof(&buffer, '\n')) |line| {
            if (line[0] == '\r') {
                return Client{
                    .stream = stream,
                };
            }
        }

        return error.InvalidUpgradeResponse;
    }

    /// Read frames until a full message has been read then yield to
    /// application. This function must be called in a loop to ensure we're
    /// responding to pings with pongs.
    fn read(
        self: *const Client,
        buffer: []u8,
    ) !?Message {
        const reader = self.stream.reader();

        outer: while (true) { // read one frame at a time
            // refer to frame structure at the top of this file

            std.debug.print("Reading frame\n", .{});

            const frame_header = try reader.readBytesNoEof(2);

            // first byte: check fin and opcode (ignoring rsv)
            const fin = frame_header[0] & fin_mask != 0;
            const rsv = frame_header[0] & rsv_mask;
            const opcode: Opcode = switch (frame_header[0] & opc_mask) {
                0x0, 0x1, 0x2, 0x8, 0x9, 0xA => |o| @enumFromInt(o),
                else => |o| {
                    std.debug.print("Unknown opcode: {x}, closing\n", .{o});
                    break :outer;
                },
            };

            if (rsv != 0) {
                std.debug.print("Reserved bits usage not supported, closing\n", .{});
                break :outer;
            }

            // second byte: check mask bit and payload length
            const is_masked = (frame_header[1] & msk_mask) != 0;
            try std.testing.expect(!is_masked); // server messages not masked
            const len_byte: u8 = frame_header[1] & len_mask;

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

            if (payload_len > buffer.len) return error.BufferTooSmallForPayload;

            // remaining bytes are the payload (messages from server are unmasked, so the key is omitted)
            const payload = buffer[0..payload_len];

            try reader.readNoEof(payload);

            switch (opcode) {
                .text, .binary => |op| {
                    std.debug.print("Received {s} frame with a {d}-byte payload\n", .{
                        switch (op) {
                            .text => "text",
                            .binary => "binary",
                            else => unreachable,
                        },
                        payload.len,
                    });

                    return Message{
                        .type = switch (op) { // convert Opcode to Message.Type
                            .text => .text,
                            .binary => .binary,
                            else => unreachable,
                        },
                        .data = payload,
                    };
                },
                .continuation => {
                    std.debug.print("Received continuation frame\n", .{});
                },
                .close => {
                    std.debug.print(
                        "Received close frame\nResponding with close frame before closing\n",
                        .{},
                    );
                    const empty: []u8 = "";
                    self.writeFrame(Opcode.close, empty);
                    break :outer; // disconnect
                },
                .ping => { // respond with pong, echoing payload
                    std.debug.print(
                        "Received ping frame with a {d}-byte payload\nResponding with pong\n",
                        .{payload.len},
                    );
                    self.writeFrame(Opcode.pong, payload);
                },
                .pong => std.debug.print("Received pong\n", .{}),
            }
        }
        std.debug.print("Closing connection\n", .{});
        return null;
    }

    /// Write a frame to the server.
    ///
    /// NOTE: this overwrites the payload memory. The payload is read and
    /// overwritten with the masked payload one byte at a time.
    fn writeFrame(self: *const Client, opcode: Opcode, payload: []u8) void {
        const writer = self.stream.writer();

        const mask_key = maskKey();

        // max header length is 14: 2 (min) + 8 (extended payload len) + 4 (mask key)
        var header: [14]u8 = undefined;
        var header_len: usize = 2;

        // first byte: fin (true) | rsv (none) | opcode
        header[0] = fin_mask | @intFromEnum(opcode);

        // second byte: mask (true) | payload length
        header[1] = msk_mask; // first set mask bit (client frames always masked) then or with payload length
        switch (payload.len) {
            0...125 => {
                header[1] |= @as(u8, @intCast(payload.len)); // fits in 7-bit length
            },
            126...std.math.maxInt(u16) => {
                header[1] |= 126; // use 16-bit extended length
                std.mem.writeInt( // write extended payload length
                    u16,
                    header[2..4],
                    @as(u16, @intCast(payload.len)),
                    .big,
                );
                header_len += 2;
            },
            (std.math.maxInt(u16) + 1)...std.math.maxInt(u64) => {
                header[1] |= 127; // use 64-bit extended length
                std.mem.writeInt( // write extended payload length
                    u64,
                    header[2..10],
                    @as(u64, payload.len),
                    .big,
                );
                header_len += 8;
            },
        }

        std.mem.copyForwards(u8, header[header_len..], &mask_key);
        header_len += mask_key.len;

        writer.writeAll(header[0..header_len]) catch @panic("Failed to write header");

        // mask the payload
        for (payload, 0..) |byte, i| {
            payload[i] = byte ^ mask_key[i % mask_key.len];
        }

        writer.writeAll(payload) catch @panic("Failed to write payload");
    }

    fn writeMessage(self: *const Client, msg: Message) void {
        self.writeFrame(switch (msg.type) {
            .text => Opcode.text,
            .binary => Opcode.binary,
        }, msg.data);
    }

    fn deinit(self: *const Client) void {
        self.stream.close();
    }
};

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

    const buffer = try allocator.alloc(u8, 16 * 1024 * 1024); // max wstest payload is 16M
    defer allocator.free(buffer);

    const case_count: usize = blk: {
        std.debug.print("\nGETTING CASE COUNT\n\n", .{});

        const client = try Client.Connect("/getCaseCount");
        defer client.deinit();

        while (try client.read(buffer)) |message| { // only expecting one message
            try std.testing.expect(message.type == .text);
            std.debug.print("Case count: {s}\n", .{message.data});
            break :blk try std.fmt.parseInt(u16, message.data, 10);
        }

        @panic("Failed to retrieve case count from wstest");
    };

    defer {
        std.debug.print("\nGENERATING RESULTS REPORT\n\n", .{});

        const client = Client.Connect(
            "/updateReports?agent=Adventus",
        ) catch @panic("Failed to connect client");
        defer client.deinit();

        while (client.read(
            buffer,
        ) catch @panic("Failed to read a message\n")) |_| {}
    }

    for (1..case_count + 1) |case| {
        std.debug.print("\nCASE {d}\n\n", .{case});

        const path: []const u8 = try std.fmt.allocPrint(
            allocator,
            "/runCase?case={d}&agent=Adventus",
            .{case},
        );
        defer allocator.free(path);

        const client = try Client.Connect(path);
        defer client.deinit();

        while (try client.read(buffer)) |message| {
            // wstest expects all messages to be echoed
            std.debug.print("Responding with echo\n", .{});
            client.writeMessage(message);
        }
    }

    // TODO: CHECK RESULTS BY COMPARING TO EXPECTED INDEX.JSON
}
