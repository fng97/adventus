const std = @import("std");

// TODO:
//
// - add logging
// - profile throughput and latency
// - handle all errors
// - add comparing wstest results to test
//
// - add references to RFC throughout comments and docs
// - validate Sec-WebSocket-Accept header
// - support send fragmentation?
// - TLS support
// - timeout handling?
// - accept hostname/URI and port as parameter
// - test autobahn test cases in parallel to speed them up?

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

        fn fromOpcode(opcode: Opcode) Type {
            return switch (opcode) {
                .text => .text,
                .binary => .binary,
                else => unreachable,
            };
        }
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

    fn fromByte(byte: u8) ?Opcode {
        return switch (byte) {
            // TODO: there must be a better way, this way we have to change the enum in two places
            0x0, 0x1, 0x2, 0x8, 0x9, 0xA => |o| @enumFromInt(o),
            else => null,
        };
    }

    fn str(self: Opcode) []const u8 {
        return switch (self) {
            .text => "text",
            .binary => "binary",
            .continuation => "continuation",
            .ping => "ping",
            .pong => "pong",
            .close => "close",
        };
    }
};

/// Close codes defined by the spec for the closing handshake.
const CloseCode = enum(u16) {
    normal_closure = 1000,
    going_away = 1001,
    protocol_error = 1002,
    unsupported_data = 1003,
    invalid_payload_data = 1007,
    policy_violation = 1008,
    message_too_big = 1009,
    missing_extension = 1010,
    internal_error = 1011,
    tls_handshake_failure = 1015,

    fn fromBytes(bytes: *const [2]u8) ?CloseCode {
        return switch (std.mem.readInt(u16, bytes, .big)) { // network byte ordering
            1000, 1001, 1002, 1003, 1007, 1008, 1009, 1010, 1011, 1015 => |code| @enumFromInt(code),
            else => null,
        };
    }

    fn toBytes(self: CloseCode) [2]u8 {
        var close_code: [2]u8 = undefined;
        std.mem.writeInt(u16, &close_code, @intFromEnum(self), .big);
        return close_code;
    }
};

/// A WebSocket Client.
///
/// This client does not validate UTF-8. That is left to the application. See
/// `std.unicode.utf8ValidateSlice`.
const Client = struct {
    stream: std.net.Stream,

    const Error = error{
        ConnectionClosedByServer,
        ReservedOpcodeUsed,
        ReservedBitsSet,
        MaskBitSet,
        ControlFrameWithFinClear,
        ControlFrameWithExtendedPayloadLen,
        BufferTooSmallForPayload,
        ContinuationBeforeInitialFragment,
        IncompleteFragmentedMessage,
        InvalidCloseCode,
        InvalidUtf8,
    };

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
        if (!std.mem.eql(u8, &status, expected_status)) return error.InvalidUpgradeResponse;

        // read one line at a time until end of header: line with just "\r\n"
        while (try reader.readUntilDelimiterOrEof(&buffer, '\n')) |line| {
            if (line[0] == '\r') return Client{ .stream = stream };
        }

        return error.InvalidUpgradeResponse;
    }

    /// Read frames until a full message has been read then yield to
    /// application. This function must be called in a loop to ensure we're
    /// responding to pings with pongs. Refer to the frame structure documented
    /// at the top of this file.
    ///
    /// This handles message fragmentation. Here are some excerpts from section
    /// 5.4 of the RFC:
    ///
    /// > A fragmented message consists of a single frame with the FIN bit
    /// > clear and an opcode other than 0, followed by zero or more frames
    /// > with the FIN bit clear and the opcode set to 0, and terminated by
    /// > a single frame with the FIN bit set and an opcode of 0.
    ///
    /// > EXAMPLE: For a text message sent as three fragments, the first
    /// > fragment would have an opcode of 0x1 and a FIN bit clear, the
    /// > second fragment would have an opcode of 0x0 and a FIN bit clear,
    /// > and the third fragment would have an opcode of 0x0 and a FIN bit
    /// > that is set.
    ///
    /// > Control frames (see Section 5.5) MAY be injected in the middle of
    /// > a fragmented message.  Control frames themselves MUST NOT be
    /// > fragmented.
    fn read(
        self: *const Client,
        buffer: []u8,
    ) !Message {
        const reader = self.stream.reader();

        // Used to store message state in the case of fragmentation. This is
        // instantiated when we receive an initial fragment and data slice is
        // extended as more fragment payloads are received.
        var fragmented_message: ?Message = null;

        // START READING, ONE FRAME AT A TIME

        while (true) {
            // This slice represents the available portion of the buffer where
            // further message fragments can be written. As we receive and store
            // fragments, we adjust this slice to start immediately after the
            // last written fragment, effectively shrinking it to track the
            // remaining free space (i.e. the tail of the buffer).
            const buffer_tail = if (fragmented_message) |msg| buffer[msg.data.len..] else buffer;

            // READ HEADER

            const frame_header = try reader.readBytesNoEof(2);

            // first byte: check fin and opcode (ignoring rsv)
            const fin = frame_header[0] & fin_mask != 0;
            const rsv = frame_header[0] & rsv_mask;
            if (rsv != 0) { // extensions not supported
                self.close(.protocol_error, "Received frame with RSV set but extension not negotiated");
                return Error.ReservedBitsSet;
            }
            const opcode: Opcode = Opcode.fromByte(frame_header[0] & opc_mask) orelse {
                self.close(.protocol_error, "Received frame with reserved opcode but extension not negotiated");
                return Error.ReservedOpcodeUsed;
            };

            // second byte: check mask bit and payload length
            const is_masked = (frame_header[1] & msk_mask) != 0;
            if (is_masked) { // server frames are never masked
                self.close(.protocol_error, "Received masked frame");
                return Error.MaskBitSet;
            }
            const len_byte: u8 = frame_header[1] & len_mask;

            switch (opcode) { // control frame checks
                .close, .ping, .pong => {
                    if (!fin) {
                        self.close(.protocol_error, "Received fragmented control frame");
                        return Error.ControlFrameWithFinClear;
                    }
                    if (len_byte > 125) {
                        self.close(.protocol_error, "Received control frame with len > 125");
                        return Error.ControlFrameWithExtendedPayloadLen;
                    }
                },
                else => {},
            }

            // IF INDICATED, READ EXTENDED PAYLOAD SIZE

            const payload_len = switch (len_byte) {
                0...125 => len_byte,
                126 => blk: { // websockets use network byte order (big endian)
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

            // READ AND HANDLE PAYLOAD

            if (payload_len > buffer_tail.len) {
                self.close(CloseCode.message_too_big, "");
                return Error.BufferTooSmallForPayload;
            }

            // remaining bytes are the payload (messages from server are unmasked, so the key is omitted)
            const payload = buffer_tail[0..payload_len];
            try reader.readNoEof(payload);

            // TODO: Fix UTF-8 validation
            // - do checks for .text, .close, and .continuation
            // - check individual fragments but only check within code points
            // - check close reason
            // if (opcode == .text and !std.unicode.utf8ValidateSlice(payload)) {
            //     self.close(CloseCode.invalid_payload_data, "Received text frame with invalid UTF8");
            //     return Error.InvalidUtf8;
            // }

            std.debug.print("Received {s} frame with a {d}-byte payload\n", .{ opcode.str(), payload.len });

            switch (opcode) {
                .text, .binary => |o| {
                    if (fragmented_message != null) {
                        self.close(
                            CloseCode.protocol_error,
                            "Received frame with FIN set before fragmented message completed",
                        );
                        return Error.IncompleteFragmentedMessage;
                    }
                    const msg = Message{ .type = .fromOpcode(o), .data = payload };
                    if (fin) return msg else fragmented_message = msg;
                },
                .continuation => { // received fragment
                    if (fragmented_message == null) {
                        self.close(CloseCode.protocol_error, "Received continuation frame without initial fragment");
                        return Error.ContinuationBeforeInitialFragment;
                    }
                    fragmented_message.?.data = buffer[0 .. fragmented_message.?.data.len + payload.len]; // extend slice
                    if (fin) return fragmented_message.?;
                },
                .close => {
                    if (payload.len > 0) {
                        if (payload.len == 1) {
                            self.close(CloseCode.protocol_error, "Received close frame with payload length of 1");
                            return Error.InvalidCloseCode;
                        }

                        if (CloseCode.fromBytes(payload[0..2])) |close_code| {
                            const close_reason = payload[2..];
                            std.debug.print(
                                "Received close frame with code {d} and reason: {s}\n",
                                .{ @intFromEnum(close_code), close_reason },
                            );
                        } else {
                            self.close(CloseCode.protocol_error, "Received close frame with invalid close code");
                            return Error.InvalidCloseCode;
                        }
                    }

                    self.close(CloseCode.normal_closure, "");
                    return Error.ConnectionClosedByServer;
                },
                .ping => self.writeFrame(Opcode.pong, payload),
                .pong => {},
            }
        }
    }

    /// Send a close frame to the server with a close code and close reason.
    /// This should be called before disconnecting the client for a "clean"
    /// disconnect.
    fn close(self: *const Client, close_code: CloseCode, comptime msg: []const u8) void {
        comptime if (msg.len > 123) @compileError("Expected close reason len to be <=123 (125 - 2 for close code)");

        std.debug.print("Closing connection with code {d}{s}{s}\n", .{
            @intFromEnum(close_code),
            if (msg.len != 0) " and reason: " else "",
            msg,
        });

        var close_payload: [125]u8 = undefined; // max control frame payload len is 125
        var close_payload_len: usize = 2;

        const close_code_bytes = close_code.toBytes();
        std.mem.copyForwards(u8, close_payload[0..2], &close_code_bytes);
        std.mem.copyForwards(u8, close_payload[2..], msg);
        close_payload_len += msg.len;

        self.writeFrame(Opcode.close, close_payload[0..close_payload_len]);
    }

    /// Write a frame to the server.
    ///
    /// NOTE: this overwrites the payload memory. The payload is overwritten
    /// with the masked payload one byte at a time.
    fn writeFrame(self: *const Client, opcode: Opcode, payload: []u8) void {
        const writer = self.stream.writer();

        const mask_key = blk: {
            var m: [4]u8 = undefined;
            std.crypto.random.bytes(&m);
            break :blk m;
        };

        // max header length is 14: 2 (min) + 8 (extended payload len) + 4 (mask key)
        var header: [14]u8 = undefined;
        var header_len: usize = 2;

        // first byte: fin (true) | rsv (none) | opcode
        header[0] = fin_mask | @intFromEnum(opcode);

        // second byte: mask (true) | payload length
        header[1] = msk_mask; // first set mask bit (client frames always masked) then or with payload length
        switch (payload.len) {
            0...125 => {
                header[1] |= @as(u8, @intCast(payload.len));
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

        // next four bytes: mask key
        std.mem.copyForwards(u8, header[header_len..], &mask_key);
        header_len += mask_key.len;

        std.debug.print("Writing {s} frame with a {d}-byte payload\n", .{
            switch (opcode) {
                .text => "text",
                .binary => "binary",
                .continuation => "continuation",
                .ping => "ping",
                .pong => "pong",
                .close => "close",
            },
            payload.len,
        });

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
        // TODO: figure out how to flush instead
        std.time.sleep(5 * std.time.ns_per_ms); // in place of flush
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

        const result = std.process.Child.run(.{ .allocator = allocator, .argv = &argv }) catch |err| {
            std.debug.print("If you see 'expected 0, found 125' it means the container wasn't properly stopped. Stop" ++
                " it with 'docker stop fuzzingserver' and try again\n", .{});
            return err;
        };
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

    std.time.sleep(2 * std.time.ns_per_s); // wait for the server to start

    // CHECK TEST CASE COUNT, RUN ALL TESTS, AND GENERATE REPORT

    const buffer = try allocator.alloc(u8, 16 * 1024 * 1024); // max wstest payload is 16M
    defer allocator.free(buffer);

    const case_count: usize = blk: {
        std.debug.print("\nGETTING CASE COUNT\n\n", .{});

        const client = try Client.Connect("/getCaseCount");
        defer client.deinit();

        while (client.read(buffer)) |message| { // only expecting one message
            try std.testing.expect(message.type == .text);
            std.debug.print("Case count: {s}\n", .{message.data});
            break :blk try std.fmt.parseInt(u16, message.data, 10);
        } else |_| {}

        @panic("Failed to retrieve case count from wstest");
    };

    defer {
        std.debug.print("\nGENERATING RESULTS REPORT\n\n", .{});
        const client = Client.Connect("/updateReports?agent=Adventus") catch @panic("Failed to connect client");
        defer client.deinit();
        while (client.read(buffer)) |_| {} else |_| {}
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

        while (client.read(buffer)) |msg| {
            std.debug.print("Responding with echo\n", .{});
            client.writeMessage(msg);
        } else |_| {} // discard errors
    }

    // TODO: CHECK RESULTS BY COMPARING TO EXPECTED INDEX.JSON
}
